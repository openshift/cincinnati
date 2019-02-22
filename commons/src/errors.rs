use actix_web::http;
use actix_web::HttpResponse;

#[derive(Debug, Fail, Eq, PartialEq)]
pub enum GraphError {
    #[fail(display = "failed to deserialize JSON: {}", _0)]
    FailedJsonIn(String),
    #[fail(display = "failed to serialize JSON: {}", _0)]
    FailedJsonOut(String),
    #[fail(display = "failed to fetch upstream graph: {}", _0)]
    FailedUpstreamFetch(String),
    #[fail(display = "failed to execute plugins: {}", _0)]
    FailedPluginExecution(String),
    #[fail(display = "failed to assemble upstream request")]
    FailedUpstreamRequest,
    #[fail(display = "invalid Content-Type requested")]
    InvalidContentType,
    #[fail(display = "mandatory client parameters missing")]
    MissingParams,
}

impl actix_web::error::ResponseError for GraphError {
    fn error_response(&self) -> HttpResponse {
        self.as_json_error()
    }
}

impl GraphError {
    // Return the HTTP JSON error response.
    pub fn as_json_error(&self) -> HttpResponse {
        let code = self.status_code();
        let json_body = json!({
            "kind": self.kind(),
            "value": self.value(),
        });
        HttpResponse::build(code).json(json_body)
    }

    // Return the HTTP status code for the error.
    fn status_code(&self) -> http::StatusCode {
        match *self {
            GraphError::FailedJsonIn(_) => http::StatusCode::INTERNAL_SERVER_ERROR,
            GraphError::FailedJsonOut(_) => http::StatusCode::INTERNAL_SERVER_ERROR,
            GraphError::FailedUpstreamFetch(_) => http::StatusCode::INTERNAL_SERVER_ERROR,
            GraphError::FailedPluginExecution(_) => http::StatusCode::INTERNAL_SERVER_ERROR,
            GraphError::FailedUpstreamRequest => http::StatusCode::INTERNAL_SERVER_ERROR,
            GraphError::InvalidContentType => http::StatusCode::NOT_ACCEPTABLE,
            GraphError::MissingParams => http::StatusCode::BAD_REQUEST,
        }
    }

    // Return the kind for the error.
    fn kind(&self) -> String {
        let kind = match *self {
            GraphError::FailedJsonIn(_) => "failed_json_in",
            GraphError::FailedJsonOut(_) => "failed_json_out",
            GraphError::FailedUpstreamFetch(_) => "failed_upstream_fetch",
            GraphError::FailedPluginExecution(_) => "failed_plugin_execution",
            GraphError::FailedUpstreamRequest => "failed_upstream_request",
            GraphError::InvalidContentType => "invalid_content_type",
            GraphError::MissingParams => "missing_params",
        };
        kind.to_string()
    }

    // Return the value for the error.
    fn value(&self) -> String {
        format!("{}", self)
    }
}
