use crate::{mediatypes::MediaTypes, v2::*};

/// Configuration for a `Client`.
#[derive(Debug)]
pub struct Config {
    index: String,
    insecure_registry: bool,
    user_agent: Option<String>,
    username: Option<String>,
    password: Option<String>,
    accept_invalid_certs: bool,
    accepted_types: Option<Vec<(MediaTypes, Option<f64>)>>,
}

impl Config {
    /// Initialize `Config` with default values.
    pub fn default() -> Self {
        Self {
            index: "registry-1.docker.io".into(),
            insecure_registry: false,
            accept_invalid_certs: false,
            accepted_types: None,
            user_agent: Some(crate::USER_AGENT.to_owned()),
            username: None,
            password: None,
        }
    }

    /// Set registry service to use (vhost or IP).
    pub fn registry(mut self, reg: &str) -> Self {
        self.index = reg.to_owned();
        self
    }

    /// Whether to use an insecure HTTP connection to the registry.
    pub fn insecure_registry(mut self, insecure: bool) -> Self {
        self.insecure_registry = insecure;
        self
    }

    /// Set whether or not to accept invalid certificates.
    pub fn accept_invalid_certs(mut self, accept_invalid_certs: bool) -> Self {
        self.accept_invalid_certs = accept_invalid_certs;
        self
    }

    /// Set custom Accept headers
    pub fn accepted_types(
        mut self,
        accepted_types: Option<Vec<(MediaTypes, Option<f64>)>>,
    ) -> Self {
        self.accepted_types = accepted_types;
        self
    }

    /// Set the user-agent to be used for registry authentication.
    pub fn user_agent(mut self, user_agent: Option<String>) -> Self {
        self.user_agent = user_agent;
        self
    }

    /// Set the username to be used for registry authentication.
    pub fn username(mut self, user: Option<String>) -> Self {
        self.username = user;
        self
    }

    /// Set the password to be used for registry authentication.
    pub fn password(mut self, password: Option<String>) -> Self {
        self.password = password;
        self
    }

    /// Read credentials from a JSON config file
    pub fn read_credentials<T: ::std::io::Read>(mut self, reader: T) -> Self {
        if let Ok(creds) = crate::get_credentials(reader, &self.index) {
            self.username = creds.0;
            self.password = creds.1;
        };
        self
    }

    /// Return a `Client` to interact with a v2 registry.
    pub fn build(self) -> Result<Client> {
        let base = if self.insecure_registry {
            "http://".to_string() + &self.index
        } else {
            "https://".to_string() + &self.index
        };
        trace!(
            "Built client for {:?}: endpoint {:?} - user {:?}",
            self.index,
            base,
            self.username
        );
        let creds = match (self.username, self.password) {
            (None, None) => None,
            (u, p) => Some((
                u.unwrap_or_else(|| "".into()),
                p.unwrap_or_else(|| "".into()),
            )),
        };
        let client = reqwest::ClientBuilder::new()
            .danger_accept_invalid_certs(self.accept_invalid_certs)
            .build()?;

        let accepted_types = match self.accepted_types {
            Some(a) => a,
            None => match self.index == "gcr.io" || self.index.ends_with(".gcr.io") {
                false => vec![
                    // accept header types and their q value, as documented in
                    // https://tools.ietf.org/html/rfc7231#section-5.3.2
                    (MediaTypes::ManifestV2S2, Some(0.5)),
                    (MediaTypes::ManifestV2S1Signed, Some(0.4)),
                    // TODO(steveeJ): uncomment this when all the Manifest methods work for it
                    // mediatypes::MediaTypes::ManifestList,
                ],
                // GCR incorrectly parses `q` parameters, so we use special Accept for it.
                // Bug: https://issuetracker.google.com/issues/159827510.
                // TODO: when bug is fixed, this workaround should be removed.
                true => vec![
                    (MediaTypes::ManifestV2S2, None),
                    (MediaTypes::ManifestV2S1Signed, None),
                ],
            },
        };
        let c = Client {
            base_url: base,
            credentials: creds,
            user_agent: self.user_agent,
            auth: None,
            client,
            accepted_types,
        };
        Ok(c)
    }
}
