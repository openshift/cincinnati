//! Cincinnati graph service.

use actix_web::http::header::{self, HeaderValue};
use actix_web::{HttpMessage, HttpRequest, HttpResponse};
use cincinnati::plugins::internal::channel_filter::ChannelFilterPlugin;
use cincinnati::plugins::internal::metadata_fetch_quay::DEFAULT_QUAY_LABEL_FILTER;
use cincinnati::plugins::InternalPluginWrapper;
use cincinnati::{plugins, Graph, CONTENT_TYPE};
use commons::{self, GraphError};
use futures::{future, Future, Stream};
use hyper::{Body, Client, Request};
use prometheus::{Counter, Histogram};
use serde_json;
use AppState;

lazy_static! {
    static ref HTTP_GRAPH_REQS: Counter = register_counter!(
        "http_graph_requests_total",
        "Total number of HTTP /v1/graph requests."
    )
    .unwrap();
    static ref HTTP_GRAPH_BAD_REQS: Counter = register_counter!(
        "http_graph_bad_requests_total",
        "Total number of bad HTTP /v1/graph requests."
    )
    .unwrap();
    static ref HTTP_UPSTREAM_REQS: Counter = register_counter!(
        "http_upstream_requests_total",
        "Total number of HTTP upstream requests."
    )
    .unwrap();
    static ref HTTP_UPSTREAM_UNREACHABLE: Counter = register_counter!(
        "http_upstream_errors_total",
        "Total number of HTTP upstream unreachable errors."
    )
    .unwrap();
    static ref HTTP_SERVE_HIST: Histogram = register_histogram!(
        "http_graph_serve_duration_seconds",
        "HTTP graph serving latency in seconds."
    )
    .unwrap();
}

/// Serve Cincinnati graph requests.
pub(crate) fn index(
    req: HttpRequest<AppState>,
) -> Box<Future<Item = HttpResponse, Error = GraphError>> {
    HTTP_GRAPH_REQS.inc();

    // Check that the client can accept JSON media type.
    if let Err(e) = commons::ensure_content_type(req.headers(), CONTENT_TYPE) {
        HTTP_GRAPH_BAD_REQS.inc();
        return Box::new(future::err(e));
    }

    // Check for required client parameters.
    let mandatory_params = &req.state().mandatory_params;
    if let Err(e) = commons::ensure_query_params(mandatory_params, req.query_string()) {
        HTTP_GRAPH_BAD_REQS.inc();
        return Box::new(future::err(e));
    }

    let configured_plugins: Vec<Box<plugins::Plugin<plugins::PluginIO>>> = {
        // TODO(steveeJ): actually make this vec configurable
        vec![Box::new(InternalPluginWrapper(ChannelFilterPlugin {
            // TODO(steveej): make this configurable
            key_prefix: String::from(DEFAULT_QUAY_LABEL_FILTER),
            key_suffix: String::from("release.channels"),
        }))]
    };

    let plugin_params = req.query().to_owned();

    // Assemble a request for the upstream Cincinnati service.
    let ups_req = match Request::get(&req.state().upstream)
        .header(header::ACCEPT, HeaderValue::from_static(CONTENT_TYPE))
        .body(Body::empty())
    {
        Ok(req) => req,
        Err(_) => return Box::new(future::err(GraphError::FailedUpstreamRequest)),
    };

    HTTP_UPSTREAM_REQS.inc();
    let timer = HTTP_SERVE_HIST.start_timer();
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
        .and_then(move |body| {
            let graph: Graph = serde_json::from_slice(&body)
                .map_err(|e| GraphError::FailedJsonIn(e.to_string()))?;

            let graph = match cincinnati::plugins::process(
                &configured_plugins,
                cincinnati::plugins::InternalIO {
                    graph,
                    // the plugins used in the graph-builder don't expect any parameters yet
                    parameters: plugin_params,
                },
            ) {
                Ok(graph) => graph,
                Err(e) => return Err(GraphError::FailedPluginExecution(e.to_string())),
            };

            let resp = HttpResponse::Ok().content_type(CONTENT_TYPE).body(
                serde_json::to_string(&graph)
                    .map_err(|e| GraphError::FailedJsonOut(e.to_string()))?,
            );
            Ok(resp)
        })
        .then(move |r| {
            timer.observe_duration();
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

    fn common_init() -> Runtime {
        let _ = env_logger::try_init_from_env(env_logger::Env::default());
        Runtime::new().unwrap()
    }

    #[test]
    fn missing_content_type() {
        let mut rt = common_init();
        let state = AppState::default();

        let http_req = actix_web::test::TestRequest::with_state(state).finish();
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

        let http_req = actix_web::test::TestRequest::with_state(state)
            .header(
                http::header::ACCEPT,
                http::header::HeaderValue::from_static(cincinnati::CONTENT_TYPE),
            )
            .finish();
        let graph_call = graph::index(http_req);
        let resp = rt.block_on(graph_call).unwrap_err();

        assert_eq!(
            resp,
            graph::GraphError::MissingParams(vec!["id".to_string()])
        );
    }
}
