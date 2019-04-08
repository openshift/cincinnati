//! Status service.

use actix_web::{HttpRequest, HttpResponse};
use futures::future;
use futures::prelude::*;
use prometheus;

/// Expose metrics (Prometheus textual format).
pub fn serve_metrics(
    _req: HttpRequest<()>,
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
