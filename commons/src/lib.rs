//! Common utilities for Cincinnati backend.

#![deny(missing_docs)]

extern crate actix_web;
#[macro_use]
extern crate lazy_static;
#[macro_use]
extern crate serde_derive;

mod config;
pub use crate::config::MergeOptions;

pub mod de;
pub mod metrics;
pub mod testing;
pub mod tracing;

mod errors;
pub use errors::{register_metrics, Fallible, GraphError, MISSING_APPSTATE_PANIC_MSG};

/// Commonly used imports for error handling.
pub mod prelude_errors {
    pub use crate::errors::prelude::*;
}

use actix_web::http::header::{HeaderMap, HeaderValue, ACCEPT};
use std::collections::HashMap;
use std::collections::HashSet;
use url::form_urlencoded;

lazy_static! {
    /// list of cincinnati versions
    pub static ref CINCINNATI_VERSION: HashMap<&'static str, i32> =
        [("application/vnd.redhat.cincinnati.v1+json", 1)]
            .iter()
            .cloned()
            .collect();
    /// minimum cincinnati version supported
    pub static ref MIN_CINCINNATI_VERSION: &'static str = "application/vnd.redhat.cincinnati.v1+json";
}

/// Strip all but one leading slash and all trailing slashes
pub fn parse_path_prefix<S>(path_prefix: S) -> String
where
    S: AsRef<str>,
{
    format!("/{}", path_prefix.as_ref().to_string().trim_matches('/'))
}

/// Deserialize path_prefix
pub fn de_path_prefix<'de, D>(deserializer: D) -> Result<Option<String>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    use serde::Deserialize;
    let path_prefix = String::deserialize(deserializer)?;
    Ok(Some(parse_path_prefix(path_prefix)))
}

/// Parse a comma-separated set of client parameters keys.
pub fn parse_params_set<S>(params: S) -> HashSet<String>
where
    S: AsRef<str>,
{
    params
        .as_ref()
        .split(',')
        .filter_map(|key| {
            let trimmed = key.trim().to_string();
            if trimmed.is_empty() {
                None
            } else {
                Some(trimmed)
            }
        })
        .collect()
}

/// Make sure `query` string contains all `params` keys.
pub fn ensure_query_params(
    required_params: &HashSet<String>,
    query: &str,
) -> Result<(), GraphError> {
    // No mandatory parameters, always fine.
    if required_params.is_empty() {
        return Ok(());
    }

    // Extract and de-duplicate keys from input query.
    let query_keys: HashSet<String> = form_urlencoded::parse(query.as_bytes())
        .into_owned()
        .map(|(k, _)| k)
        .collect();

    // Make sure no mandatory parameters are missing.
    let mut missing: Vec<String> = required_params.difference(&query_keys).cloned().collect();
    if !missing.is_empty() {
        missing.sort();
        return Err(GraphError::MissingParams(missing));
    }

    Ok(())
}

/// Make sure the client can accept the provided media type.
pub fn validate_content_type(
    headers: &HeaderMap,
    mut content_type: Vec<HeaderValue>,
    accept_default: HeaderValue,
) -> Result<String, GraphError> {
    let header_value = match headers.get(ACCEPT) {
        None => {
            let minimum_version = MIN_CINCINNATI_VERSION.to_string();
            return Ok(minimum_version);
        }
        Some(v) => v,
    };

    let wildcard = HeaderValue::from_static("*");
    let double_wildcard = HeaderValue::from_static("*/*");

    let mut top_types: Vec<HeaderValue> = content_type
        .iter()
        .map(|ct| {
            let top_type = ct.to_str().unwrap_or("").split("/").next().unwrap_or("");
            let top_type_wildcard = HeaderValue::from_str(&format!("{}/*", top_type));
            assert!(
                top_type_wildcard.is_ok(),
                "could not form top-type wildcard from {}",
                top_type
            );
            top_type_wildcard.unwrap()
        })
        .collect();

    let mut acceptable_content_types: Vec<HeaderValue> =
        vec![wildcard, double_wildcard, accept_default.clone()];
    acceptable_content_types.append(&mut content_type);
    acceptable_content_types.append(&mut top_types);

    // FIXME: this is not a full-blown Accept parser
    if acceptable_content_types.iter().any(|c| c == header_value) {
        return if header_value
            .to_str()
            .unwrap_or("")
            .split("/")
            .any(|i| i == "*")
        {
            Ok(HeaderValue::to_str(&accept_default)
                .unwrap()
                .parse()
                .unwrap())
        } else {
            match HeaderValue::to_str(header_value) {
                Ok(a) => Ok(a.parse().unwrap()),
                Err(_e) => Ok(MIN_CINCINNATI_VERSION.to_string()),
            }
        };
    } else {
        Err(GraphError::InvalidContentType)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_path_prefix() {
        assert_eq!(parse_path_prefix("//a/b/c//"), "/a/b/c");
        assert_eq!(parse_path_prefix("/a/b/c/"), "/a/b/c");
        assert_eq!(parse_path_prefix("/a/b/c"), "/a/b/c");
        assert_eq!(parse_path_prefix("a/b/c"), "/a/b/c");
    }

    #[test]
    fn test_parse_params_set() {
        assert_eq!(parse_params_set(""), HashSet::new());

        let basic = vec!["a".to_string(), "b".to_string(), "c".to_string()];
        assert_eq!(parse_params_set("a,b,c"), basic.into_iter().collect());

        let dedup = vec!["a".to_string(), "b".to_string()];
        assert_eq!(parse_params_set("a,b,a"), dedup.into_iter().collect());

        let trimmed = vec!["foo".to_string(), "bar".to_string()];
        assert_eq!(
            parse_params_set("foo , , bar"),
            trimmed.into_iter().collect()
        );
    }

    #[test]
    fn test_ensure_query_params() {
        let empty = HashSet::new();
        ensure_query_params(&empty, "").unwrap();
        ensure_query_params(&empty, "a=b").unwrap();

        let simple = vec!["a".to_string()].into_iter().collect();
        ensure_query_params(&simple, "a=b").unwrap();
        ensure_query_params(&simple, "a=b&a=c").unwrap();
        ensure_query_params(&simple, "").unwrap_err();
        ensure_query_params(&simple, "c=d").unwrap_err();
    }

    #[test]
    fn test_validate_content_type() {
        let most_recent_version = "application/vnd.redhat.cincinnati.v1+json";
        let all_supported_versions: Vec<HeaderValue> = CINCINNATI_VERSION
            .keys()
            .map(|val| HeaderValue::from_static(val))
            .collect();

        // Test for empty header
        // No accept value provided with header, server accepts `application/json` and defaults to `application/json`
        let mut headers = HeaderMap::new();
        let accept_default = HeaderValue::from_str("application/json").unwrap();
        let version = validate_content_type(
            &headers,
            vec![accept_default.clone()],
            accept_default.clone(),
        )
        .unwrap(); // if the request leaves Accept empty, we return the minimum supported cincinnati version as that's the lowest version we support
        assert_eq!(version, MIN_CINCINNATI_VERSION.to_string());

        // Support old clients with older cincinnati config.
        // `application/json` provided with header, server accepts `application/json` and defaults to `application/json`
        headers.insert(
            ACCEPT,
            //"application/json, text/*; q=0.2".parse().unwrap(), // prefer JSON, but also accept any text/* after an 80% markdown in quality.  FIXME: needs a smarter parser in validate_content_type
            "application/json".parse().unwrap(),
        );
        let version = validate_content_type(
            &headers,
            vec![accept_default.clone()],
            accept_default.clone(),
        )
        .unwrap();
        assert_eq!(version, "application/json");

        // `application/*` provided with header, server accepts `application/json` and defaults to `application/json`
        headers.insert(ACCEPT, "application/*".parse().unwrap());
        let version = validate_content_type(
            &headers,
            vec![accept_default.clone()],
            accept_default.clone(),
        )
        .unwrap();
        assert_eq!(version, "application/json");

        // Incompatible Accept header
        // `image/png` provided with header, server accepts `application/json` and defaults to `application/json`
        let image_type: Vec<HeaderValue> = vec![HeaderValue::from_str("image/png").unwrap()];
        //server should throw error on non-supported ACCEPT
        validate_content_type(&headers, image_type, accept_default.clone()).unwrap_err();

        // Check latest version with all accepted version types
        // `most_recent_version` provided with header, server accepts
        // `all_supported_versions` and defaults to `application/json`
        headers.insert(ACCEPT, most_recent_version.parse().unwrap());
        let version = validate_content_type(
            &headers,
            all_supported_versions.clone(),
            accept_default.clone(),
        )
        .unwrap();
        // Server returns the response with content_type `most_recent_version`
        assert_eq!(version, most_recent_version);

        // Support old clients with proactive negotiation config.
        // `application/json` provided with header, server accepts `all_supported_versions`
        // and defaults to `application/json`
        headers.insert(ACCEPT, "application/json".parse().unwrap());
        let version =
            validate_content_type(&headers, all_supported_versions, accept_default.clone())
                .unwrap();
        // Server returns the response with content_type `application/json`
        assert_eq!(version, "application/json");

        // Test function with non `application` input. Input is valid for function.
        //`text/*` provided with header, server accepts `text/plain` and defaults to `text/plain`
        headers.insert(
            // FIXME: drop once validate_content_type gets a smarter parser and the previous insert can include the text/* entry
            ACCEPT,
            "text/*".parse().unwrap(),
        );
        let accept_default = HeaderValue::from_str("text/plain").unwrap();
        let version = validate_content_type(
            &headers,
            vec![accept_default.clone()],
            accept_default.clone(),
        )
        .unwrap();
        assert_eq!(version, "text/plain");
    }
}
