//! Metrics service.

use actix_web::{HttpRequest, HttpResponse};
use failure::Fallible;
use futures::future;
use futures::prelude::*;
use prometheus::{self, Registry};

/// For types that store a static Registry reference
pub trait HasRegistry {
    /// Get the static registry reference
    fn registry(&self) -> &'static Registry;
}

/// Minimally wraps a Registry for implementing `HasRegistry`.
pub struct RegistryWrapper(pub &'static Registry);

impl HasRegistry for RegistryWrapper {
    fn registry(&self) -> &'static Registry {
        self.0
    }
}

/// Serve metrics requests (Prometheus textual format).
pub fn serve<T>(req: HttpRequest) -> Box<dyn Future<Item = HttpResponse, Error = failure::Error>>
where
    T: 'static + HasRegistry,
{
    use prometheus::Encoder;

    let registry: &Registry = match req.app_data::<T>() {
        Some(t) => t.registry(),
        None => {
            return Box::new(futures::future::err(failure::err_msg(
                "could not get registry from app_data",
            )))
        }
    };

    let resp = future::ok(registry.gather())
        .and_then(|metrics| {
            let tenc = prometheus::TextEncoder::new();
            let mut buf = vec![];
            tenc.encode(&metrics, &mut buf).and(Ok(buf))
        })
        .from_err()
        .map(|content| HttpResponse::Ok().body(content));
    Box::new(resp)
}

/// Create a custom Prometheus registry.
pub fn new_registry(prefix: Option<String>) -> Fallible<Registry> {
    Registry::new_custom(prefix.clone(), None).map_err(|e| {
        failure::err_msg(format!(
            "could not create a custom regostry with prefix {:?}: {}",
            prefix,
            e.to_string()
        ))
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing;
    use actix_web::test::TestRequest;

    #[test]
    fn serve_metrics_basic() -> Fallible<()> {
        let mut rt = testing::init_runtime()?;

        let metrics_prefix = "cincinnati";
        let registry_wrapped = RegistryWrapper(Box::leak(Box::new(new_registry(Some(
            metrics_prefix.to_string(),
        ))?)));

        testing::dummy_gauge(&registry_wrapped.0, 42.0)?;

        let http_req = TestRequest::default()
            .data(registry_wrapped)
            .to_http_request();

        let metrics_call = serve::<RegistryWrapper>(http_req);
        let resp = rt.block_on(metrics_call)?;

        assert_eq!(resp.status(), 200);
        if let actix_web::body::ResponseBody::Body(body) = resp.body() {
            if let actix_web::body::Body::Bytes(bytes) = body {
                assert!(!bytes.is_empty());
                assert!(twoway::find_bytes(
                    bytes.as_ref(),
                    format!("{}_dummy_gauge 42\n", metrics_prefix).as_bytes()
                )
                .is_some());
            } else {
                bail!("expected Body")
            }
        } else {
            bail!("expected bytes in body")
        };

        Ok(())
    }
}
