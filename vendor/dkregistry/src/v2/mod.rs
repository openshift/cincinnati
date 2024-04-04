//! Client library for Docker Registry API v2.
//!
//! This module provides a `Client` which can be used to list
//! images and tags, to check for the presence of blobs (manifests,
//! layers and other objects) by digest, and to retrieve them.
//!
//! ## Example
//!
//! ```rust,no_run
//! # extern crate dkregistry;
//! # extern crate tokio;
//! # #[tokio::main]
//! # async fn main() {
//! # async fn run() -> dkregistry::errors::Result<()> {
//! #
//! use dkregistry::v2::Client;
//!
//! // Retrieve an image manifest.
//! let dclient = Client::configure()
//!                      .registry("quay.io")
//!                      .build()?;
//! let manifest = dclient.get_manifest("coreos/etcd", "v3.1.0").await?;
//! #
//! # Ok(())
//! # };
//! # run().await.unwrap();
//! # }
//! ```

use crate::errors::*;
use crate::mediatypes::MediaTypes;
use futures::prelude::*;
use reqwest::{Method, StatusCode, Url};

mod config;
pub use self::config::Config;

mod catalog;

mod auth;
pub use auth::WwwHeaderParseError;

pub mod manifest;

mod tags;

mod blobs;

mod content_digest;
pub(crate) use self::content_digest::ContentDigest;
pub use self::content_digest::ContentDigestError;

/// A Client to make outgoing API requests to a registry.
#[derive(Clone, Debug)]
pub struct Client {
    base_url: String,
    credentials: Option<(String, String)>,
    user_agent: Option<String>,
    auth: Option<auth::Auth>,
    client: reqwest::Client,
    accepted_types: Vec<(MediaTypes, Option<f64>)>,
}

impl Client {
    pub fn configure() -> Config {
        Config::default()
    }

    /// Ensure remote registry supports v2 API.
    pub async fn ensure_v2_registry(self) -> Result<Self> {
        if !self.is_v2_supported().await? {
            Err(Error::V2NotSupported)
        } else {
            Ok(self)
        }
    }

    /// Check whether remote registry supports v2 API.
    pub async fn is_v2_supported(&self) -> Result<bool> {
        match self.is_v2_supported_and_authorized().await {
            Ok((v2_supported, _)) => Ok(v2_supported),
            Err(crate::Error::UnexpectedHttpStatus(_)) => Ok(false),
            Err(e) => Err(e),
        }
    }

    /// Check whether remote registry supports v2 API and `self` is authorized.
    /// Authorized means to successfully GET the `/v2` endpoint on the remote registry.
    pub async fn is_v2_supported_and_authorized(&self) -> Result<(bool, bool)> {
        let api_header = "Docker-Distribution-API-Version";
        let api_version = "registry/2.0";

        // GET request to bare v2 endpoint.
        let v2_endpoint = format!("{}/v2/", self.base_url);
        let request = reqwest::Url::parse(&v2_endpoint).map(|url| {
            trace!("GET {:?}", url);
            self.build_reqwest(Method::GET, url)
        })?;

        let response = request.send().await?;

        let b = match (response.status(), response.headers().get(api_header)) {
            (StatusCode::OK, Some(x)) => Ok((x == api_version, true)),
            (StatusCode::UNAUTHORIZED, Some(x)) => Ok((x == api_version, false)),
            (s, v) => {
                trace!("Got unexpected status {}, header version {:?}", s, v);
                return Err(crate::Error::UnexpectedHttpStatus(s));
            }
        };

        b
    }

    /// Takes reqwest's async RequestBuilder and injects an authentication header if a token is present
    fn build_reqwest(&self, method: Method, url: Url) -> reqwest::RequestBuilder {
        let mut builder = self.client.request(method, url);

        if let Some(auth) = &self.auth {
            builder = auth.add_auth_headers(builder);
        };

        if let Some(ua) = &self.user_agent {
            builder = builder.header(reqwest::header::USER_AGENT, ua.as_str());
        };

        builder
    }
}

#[derive(Debug, Default, Deserialize, Serialize)]
struct ApiError {
    code: String,
    message: String,
    detail: String,
}

#[derive(Debug, Default, Deserialize, Serialize)]
struct Errors {
    errors: Vec<ApiError>,
}
