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
pub(crate) fn serve(_req: HttpRequest) -> Box<Future<Item = HttpResponse, Error = failure::Error>> {
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

#[cfg(test)]
mod tests {
    use crate::metrics;
    use actix_web::test::TestRequest;
    use commons::testing;
    use failure::Fallible;

    #[test]
    fn serve_metrics_basic() -> Fallible<()> {
        let mut rt = testing::init_runtime()?;
        testing::dummy_gauge(&metrics::PROM_REGISTRY, 42.0)?;

        let http_req = TestRequest::default().to_http_request();
        let metrics_call = metrics::serve(http_req);
        let resp = rt.block_on(metrics_call)?;

        assert_eq!(resp.status(), 200);
        if let actix_web::body::ResponseBody::Body(body) = resp.body() {
            if let actix_web::body::Body::Bytes(bytes) = body {
                assert!(!bytes.is_empty());
                assert!(
                    twoway::find_bytes(bytes.as_ref(), b"cincinnati_pe_dummy_gauge 42\n").is_some()
                );
            } else {
                bail!("expected Body")
            }
        } else {
            bail!("expected bytes in body")
        };

        Ok(())
    }
}
