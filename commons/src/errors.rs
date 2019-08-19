use actix_web::http;
use actix_web::HttpResponse;
use failure::Fallible;
use prometheus::{IntCounterVec, Opts, Registry};

lazy_static! {
    static ref V1_GRAPH_ERRORS: IntCounterVec = IntCounterVec::new(
        Opts::new(
            "v1_graph_response_errors_total",
            "Error responses on /v1/graph"
        ),
        &["kind"]
    )
    .unwrap();
}

/// Panic message for a request with a missing appstate.
/// This panic is used to guarantee internal consistency.
pub static MISSING_APPSTATE_PANIC_MSG: &str =
    "the request has no app_data attached. this is a bug.";

/// Register relevant metrics to a prometheus registry.
pub fn register_metrics(registry: &Registry) -> Fallible<()> {
    registry.register(Box::new(V1_GRAPH_ERRORS.clone()))?;
    Ok(())
}

#[derive(Debug, Fail, Eq, PartialEq)]
/// Error that can be returned by `/v1/graph` endpoint.
pub enum GraphError {
    /// Failed to deserialize JSON.
    #[fail(display = "failed to deserialize JSON: {}", _0)]
    FailedJsonIn(String),

    /// Failed to serialize JSON.
    #[fail(display = "failed to serialize JSON: {}", _0)]
    FailedJsonOut(String),

    /// Error response from upstream.
    #[fail(display = "failed to fetch upstream graph: {}", _0)]
    FailedUpstreamFetch(String),

    /// Plugin failure.
    #[fail(display = "failed to execute plugins: {}", _0)]
    FailedPluginExecution(String),

    /// Error while reaching upstream.
    #[fail(display = "failed to assemble upstream request")]
    FailedUpstreamRequest,

    /// Requested invalid mediatype.
    #[fail(display = "invalid Content-Type requested")]
    InvalidContentType,

    /// Missing client parameters.
    #[fail(display = "mandatory client parameters missing")]
    MissingParams(Vec<String>),

    /// Invalid client parameters.
    #[fail(display = "invalid client parameters")]
    InvalidParams(String),
}

impl actix_web::error::ResponseError for GraphError {
    fn render_response(&self) -> HttpResponse {
        let kind = self.kind();
        V1_GRAPH_ERRORS.with_label_values(&[&kind]).inc();
        self.as_json_error()
    }
}

impl GraphError {
    /// Return the HTTP JSON error response.
    pub fn as_json_error(&self) -> HttpResponse {
        let code = self.status_code();
        let json_body = json!({
            "kind": self.kind(),
            "value": self.value(),
        });
        HttpResponse::build(code).json(json_body)
    }

    /// Return the HTTP status code for the error.
    pub fn status_code(&self) -> http::StatusCode {
        match *self {
            GraphError::FailedJsonIn(_) => http::StatusCode::INTERNAL_SERVER_ERROR,
            GraphError::FailedJsonOut(_) => http::StatusCode::INTERNAL_SERVER_ERROR,
            GraphError::FailedUpstreamFetch(_) => http::StatusCode::INTERNAL_SERVER_ERROR,
            GraphError::FailedPluginExecution(_) => http::StatusCode::INTERNAL_SERVER_ERROR,
            GraphError::FailedUpstreamRequest => http::StatusCode::INTERNAL_SERVER_ERROR,
            GraphError::InvalidContentType => http::StatusCode::NOT_ACCEPTABLE,
            GraphError::MissingParams(_) => http::StatusCode::BAD_REQUEST,
            GraphError::InvalidParams(_) => http::StatusCode::BAD_REQUEST,
        }
    }

    /// Return the kind for the error.
    pub fn kind(&self) -> String {
        let kind = match *self {
            GraphError::FailedJsonIn(_) => "failed_json_in",
            GraphError::FailedJsonOut(_) => "failed_json_out",
            GraphError::FailedUpstreamFetch(_) => "failed_upstream_fetch",
            GraphError::FailedPluginExecution(_) => "failed_plugin_execution",
            GraphError::FailedUpstreamRequest => "failed_upstream_request",
            GraphError::InvalidContentType => "invalid_content_type",
            GraphError::MissingParams(_) => "missing_params",
            GraphError::InvalidParams(_) => "invalid_params",
        };
        kind.to_string()
    }

    /// Return the value for the error.
    pub fn value(&self) -> String {
        let error_msg = format!("{}", self);
        match self {
            GraphError::MissingParams(params) => format!("{}: {}", error_msg, params.join(", ")),
            _ => error_msg,
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::ensure_query_params;

    #[test]
    fn error_msg_missing_params() {
        let expected = vec!["bar".to_string(), "foo".to_string()]
            .into_iter()
            .collect();
        let err_msg = ensure_query_params(&expected, "key=value")
            .unwrap_err()
            .value();

        assert!(err_msg.contains("bar, foo"), "unexpected: {}", err_msg);
        assert!(!err_msg.contains("key"), "unexpected: {}", err_msg);
    }
}
