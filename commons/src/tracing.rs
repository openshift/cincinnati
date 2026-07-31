//! Tracing service.

use opentelemetry::{
    global,
    propagation::{Extractor, Injector, TextMapPropagator},
    trace::Span,
    Context, KeyValue,
};
use opentelemetry_otlp::WithExportConfig;
use opentelemetry_sdk::{
    propagation::TraceContextPropagator,
    trace::{Sampler, SdkTracerProvider},
    Resource,
};

use std::collections::HashMap;

use actix_web::dev::ServiceRequest;
use actix_web::http::header::HeaderMap as HttpHeaderMap;
use reqwest::header::{HeaderMap, HeaderName, HeaderValue};

use crate::prelude_errors::*;

/// init_tracer sets up an OTLP tracer (compatible with Jaeger ≥1.35 and any OTLP collector).
pub fn init_tracer(name: &'static str, maybe_agent_endpoint: Option<String>) -> Fallible<()> {
    // Skip provider config if agent endpoint is not set
    let agent_endpoint = match maybe_agent_endpoint {
        None => return Ok(()),
        Some(s) => s,
    };

    let exporter = opentelemetry_otlp::SpanExporter::builder()
        .with_http()
        .with_endpoint(agent_endpoint)
        .build()?;

    let provider = SdkTracerProvider::builder()
        .with_sampler(Sampler::AlwaysOn)
        .with_resource(
            Resource::builder_empty()
                .with_attribute(KeyValue::new("service.name", name))
                .build(),
        )
        .with_simple_exporter(exporter)
        .build();
    global::set_tracer_provider(provider);

    Ok(())
}

/// get_tracer returns an instance of global tracer
pub fn get_tracer() -> global::BoxedTracer {
    global::tracer("cincinnati")
}

struct HttpHeaderMapCarrier<'a>(&'a HttpHeaderMap);
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
    propagator.extract(&HttpHeaderMapCarrier(req.headers()))
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
pub fn set_span_tags<S: Span>(req_path: &str, headers: &HttpHeaderMap, span: &mut S) {
    span.set_attribute(KeyValue::new("path", req_path.to_string()));
    headers.iter().for_each(|(k, v)| {
        let value = v.to_str().unwrap().to_string();
        span.set_attribute(KeyValue::new(format!("header.{}", k), value))
    });
}
