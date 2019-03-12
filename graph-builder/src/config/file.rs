//! TOML file configuration options.

use failure::{Fallible, ResultExt};
use std::collections::{HashMap, HashSet};
use std::io::Read;
use std::{fs, io, path};

/// TOML configuration, top-level.
#[derive(Debug, Deserialize)]
pub struct FileOptions {
    /// Verbosity level.
    pub verbosity: Option<u8>,

    /// Upstream options.
    pub upstream: Option<UpstreamOptions>,

    /// Web frontend options.
    pub service: Option<ServiceOptions>,

    /// Plugins ordering and options.
    pub plugins: Option<Vec<HashMap<String, String>>>,
}

impl FileOptions {
    pub fn read_filepath<P: AsRef<path::Path>>(cfg_path: P) -> Fallible<Self> {
        let cfg_file = fs::File::open(&cfg_path).context(format!(
            "failed to open config path {:?}",
            cfg_path.as_ref()
        ))?;
        let mut bufrd = io::BufReader::new(cfg_file);

        let mut content = vec![];
        bufrd.read_to_end(&mut content)?;
        let cfg = toml::from_slice(&content).context(format!(
            "failed to read config file {}",
            cfg_path.as_ref().display()
        ))?;

        Ok(cfg)
    }
}

/// TOML configuration, upstream fetcher.
#[derive(Debug, Deserialize)]
pub struct UpstreamOptions {
    /// Fetcher method.
    pub method: Option<String>,

    /// Docker-registry v2 upstream options.
    pub registry: Option<RegistryOptions>,
}

/// CLI configuration flags, HTTP frontend serving Cincinnati.
#[derive(Debug, Deserialize)]
pub struct ServiceOptions {
    /// Address on which the server will listen
    pub address: Option<String>,

    /// Port to which the server will bind
    pub port: Option<u16>,

    /// Path prefix for all paths.
    pub path_prefix: Option<String>,

    /// Comma-separated set of mandatory client parameters.
    pub mandatory_client_parameters: Option<HashSet<String>>,
}

/// TOML configuration, Docker-v2 registry fetcher.
#[derive(Debug, Deserialize)]
pub struct RegistryOptions {
    /// Duration of the pause (in seconds) between registry scans.
    pub period: Option<u64>,

    /// URL for the container image registry.
    pub url: Option<String>,

    /// Name of the container image repository.
    pub repository: Option<String>,

    /// Credentials file for authentication against the image registry.
    pub credentials_path: Option<String>,

    /// Metadata key where to record the manifest-reference.
    pub manifestref_key: Option<String>,
}
