//! Status service.

use actix_web::{HttpRequest, HttpResponse};
use futures::future;
use futures::prelude::*;
use prometheus;
use std::sync::{Arc, RwLock};

/// State for the status service.
#[derive(Clone)]
pub struct StatusState {
    /// Cached graph.
    json_graph: Arc<RwLock<String>>,
}

impl StatusState {
    pub fn new(json_graph: Arc<RwLock<String>>) -> Self {
        Self { json_graph }
    }
}

/// Expose metrics (Prometheus textual format).
pub fn serve_metrics(
    _req: HttpRequest<StatusState>,
) -> Box<Future<Item = HttpResponse, Error = failure::Error>> {
    use prometheus::Encoder;

    let resp = future::ok(prometheus::gather())
        .and_then(|metrics| {
            let tenc = prometheus::TextEncoder::new();
            let mut buf = vec![];
            tenc.encode(&metrics, &mut buf).and(Ok(buf))
        })
        .from_err()
        .map(|content| HttpResponse::Ok().body(content));
    Box::new(resp)
}

/// Expose liveness status.
///
/// Status:
///  * Live (200 code): JSON graph is accessible (lock not poisoned).
///  * Not Live (500 code): everything else.
pub fn serve_liveness(
    req: HttpRequest<StatusState>,
) -> Box<Future<Item = HttpResponse, Error = failure::Error>> {
    let live = req
        .state()
        .json_graph
        .read()
        .ok()
        .map(|_| true)
        .unwrap_or(false);

    let resp = if live {
        HttpResponse::Ok().finish()
    } else {
        HttpResponse::InternalServerError().finish()
    };

    Box::new(future::ok(resp))
}
