//! Status service.

use crate::graph::State;
use actix_web::{HttpRequest, HttpResponse};
use futures::future;
use futures::prelude::*;
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
    _req: HttpRequest,
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
    req: HttpRequest,
) -> Box<Future<Item = HttpResponse, Error = failure::Error>> {
    let resp = if req
        .app_data::<State>()
        .expect(commons::MISSING_APPSTATE_PANIC_MSG)
        .is_live()
    {
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
    req: HttpRequest,
) -> Box<Future<Item = HttpResponse, Error = failure::Error>> {
    let resp = if req
        .app_data::<State>()
        .expect(commons::MISSING_APPSTATE_PANIC_MSG)
        .is_ready()
    {
        HttpResponse::Ok().finish()
    } else {
        HttpResponse::InternalServerError().finish()
    };
    Box::new(future::ok(resp))
}

#[cfg(test)]
mod tests {
    use crate::graph::State;
    use crate::status;
    use actix_web::test::TestRequest;
    use commons::testing;
    use failure::Fallible;
    use parking_lot::RwLock;
    use std::collections::HashSet;
    use std::sync::Arc;

    fn mock_state() -> State {
        let json_graph = Arc::new(RwLock::new(String::new()));
        let live = Arc::new(RwLock::new(false));
        let ready = Arc::new(RwLock::new(false));

        State::new(
            json_graph.clone(),
            HashSet::new(),
            live.clone(),
            ready.clone(),
            Box::leak(Box::new([])),
        )
    }

    #[test]
    fn serve_metrics_basic() -> Fallible<()> {
        let mut rt = testing::init_runtime()?;
        testing::dummy_gauge(&status::PROM_REGISTRY, 42.0)?;

        let http_req = TestRequest::default().data(mock_state()).to_http_request();
        let metrics_call = status::serve_metrics(http_req);
        let resp = rt.block_on(metrics_call)?;

        assert_eq!(resp.status(), 200);
        if let actix_web::body::ResponseBody::Body(body) = resp.body() {
            if let actix_web::body::Body::Bytes(bytes) = body {
                assert!(!bytes.is_empty());
                println!("{:?}", std::str::from_utf8(bytes.as_ref()));
                assert!(
                    twoway::find_bytes(bytes.as_ref(), b"cincinnati_gb_dummy_gauge 42\n").is_some()
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
