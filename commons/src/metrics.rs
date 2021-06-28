//! Metrics service.

use crate::prelude_errors::*;
use actix_web::HttpResponse;
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
pub async fn serve<T>(app_data: actix_web::web::Data<T>) -> HttpResponse
where
    T: 'static + HasRegistry,
{
    use prometheus::Encoder;

    let metrics = app_data.registry().gather();
    let tenc = prometheus::TextEncoder::new();
    let mut buf = vec![];
    match tenc.encode(&metrics, &mut buf) {
        Ok(()) => HttpResponse::Ok().body(buf),
        Err(e) => HttpResponse::InternalServerError().message_body(format!("{}", e).into()),
    }
}

/// Create a custom Prometheus registry.
pub fn new_registry(prefix: Option<String>) -> Fallible<Registry> {
    Registry::new_custom(prefix.clone(), None).map_err(|e| {
        format_err!(format!(
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
    use memchr::memmem;

    #[test]
    fn serve_metrics_basic() -> Fallible<()> {
        let rt = testing::init_runtime()?;

        let metrics_prefix = "cincinnati";
        let registry_wrapped = RegistryWrapper(Box::leak(Box::new(new_registry(Some(
            metrics_prefix.to_string(),
        ))?)));

        testing::dummy_gauge(&registry_wrapped.0, 42.0)?;

        let metrics_call = serve::<RegistryWrapper>(actix_web::web::Data::new(registry_wrapped));
        let resp = rt.block_on(metrics_call);

        assert_eq!(resp.status(), 200);
        if let actix_web::body::ResponseBody::Body(body) = resp.body() {
            if let actix_web::body::Body::Bytes(bytes) = body {
                assert!(!bytes.is_empty());
                assert!(memmem::find_iter(
                    bytes.as_ref(),
                    format!("{}_dummy_gauge 42\n", metrics_prefix).as_bytes()
                )
                .next()
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
