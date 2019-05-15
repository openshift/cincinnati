use failure::Fallible;
use reqwest;

mod manifest;

mod tag;
pub use self::tag::Tag;

pub static DEFAULT_API_BASE: &str = "https://quay.io/api/v1/";

/// Client to make outgoing API requests to a quay instance.
#[derive(Clone, Debug)]
pub struct Client {
    /// Base URL for API endpoint.
    api_base: reqwest::Url,
    /// Asynchronous reqwest client.
    hclient: reqwest::async::Client,
    /// Authentication token.
    token: Option<String>,
}

impl Client {
    /// Return a client builder with default options.
    pub fn builder() -> ClientBuilder {
        ClientBuilder::default()
    }

    /// Return a request builder with base URL and parameters set.
    pub(crate) fn new_request<S: AsRef<str>>(
        &self,
        method: reqwest::Method,
        url_suffix: S,
    ) -> Fallible<reqwest::async::RequestBuilder> {
        let url = self.api_base.clone().join(url_suffix.as_ref())?;
        let builder = {
            let plain = self.hclient.request(method, url);
            match self.token {
                None => plain,
                Some(ref token) => {
                    let bearer_token = format!("Bearer {}", token);
                    plain.header("Authorization", bearer_token)
                }
            }
        };
        Ok(builder)
    }
}

/// Client to make outgoing API requests to a quay instance.
#[derive(Clone, Debug)]
pub struct ClientBuilder {
    api_base: Option<String>,
    hclient: Option<reqwest::async::Client>,
    token: Option<String>,
}

impl ClientBuilder {
    /// Set (or reset) the HTTP client to use.
    pub fn http_client(self, hclient: Option<reqwest::async::Client>) -> Self {
        let mut builder = self;
        builder.hclient = hclient;
        builder
    }

    /// Set (or reset) the access token to use.
    pub fn access_token(self, token: Option<String>) -> Self {
        let mut builder = self;
        builder.token = token;
        builder
    }

    /// Set (or reset) the base API endpoint URL to use.
    pub fn api_base(self, api_base: Option<String>) -> Self {
        let mut builder = self;
        builder.api_base = api_base;
        builder
    }

    /// Build a client with specified parameters.
    pub fn build(self) -> Fallible<Client> {
        let hclient = match self.hclient {
            Some(client) => client,
            None => reqwest::async::ClientBuilder::new().build()?,
        };
        let api_base = match self.api_base {
            Some(ref base) => reqwest::Url::parse(base)?,
            None => reqwest::Url::parse(DEFAULT_API_BASE)?,
        };
        let quay_client = Client {
            api_base,
            hclient,
            token: self.token,
        };
        Ok(quay_client)
    }
}

impl Default for ClientBuilder {
    fn default() -> Self {
        Self {
            api_base: Some(DEFAULT_API_BASE.to_string()),
            hclient: None,
            token: None,
        }
    }
}
