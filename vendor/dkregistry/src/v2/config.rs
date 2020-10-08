use crate::v2::*;

/// Configuration for a `Client`.
#[derive(Debug)]
pub struct Config {
    index: String,
    insecure_registry: bool,
    user_agent: Option<String>,
    username: Option<String>,
    password: Option<String>,
    accept_invalid_certs: bool,
}

impl Config {
    /// Initialize `Config` with default values.
    pub fn default() -> Self {
        Self {
            index: "registry-1.docker.io".into(),
            insecure_registry: false,
            accept_invalid_certs: false,
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

        let c = Client {
            base_url: base,
            credentials: creds,
            index: self.index,
            user_agent: self.user_agent,
            auth: None,
            client: client,
        };
        Ok(c)
    }
}
