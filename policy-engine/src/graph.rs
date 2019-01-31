//! Cincinnati graph service.

use actix_web::http::header::{self, HeaderValue};
use actix_web::{HttpMessage, HttpRequest, HttpResponse};
use cincinnati::{Graph, CONTENT_TYPE};
use commons;
use failure::Error;
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
pub(crate) fn index(req: HttpRequest<AppState>) -> Box<Future<Item = HttpResponse, Error = Error>> {
    HTTP_GRAPH_REQS.inc();

    // Check for required client parameters.
    let mandatory_params = &req.state().mandatory_params;
    if commons::ensure_query_params(mandatory_params, req.query_string()).is_err() {
        HTTP_GRAPH_BAD_REQS.inc();
        return Box::new(future::ok(HttpResponse::BadRequest().finish()));
    }

    match req.headers().get(header::ACCEPT) {
        Some(entry) if entry == HeaderValue::from_static(CONTENT_TYPE) => {
            let ups_req = Request::get(&req.state().upstream)
                .header(header::ACCEPT, HeaderValue::from_static(CONTENT_TYPE))
                .body(Body::empty())
                .expect("unable to form request");
            HTTP_UPSTREAM_REQS.inc();
            let timer = HTTP_SERVE_HIST.start_timer();
            let serve = Client::new()
                .request(ups_req)
                .from_err::<Error>()
                .map_err(|e| {
                    HTTP_UPSTREAM_UNREACHABLE.inc();
                    e
                })
                .and_then(|res| {
                    if res.status().is_success() {
                        future::ok(res)
                    } else {
                        future::err(format_err!(
                            "failed to fetch upstream graph: {}",
                            res.status()
                        ))
                    }
                })
                .and_then(|res| res.into_body().concat2().from_err::<Error>())
                .and_then(|body| {
                    let graph: Graph = serde_json::from_slice(&body)?;
                    Ok(HttpResponse::Ok()
                        .content_type(CONTENT_TYPE)
                        .body(serde_json::to_string(&graph)?))
                })
                .then(move |r| {
                    timer.observe_duration();
                    r
                });
            Box::new(serve)
        }
        _ => {
            HTTP_GRAPH_BAD_REQS.inc();
            Box::new(future::ok(HttpResponse::NotAcceptable().finish()))
        }
    }
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
        let resp = rt.block_on(graph_call).unwrap();

        assert_eq!(resp.status(), http::StatusCode::NOT_ACCEPTABLE);
    }

    #[test]
    fn missing_mandatory_params() {
        let mut rt = common_init();
        let mandatory_params = vec!["id".to_string()].into_iter().collect();
        let state = AppState {
            mandatory_params,
            ..Default::default()
        };
        let http_req = actix_web::test::TestRequest::with_state(state).finish();

        let graph_call = graph::index(http_req);
        let resp = rt.block_on(graph_call).unwrap();

        assert_eq!(resp.status(), http::StatusCode::BAD_REQUEST);
    }
}
