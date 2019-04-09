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
    /// Track whether the main service is ready.
    ready: Arc<RwLock<bool>>,
}

impl StatusState {
    pub fn new(json_graph: Arc<RwLock<String>>) -> Self {
        Self {
            json_graph,
            ready: Arc::new(RwLock::new(false)),
        }
    }

    /// Check whether (non-poisoned and non-empty) graph-content can be served.
    fn has_graph_data(&self) -> bool {
        self.json_graph
            .read()
            .ok()
            .map(|json| !json.is_empty())
            .unwrap_or(false)
    }

    /// Return whether the main service is alive.
    fn is_live(&self) -> bool {
        self.json_graph.read().ok().map(|_| true).unwrap_or(false)
    }

    /// Return whether the main service is ready.
    fn is_ready(&self) -> bool {
        self.ready.read().ok().map(|val| *val).unwrap_or(false)
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
    let resp = if req.state().is_live() {
        HttpResponse::Ok().finish()
    } else {
        HttpResponse::InternalServerError().finish()
    };
    Box::new(future::ok(resp))
}

/// Expose readiness status.
///
/// Status:
///  * Ready (200 code): JSON graph is already available.
///  * Not Ready (500 code): no JSON graph available yet.
pub fn serve_readiness(
    req: HttpRequest<StatusState>,
) -> Box<Future<Item = HttpResponse, Error = failure::Error>> {
    let mut ready = req.state().is_ready();

    if !ready && req.state().has_graph_data() {
        if let Ok(mut ready_state) = req.state().ready.write() {
            *ready_state = true;
            ready = true;
        }
    }

    let resp = if ready {
        HttpResponse::Ok().finish()
    } else {
        HttpResponse::InternalServerError().finish()
    };
    Box::new(future::ok(resp))
}
