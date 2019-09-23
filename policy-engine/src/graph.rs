//! Cincinnati graph service.

use crate::AppState;
use actix_web::web::Query;
use actix_web::{HttpRequest, HttpResponse};
use cincinnati::CONTENT_TYPE;
use commons::{self, GraphError};
use failure::Fallible;
use futures::{future, Future};
use prometheus::{histogram_opts, Counter, Histogram, Registry};
use serde_json;
use std::collections::HashMap;

lazy_static! {
    static ref V1_GRAPH_INCOMING_REQS: Counter = Counter::new(
        "v1_graph_incoming_requests_total",
        "Total number of incoming HTTP client request to /v1/graph"
    )
    .unwrap();
    // Histogram with custom bucket values for serving latency metric (in seconds), values are picked based on monthly data
    static ref V1_GRAPH_SERVE_HIST: Histogram = Histogram::with_opts(histogram_opts!(
        "v1_graph_serve_duration_seconds",
        "HTTP graph serving latency in seconds",
        vec![0.005, 0.01, 0.025, 0.05, 0.075, 0.1, 0.25, 0.5, 0.75, 1.0, 5.0]
    ))
    .unwrap();
}

/// Register relevant metrics to a prometheus registry.
pub(crate) fn register_metrics(registry: &Registry) -> Fallible<()> {
    commons::register_metrics(&registry)?;
    registry.register(Box::new(V1_GRAPH_INCOMING_REQS.clone()))?;
    registry.register(Box::new(V1_GRAPH_SERVE_HIST.clone()))?;
    Ok(())
}

/// Serve Cincinnati graph requests.
pub(crate) fn index(req: HttpRequest) -> Box<dyn Future<Item = HttpResponse, Error = GraphError>> {
    V1_GRAPH_INCOMING_REQS.inc();

    // Check that the client can accept JSON media type.
    if let Err(e) = commons::ensure_content_type(req.headers(), CONTENT_TYPE) {
        return Box::new(future::err(e));
    }

    // Check for required client parameters.
    let mandatory_params = &req
        .app_data::<AppState>()
        .expect(commons::MISSING_APPSTATE_PANIC_MSG)
        .mandatory_params;
    if let Err(e) = commons::ensure_query_params(mandatory_params, req.query_string()) {
        return Box::new(future::err(e));
    }

    let plugin_params = match Query::<HashMap<String, String>>::from_query(req.query_string()) {
        Ok(query) => query.into_inner(),
        Err(e) => {
            return Box::new(futures::future::err(commons::GraphError::InvalidParams(
                e.to_string(),
            )))
        }
    };

    let plugins = req
        .app_data::<AppState>()
        .expect(commons::MISSING_APPSTATE_PANIC_MSG)
        .plugins;

    let timer = V1_GRAPH_SERVE_HIST.start_timer();
    let serve = futures::future::ok(())
        .and_then(move |_| {
            cincinnati::plugins::process(
                plugins.iter(),
                cincinnati::plugins::PluginIO::InternalIO(cincinnati::plugins::InternalIO {
                    graph: Default::default(),
                    parameters: plugin_params,
                }),
            )
            .map_err(|e| match e.downcast::<GraphError>() {
                Ok(graph_error) => graph_error,
                Err(other_error) => GraphError::FailedPluginExecution(other_error.to_string()),
            })
        })
        .and_then(|internal_io| {
            serde_json::to_string(&internal_io.graph)
                .map_err(|e| GraphError::FailedJsonOut(e.to_string()))
        })
        .map(|graph_json| {
            HttpResponse::Ok()
                .content_type(CONTENT_TYPE)
                .body(graph_json)
        })
        .then(move |r| {
            timer.observe_duration();

            if let Err(e) = &r {
                error!(
                    "Error serving request with parameters '{:?}': {}",
                    req.query_string(),
                    e
                );
            }

            r
        });
    Box::new(serve)
}

#[cfg(test)]
mod tests {
    extern crate tokio;

    use self::tokio::runtime::current_thread::Runtime;
    use crate::graph;
    use crate::AppState;
    use actix_web::http;
    use cincinnati::plugins::prelude::*;
    use mockito;
    use std::error::Error;

    fn common_init() -> Runtime {
        let _ = env_logger::try_init_from_env(env_logger::Env::default());
        Runtime::new().unwrap()
    }

    #[test]
    fn missing_content_type() {
        let mut rt = common_init();
        let state = AppState::default();

        let http_req = actix_web::test::TestRequest::get()
            .data(actix_web::web::Data::new(state))
            .to_http_request();
        let graph_call = graph::index(http_req);
        let resp = rt.block_on(graph_call).unwrap_err();

        assert_eq!(resp, graph::GraphError::InvalidContentType);
    }

    #[test]
    fn missing_mandatory_params() {
        let mut rt = common_init();
        let mandatory_params = vec!["id".to_string()].into_iter().collect();
        let state = AppState {
            mandatory_params,
            ..Default::default()
        };

        let http_req = actix_web::test::TestRequest::get()
            .data(state)
            .header(
                http::header::ACCEPT,
                http::header::HeaderValue::from_static(cincinnati::CONTENT_TYPE),
            )
            .to_http_request();
        let graph_call = graph::index(http_req);
        let resp = rt.block_on(graph_call).unwrap_err();

        assert_eq!(
            resp,
            graph::GraphError::MissingParams(vec!["id".to_string()])
        );
    }

    #[test]
    #[should_panic(expected = "the request has no app_data attached. this is a bug.")]
    fn index_with_missing_appstate_must_panic() {
        let mut rt = common_init();

        let http_req = actix_web::test::TestRequest::get()
            .header(
                http::header::ACCEPT,
                http::header::HeaderValue::from_static(cincinnati::CONTENT_TYPE),
            )
            .to_http_request();
        let graph_call = graph::index(http_req);
        let resp = rt.block_on(graph_call).unwrap_err();

        assert_eq!(
            resp,
            graph::GraphError::MissingParams(vec!["id".to_string()])
        );
    }

    #[test]
    fn failed_plugin_execution() -> Result<(), Box<dyn Error>> {
        let mut rt = common_init();

        let plugins = build_plugins(
            &[plugin_config!(
                ("name", "channel-filter"),
                ("key_prefix", "io.openshift.upgrades.graph"),
                ("key_suffix", "release.channels")
            )?],
            None,
        )?;

        let mandatory_params = vec!["channel".to_string()].into_iter().collect();

        let state = AppState {
            mandatory_params,
            plugins: Box::leak(Box::new(plugins)),
            ..Default::default()
        };

        let http_req = actix_web::test::TestRequest::get()
            .data(state)
            .uri(&format!("{}?channel=':'", "http://unused.test"))
            .header(
                http::header::ACCEPT,
                http::header::HeaderValue::from_static(cincinnati::CONTENT_TYPE),
            )
            .to_http_request();

        let graph_call = graph::index(http_req);

        let _m = mockito::mock("GET", "/")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{"nodes":[],"edges":[]}"#)
            .create();

        match rt.block_on(graph_call) {
            Err(graph::GraphError::InvalidParams(ref msg))
                if msg.contains("does not match regex") =>
            {
                Ok(())
            }
            res => Err(format!("expected InvalidParams error, got: {:?}", res).into()),
        }
    }

    #[test]
    fn webservice_graph_json_response() -> Result<(), Box<dyn Error>> {
        let _ = common_init();

        enum TestResult {
            Success(String),
            Error(commons::GraphError),
        }

        impl TestResult {
            fn status_code(&self) -> http::StatusCode {
                match self {
                    TestResult::Success(_) => http::StatusCode::OK,
                    TestResult::Error(error) => error.status_code(),
                }
            }
        }

        struct TestParams<'a> {
            name: &'a str,
            mandatory_params: &'a [&'a str],
            passed_params: &'a [(&'a str, &'a str)],
            plugin_config: &'a [Box<dyn PluginSettings>],
            expected_result: TestResult,
        }

        static SERVED_GRAPH_BODY: &str = r#"{"nodes":[],"edges":[]}"#;

        fn run_test(
            mandatory_params: &[&str],
            passed_params: &[(&str, &str)],
            plugin_config: &[Box<dyn PluginSettings>],
            expected_result: &TestResult,
        ) -> Result<(), Box<dyn Error>> {
            let service_uri_base = "/graph";
            let service_uri = format!(
                "{}{}",
                service_uri_base,
                if passed_params.is_empty() {
                    String::new()
                } else {
                    passed_params
                        .iter()
                        .fold(std::string::String::from("?"), |existing, current| {
                            format!("{}{}={}", existing, current.0, current.1)
                        })
                }
            );

            // run mock graph-builder
            let _m = mockito::mock("GET", "/")
                .with_status(200)
                .with_header("content-type", "application/json")
                .with_body(SERVED_GRAPH_BODY.to_string())
                .create();

            // prepare and run the policy-engine test-service
            let plugins = build_plugins(plugin_config, None)?;

            let app = actix_web::App::new()
                .register_data(actix_web::web::Data::new(AppState {
                    mandatory_params: mandatory_params.iter().map(|s| s.to_string()).collect(),
                    plugins: Box::leak(Box::new(plugins)),
                    ..Default::default()
                }))
                .service(
                    actix_web::web::resource(&service_uri_base)
                        .route(actix_web::web::get().to(graph::index)),
                );

            let mut pe_svc = actix_web::test::init_service(app);

            let body = {
                let mut response = actix_web::test::call_service(
                    &mut pe_svc,
                    actix_web::test::TestRequest::with_uri(&service_uri)
                        .header("Accept", "application/json")
                        .to_request(),
                );

                if response.status() != expected_result.status_code() {
                    return Err(format!("unexpected statuscode:{}", response.status()).into());
                };

                let body = match response.take_body() {
                    actix_web::dev::ResponseBody::Body(b) => match b {
                        actix_web::dev::Body::Bytes(bytes) => bytes,
                        unknown => {
                            return Err(format!("expected byte body, got '{:?}'", unknown).into())
                        }
                    },
                    _ => return Err("expected body response".into()),
                };

                std::str::from_utf8(&body)?.to_owned()
            };

            let mut json: serde_json::Value = serde_json::from_str(&body)?;

            let toplevel = if let Some(obj) = json.as_object_mut() {
                obj
            } else {
                return Err("not a JSON object".into());
            };

            match expected_result {
                TestResult::Success(expected_body) => {
                    assert_eq!(expected_body.to_owned(), body);
                }
                TestResult::Error(expected_error) => {
                    if let Some(kind) = toplevel.remove("kind") {
                        assert_eq!(kind, expected_error.kind())
                    } else {
                        return Err("expected 'kind' in JSON object".into());
                    }

                    if let Some(value) = toplevel.remove("value") {
                        if let Some(result_value) = value.as_str() {
                            if !result_value.contains(&expected_error.value()) {
                                return Err(format!(
                                    "value '{}' doesn't contain: \'{}\'",
                                    result_value,
                                    expected_error.value(),
                                )
                                .into());
                            }
                        } else {
                            return Err(format!("couldn't parse '{}' as string", value).into());
                        }
                    } else {
                        return Err("expected 'value' in JSON object".into());
                    }
                }
            };

            Ok(())
        }

        use cincinnati::plugins::internal::channel_filter::ChannelFilterPlugin;
        use cincinnati::plugins::internal::cincinnati_graph_fetch::CincinnatiGraphFetchPlugin;
        use std::iter::FromIterator;

        [
            TestParams {
                name: "successful upstream graph fetch",
                mandatory_params: &[],
                passed_params: &[],
                plugin_config: &[plugin_config!(
                    ("name", CincinnatiGraphFetchPlugin::PLUGIN_NAME),
                    ("upstream", &mockito::server_url())
                )?],
                expected_result: TestResult::Success(SERVED_GRAPH_BODY.to_string()),
            },
            TestParams {
                name: "offline upstream",
                mandatory_params: &[],
                passed_params: &[],
                plugin_config: &[plugin_config!(
                    ("name", CincinnatiGraphFetchPlugin::PLUGIN_NAME),
                    ("upstream", "http://offline.url.test")
                )?],
                expected_result: TestResult::Error(commons::GraphError::FailedUpstreamFetch(
                    "http://offline.url.test/: error trying to connect".to_string(),
                )),
            },
            TestParams {
                name: "missing channel parameter",
                mandatory_params: &["channel"],
                passed_params: &[],
                plugin_config: &[plugin_config!(("name", ChannelFilterPlugin::PLUGIN_NAME))?],
                expected_result: TestResult::Error(commons::GraphError::MissingParams(vec![
                    "channel".to_string(),
                ])),
            },
            TestParams {
                name: "invalid channel name",
                mandatory_params: &["channel"],
                passed_params: &[("channel", "invalid:channel")],
                plugin_config: &[plugin_config!(("name", ChannelFilterPlugin::PLUGIN_NAME))?],
                expected_result: TestResult::Error(commons::GraphError::InvalidParams(
                    "channel 'invalid:channel'".to_string(),
                )),
            },
            TestParams {
                name: "invalid channel name with equal sign",
                mandatory_params: &["channel"],
                passed_params: &[("channel", "invalid=channel")],
                plugin_config: &[plugin_config!(("name", ChannelFilterPlugin::PLUGIN_NAME))?],
                expected_result: TestResult::Error(commons::GraphError::InvalidParams(
                    "channel 'invalid=channel'".to_string(),
                )),
            },
        ]
        .iter()
        .try_for_each(|test_param| {
            run_test(
                &test_param.mandatory_params,
                &test_param.passed_params,
                &test_param.plugin_config,
                &test_param.expected_result,
            )
            .map_err(|e| format!("test '{}' failed: {}", test_param.name, e).into())
        })
    }
}
