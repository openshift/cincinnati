//! Tracing service.

use opentelemetry::{
    global,
    propagation::{Extractor, Injector, TextMapPropagator},
    sdk::{
        propagation::TraceContextPropagator,
        trace::{Config, Sampler, TracerProvider as sdk_tracerprovider},
    },
    trace::{Span, TracerProvider},
    Context, Key,
};

use std::collections::HashMap;

use actix_web::dev::ServiceRequest;
use actix_web::http;
use reqwest::header::{HeaderMap, HeaderName, HeaderValue};

use crate::prelude_errors::*;

/// init_tracer sets up Jaeger tracer
pub fn init_tracer(name: &'static str, maybe_agent_endpoint: Option<String>) -> Fallible<()> {
    // Skip provider config if agent endpoint is not set
    let agent_endpoint = match maybe_agent_endpoint {
        None => return Ok(()),
        Some(s) => s,
    };

    let exporter = opentelemetry_jaeger::new_pipeline()
        .with_agent_endpoint(agent_endpoint)
        .with_service_name(name.to_string())
        .with_tags(vec![Key::new("exporter").string("jaeger")])
        .init_exporter()?;

    let provider = sdk_tracerprovider::builder()
        .with_simple_exporter(exporter)
        .with_config(Config {
            sampler: Box::new(Sampler::AlwaysOn),
            ..Default::default()
        })
        .build();
    global::set_tracer_provider(provider);

    Ok(())
}

/// get_tracer returns an instance of global tracer
pub fn get_tracer() -> global::BoxedTracer {
    global::tracer_provider().get_tracer("", None)
}

struct HttpHeaderMapCarrier<'a>(&'a http::HeaderMap);
impl<'a> Extractor for HttpHeaderMapCarrier<'a> {
    fn get(&self, key: &str) -> Option<&str> {
        self.0.get(key).and_then(|value| value.to_str().ok())
    }
    fn keys(&self) -> Vec<&str> {
        unimplemented!()
    }
}
impl<'a> Injector for HttpHeaderMapCarrier<'a> {
    fn set(&mut self, _key: &str, _value: String) {
        unimplemented!()
    }
}

struct ClientHeaderMapCarrier(HashMap<String, String>);
impl Extractor for ClientHeaderMapCarrier {
    fn get(&self, key: &str) -> Option<&str> {
        self.0.get(key).map(|s| s.as_str())
    }

    fn keys(&self) -> Vec<&str> {
        unimplemented!()
    }
}
impl Injector for ClientHeaderMapCarrier {
    fn set(&mut self, key: &str, value: String) {
        self.0.insert(key.to_string(), value);
    }
}

/// Return the parent context for the request if specific headers found.
pub fn get_context(req: &ServiceRequest) -> Context {
    let propagator = TraceContextPropagator::new();
    propagator.extract(&HttpHeaderMapCarrier(&req.headers()))
}

/// Inject context data into headers
pub fn set_context(context: Context, headers: &mut HeaderMap) -> crate::errors::Fallible<()> {
    use std::str::FromStr;

    let mut carrier = {
        let headers_converted = headers.iter().try_fold(
            HashMap::<String, String>::with_capacity(headers.len()),
            |mut sum, (name, value)| -> crate::errors::Fallible<_> {
                sum.insert(name.as_str().to_string(), value.to_str()?.to_string());
                Ok(sum)
            },
        )?;

        ClientHeaderMapCarrier(headers_converted)
    };

    let propagator = TraceContextPropagator::new();
    propagator.inject_context(&context, &mut carrier);

    for (name, value) in carrier.0 {
        headers.insert(HeaderName::from_str(&name)?, HeaderValue::from_str(&value)?);
    }

    Ok(())
}

/// Add span attributes from servicerequest
pub fn set_span_tags(req_path: &str, headers: &http::header::HeaderMap, span: &mut dyn Span) {
    span.set_attribute(Key::new("path").string(req_path.to_string()));
    headers.iter().for_each(|(k, v)| {
        let value = v.to_str().unwrap().to_string();
        span.set_attribute(Key::new(format!("header.{}", k)).string(value))
    });
}
