//! Status service.

use actix_web::{HttpRequest, HttpResponse};
use futures::future;
use futures::prelude::*;
use crate::graph::State;
use prometheus;

/// Common prefix for graph-builder metrics.
static GB_METRICS_PREFIX: &str = "cincinnati_gb";

lazy_static! {
    /// Metrics registry.
    pub static ref PROM_REGISTRY: prometheus::Registry =
        prometheus::Registry::new_custom(Some(GB_METRICS_PREFIX.to_string()), None)
            .expect("could not create metrics registry");
}

/// Expose metrics (Prometheus textual format).
pub fn serve_metrics(
    _req: HttpRequest<State>,
) -> Box<Future<Item = HttpResponse, Error = failure::Error>> {
    use prometheus::Encoder;

    let resp = future::ok(PROM_REGISTRY.gather())
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
///  * Live (200 code): The upstream scrape loop thread is running
///  * Not Live (500 code): everything else.
pub fn serve_liveness(
    req: HttpRequest<State>,
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
///  * Ready (200 code): a JSON graph as the result of a successful scrape is available.
///  * Not Ready (500 code): no JSON graph available yet.
pub fn serve_readiness(
    req: HttpRequest<State>,
) -> Box<Future<Item = HttpResponse, Error = failure::Error>> {
    let resp = if req.state().is_ready() {
        HttpResponse::Ok().finish()
    } else {
        HttpResponse::InternalServerError().finish()
    };
    Box::new(future::ok(resp))
}
