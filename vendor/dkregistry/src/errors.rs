//! Defines root error type

#[non_exhaustive]
#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("base64 decode error")]
    Base64Decode(#[from] base64::DecodeError),
    #[error("header parse error")]
    HeaderParse(#[from] http::header::ToStrError),
    #[error("json error")]
    Json(#[from] serde_json::Error),
    #[error("http transport error: {0}")]
    Reqwest(#[from] reqwest::Error),
    #[error("URI parse error")]
    Uri(#[from] url::ParseError),
    #[error("input is not UTF-8")]
    Ut8Parse(#[from] std::string::FromUtf8Error),
    #[error("strum error")]
    StrumParse(#[from] strum::ParseError),
    #[error("authentication information missing for index {0}")]
    AuthInfoMissing(String),
    #[error("unknown media type {0:?}")]
    UnknownMimeType(mime::Mime),
    #[error("unknown media type {0:?}")]
    UnsupportedMediaType(crate::mediatypes::MediaTypes),
    #[error("mime parse error")]
    MimeParse(#[from] mime::FromStrError),
    #[error("missing authentication header {0}")]
    MissingAuthHeader(&'static str),
    #[error("unexpected HTTP status {0}")]
    UnexpectedHttpStatus(http::StatusCode),
    #[error("invalid auth token '{0}'")]
    InvalidAuthToken(String),
    #[error("API V2 not supported")]
    V2NotSupported,
    #[error("obtained token is invalid")]
    LoginReturnedBadToken,
    #[error("www-authenticate header parse error")]
    Www(#[from] crate::v2::WwwHeaderParseError),
    #[error("request failed with status {status}")]
    Client { status: http::StatusCode },
    #[error("request failed with status {status}")]
    Server { status: http::StatusCode },
    #[error("content digest error")]
    ContentDigestParse(#[from] crate::v2::ContentDigestError),
    #[error("no header Content-Type given and no workaround to apply")]
    MediaTypeSniff,
    #[error("manifest error")]
    Manifest(#[from] crate::v2::manifest::ManifestError),
    #[error("reference is invalid")]
    ReferenceParse(#[from] crate::reference::ReferenceParseError),
    #[error("requested operation requires that credentials are available")]
    NoCredentials,
}

pub type Result<T> = std::result::Result<T, Error>;

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_error_bounds() {
        fn check_bounds<T: Send + Sync + 'static>() {}
        check_bounds::<Error>();
    }
}
