//! Metrics service.

use actix_web::{HttpRequest, HttpResponse};
use futures::future;
use futures::prelude::*;
use prometheus;

/// Common prefix for policy-engine metrics.
static PE_METRICS_PREFIX: &str = "cincinnati_pe";

lazy_static! {
    /// Metrics registry.
    pub(crate) static ref PROM_REGISTRY: prometheus::Registry =
        prometheus::Registry::new_custom(Some(PE_METRICS_PREFIX.to_string()), None)
            .expect("could not create metrics registry");
}

/// Serve metrics requests (Prometheus textual format).
pub(crate) fn serve(
    _req: HttpRequest<()>,
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
