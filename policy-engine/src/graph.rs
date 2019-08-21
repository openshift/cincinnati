//! Cincinnati graph service.

use crate::AppState;
use actix_web::http::header::{self, HeaderValue};
use actix_web::{HttpRequest, HttpResponse};
use cincinnati::CONTENT_TYPE;
use commons::{self, GraphError};
use failure::Fallible;
use futures::{future, Future, Stream};
use hyper::{Body, Client, Request};
use prometheus::{Counter, Histogram, HistogramOpts, Registry};
use serde_json;

lazy_static! {
    static ref HTTP_UPSTREAM_REQS: Counter = Counter::new(
        "http_upstream_requests_total",
        "Total number of HTTP upstream requests"
    )
    .unwrap();
    static ref HTTP_UPSTREAM_UNREACHABLE: Counter = Counter::new(
        "http_upstream_errors_total",
        "Total number of HTTP upstream unreachable errors"
    )
    .unwrap();
    static ref V1_GRAPH_INCOMING_REQS: Counter = Counter::new(
        "v1_graph_incoming_requests_total",
        "Total number of incoming HTTP client request to /v1/graph"
    )
    .unwrap();
    static ref V1_GRAPH_SERVE_HIST: Histogram = Histogram::with_opts(HistogramOpts::new(
        "v1_graph_serve_duration_seconds",
        "HTTP graph serving latency in seconds"
    ))
    .unwrap();
}

/// Register relevant metrics to a prometheus registry.
pub(crate) fn register_metrics(registry: &Registry) -> Fallible<()> {
    commons::register_metrics(&registry)?;
    registry.register(Box::new(V1_GRAPH_INCOMING_REQS.clone()))?;
    registry.register(Box::new(HTTP_UPSTREAM_REQS.clone()))?;
    registry.register(Box::new(HTTP_UPSTREAM_UNREACHABLE.clone()))?;
    registry.register(Box::new(V1_GRAPH_SERVE_HIST.clone()))?;
    Ok(())
}

/// Serve Cincinnati graph requests.
pub(crate) fn index(req: HttpRequest) -> Box<Future<Item = HttpResponse, Error = GraphError>> {
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

    // TODO(steveeJ): take another look at the actix-web docs for a method that
    // provides this parameters split.
    let plugin_params = req
        .query_string()
        .to_owned()
        .split('&')
        .map(|pair| {
            let kv_split: Vec<&str> = pair.split('=').collect();

            let value = kv_split
                .get(1)
                .unwrap_or_else(|| {
                    trace!(
                        "query parameter '{}' is not a k=v pair. assuming an empty value.",
                        pair
                    );
                    &""
                })
                .to_string();

            (kv_split[0].to_string(), value)
        })
        .collect();

    let plugins = req
        .app_data::<AppState>()
        .expect(commons::MISSING_APPSTATE_PANIC_MSG)
        .plugins;

    // Assemble a request for the upstream Cincinnati service.
    let ups_req = match Request::get(
        &req.app_data::<AppState>()
            .expect(commons::MISSING_APPSTATE_PANIC_MSG)
            .upstream,
    )
    .header(header::ACCEPT, HeaderValue::from_static(CONTENT_TYPE))
    .body(Body::empty())
    {
        Ok(req) => req,
        Err(_) => return Box::new(future::err(GraphError::FailedUpstreamRequest)),
    };

    HTTP_UPSTREAM_REQS.inc();
    let timer = V1_GRAPH_SERVE_HIST.start_timer();
    let serve = Client::new()
        .request(ups_req)
        .map_err(|e| GraphError::FailedUpstreamFetch(e.to_string()))
        .and_then(|res| {
            if res.status().is_success() {
                future::ok(res)
            } else {
                HTTP_UPSTREAM_UNREACHABLE.inc();
                future::err(GraphError::FailedUpstreamFetch(res.status().to_string()))
            }
        })
        .and_then(|res| {
            res.into_body()
                .concat2()
                .map_err(|e| GraphError::FailedUpstreamFetch(e.to_string()))
        })
        .and_then(|body| {
            serde_json::from_slice(&body).map_err(|e| GraphError::FailedJsonIn(e.to_string()))
        })
        .and_then(move |graph| {
            cincinnati::plugins::process(
                plugins.iter(),
                cincinnati::plugins::PluginIO::InternalIO(cincinnati::plugins::InternalIO {
                    graph,
                    parameters: plugin_params,
                }),
            )
            .map_err(|e| GraphError::FailedPluginExecution(e.to_string()))
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
    use cincinnati::plugins::BoxedPlugin;
    use failure::Fallible;
    use mockito;
    use std::error::Error;

    fn common_init() -> Runtime {
        let _ = env_logger::try_init_from_env(env_logger::Env::default());
        Runtime::new().unwrap()
    }

    // Source policy plugins from TOML configuration file.
    fn openshift_policy_plugins() -> Fallible<Vec<BoxedPlugin>> {
        use commons::MergeOptions;

        let mut settings = crate::config::AppSettings::default();
        let opts = {
            use std::io::Write;

            let sample_config = r#"
                [[policy]]
                name = "channel-filter"
                key_prefix = "io.openshift.upgrades.graph"
                key_suffix = "release.channels"
            "#;

            let mut config_file = tempfile::NamedTempFile::new().unwrap();
            config_file
                .write_fmt(format_args!("{}", sample_config))
                .unwrap();
            crate::config::FileOptions::read_filepath(config_file.path()).unwrap()
        };

        settings.try_merge(Some(opts))?;
        let plugins = settings.policy_plugins()?;
        Ok(plugins)
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
    fn failed_plugin_execution() -> Result<(), Box<Error>> {
        use std::str::FromStr;

        let mut rt = common_init();

        let policies = openshift_policy_plugins()?;
        let mandatory_params = vec!["channel".to_string()].into_iter().collect();
        let state = AppState {
            mandatory_params,
            plugins: Box::leak(Box::new(policies)),
            upstream: hyper::Uri::from_str(&mockito::server_url())?,
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
            Err(graph::GraphError::FailedPluginExecution(ref msg))
                if msg.contains("does not match regex") =>
            {
                Ok(())
            }
            res => Err(format!("expected FailedPluginExecution error, got: {:?}", res).into()),
        }
    }

    #[test]
    fn webservice_graph_json_error_response() -> Result<(), Box<Error>> {
        let _ = common_init();

        struct TestParams<'a> {
            name: &'a str,
            mandatory_params: &'a [&'a str],
            passed_params: &'a [(&'a str, &'a str)],
            expected_error: commons::GraphError,
        }

        fn run_test(
            mandatory_params: &[&str],
            passed_params: &[(&str, &str)],
            expected_error: &commons::GraphError,
        ) -> Result<(), Box<Error>> {
            use std::str::FromStr;

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
                .with_body(r#"{"nodes":[],"edges":[]}"#)
                .create();

            // prepare and run the policy-engine test-service
            let policies = openshift_policy_plugins()?;
            let app = actix_web::App::new()
                .register_data(actix_web::web::Data::new(AppState {
                    mandatory_params: mandatory_params.iter().map(|s| s.to_string()).collect(),
                    plugins: Box::leak(Box::new(policies)),
                    upstream: hyper::Uri::from_str(&mockito::server_url())?,
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

                if response.status() != expected_error.status_code() {
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

            Ok(())
        }

        [
            TestParams {
                name: "missing channel parameter",
                mandatory_params: &["channel"],
                passed_params: &[],
                expected_error: commons::GraphError::MissingParams(vec!["channel".to_string()]),
            },
            TestParams {
                name: "invalid channel name",
                mandatory_params: &["channel"],
                passed_params: &[("channel", "invalid:channel")],
                expected_error: commons::GraphError::FailedPluginExecution(
                    "channel 'invalid:channel'".to_string(),
                ),
            },
        ]
        .iter()
        .try_for_each(|test_param| {
            run_test(
                &test_param.mandatory_params,
                &test_param.passed_params,
                &test_param.expected_error,
            )
            .map_err(|e| format!("test '{}' failed: {}", test_param.name, e).into())
        })
    }
}
