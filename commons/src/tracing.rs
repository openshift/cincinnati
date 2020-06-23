//! Tracing service.

use opentelemetry::api::{
    Carrier, HttpTextFormat, Key, Provider, Span, SpanContext, TraceContextPropagator,
};
use opentelemetry::{global, sdk};
use opentelemetry_jaeger::{Exporter, Process};
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

    let exporter = Exporter::builder()
        .with_agent_endpoint(agent_endpoint)
        .with_process(Process {
            service_name: name.to_string(),
            tags: vec![Key::new("exporter").string("jaeger")],
        })
        .init()?;

    let provider = sdk::Provider::builder()
        .with_simple_exporter(exporter)
        .with_config(sdk::Config {
            default_sampler: Box::new(sdk::Sampler::Always),
            ..Default::default()
        })
        .build();
    global::set_provider(provider);

    Ok(())
}

/// get_tracer returns an instance of global tracer
pub fn get_tracer() -> global::BoxedTracer {
    global::trace_provider().get_tracer("")
}

struct HttpHeaderMapCarrier<'a>(&'a http::HeaderMap);
impl<'a> Carrier for HttpHeaderMapCarrier<'a> {
    fn get(&self, key: &'static str) -> Option<&str> {
        self.0.get(key).and_then(|value| value.to_str().ok())
    }

    fn set(&mut self, _key: &'static str, _value: String) {
        unimplemented!()
    }
}

struct ClientHeaderMapCarrier(HashMap<String, String>);
impl Carrier for ClientHeaderMapCarrier {
    fn get(&self, key: &'static str) -> Option<&str> {
        self.0.get(key).map(|s| s.as_str())
    }

    fn set(&mut self, key: &'static str, value: String) {
        self.0.insert(key.to_string(), value);
    }
}

/// Return the parent context for the request if specific headers found.
pub fn get_context(req: &ServiceRequest) -> SpanContext {
    let propagator = TraceContextPropagator::new();
    propagator.extract(&HttpHeaderMapCarrier(&req.headers()))
}

/// Inject context data into headers
pub fn set_context(context: SpanContext, headers: &mut HeaderMap) -> crate::errors::Fallible<()> {
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
    propagator.inject(context, &mut carrier);

    for (name, value) in carrier.0 {
        headers.insert(HeaderName::from_str(&name)?, HeaderValue::from_str(&value)?);
    }

    Ok(())
}

/// Add span attributes from servicerequest
pub fn set_span_tags(req: &ServiceRequest, span: &dyn Span) {
    span.set_attribute(Key::new("path").string(req.path()));
    req.headers().iter().for_each(|(k, v)| {
        span.set_attribute(Key::new(format!("header.{}", k)).bytes(v.as_bytes().to_vec()))
    });
}
