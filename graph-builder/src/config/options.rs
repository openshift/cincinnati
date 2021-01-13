//! Options shared by CLI and TOML.

use super::AppSettings;
use commons::prelude_errors::*;
use commons::{de_path_prefix, parse_params_set, parse_path_prefix, MergeOptions};
use std::collections::HashSet;
use std::net::IpAddr;
use std::path::PathBuf;
use std::time::Duration;

// TODO(lucab): drop all aliases after staging+production deployments
// have been aligned on new flags.

/// Status service options.
#[derive(Debug, Deserialize, Serialize, StructOpt)]
pub struct StatusOptions {
    /// Address on which the status service will listen
    #[structopt(name = "status_address", long = "status.address")]
    pub address: Option<IpAddr>,

    /// Port to which the status service will bind
    #[structopt(name = "status_port", long = "status.port")]
    pub port: Option<u16>,
}

/// Options for the main Cincinnati service.
#[derive(Debug, Deserialize, Serialize, StructOpt)]
pub struct ServiceOptions {
    /// Duration of the pause (in seconds) between registry scans
    #[structopt(
        long = "service.pause_secs",
        parse(try_from_str = duration_from_secs)
    )]
    #[serde(default = "Option::default", deserialize_with = "de_duration_secs")]
    pub pause_secs: Option<Duration>,

    /// Timeout for a single scrape in seconds
    #[structopt(
        long = "service.scrape_timeout",
        parse(try_from_str = duration_from_secs)
    )]
    #[serde(default = "Option::default", deserialize_with = "de_duration_secs")]
    pub scrape_timeout_secs: Option<Duration>,

    /// Address on which the server will listen
    #[structopt(name = "service_address", long = "service.address", alias = "address")]
    pub address: Option<IpAddr>,

    /// Port to which the server will bind
    #[structopt(name = "service_port", long = "service.port", alias = "port")]
    pub port: Option<u16>,

    /// Namespace prefix for all service endpoints (e.g. '/<prefix>/v1/graph')
    #[structopt(long = "service.path_prefix", parse(from_str = parse_path_prefix))]
    #[serde(default = "Option::default", deserialize_with = "de_path_prefix")]
    pub path_prefix: Option<String>,

    /// Comma-separated set of mandatory client parameters
    #[structopt(
        long = "service.mandatory_client_parameters",
        parse(from_str = parse_params_set)
    )]
    pub mandatory_client_parameters: Option<HashSet<String>>,

    /// Optional tracing endpoint
    #[structopt(name = "tracing_endpoint", long = "service.tracing_endpoint")]
    pub tracing_endpoint: Option<String>,
}

/// Options for the Docker-registry-v2 fetcher.
#[derive(Debug, Deserialize, Serialize, StructOpt)]
pub struct DockerRegistryOptions {
    /// URL for the container image registry
    #[structopt(long = "upstream.registry.url", alias = "registry")]
    pub url: Option<String>,

    /// Name of the container image repository
    #[structopt(long = "upstream.registry.repository", alias = "repository")]
    pub repository: Option<String>,

    /// Credentials file (in "dockercfg" format) for authentication against the image registry
    #[structopt(
        long = "upstream.registry.credentials_path",
        alias = "credentials-file"
    )]
    pub credentials_path: Option<PathBuf>,

    /// Metadata key where to record the manifest-reference
    #[structopt(long = "upstream.registry.manifestref_key")]
    pub manifestref_key: Option<String>,

    /// Concurrency for graph fetching
    #[structopt(long = "upstream.registry.fetch_concurrency")]
    pub fetch_concurrency: Option<usize>,
}

impl MergeOptions<Option<ServiceOptions>> for AppSettings {
    fn try_merge(&mut self, opts: Option<ServiceOptions>) -> Fallible<()> {
        if let Some(service) = opts {
            assign_if_some!(self.pause_secs, service.pause_secs);
            assign_if_some!(self.scrape_timeout_secs, service.scrape_timeout_secs);
            assign_if_some!(self.address, service.address);
            assign_if_some!(self.port, service.port);
            assign_if_some!(self.path_prefix, service.path_prefix);
            assign_if_some!(self.tracing_endpoint, service.tracing_endpoint);
            if let Some(params) = service.mandatory_client_parameters {
                self.mandatory_client_parameters.extend(params);
            }
        }
        Ok(())
    }
}

impl MergeOptions<Option<StatusOptions>> for AppSettings {
    fn try_merge(&mut self, opts: Option<StatusOptions>) -> Fallible<()> {
        if let Some(status) = opts {
            assign_if_some!(self.status_address, status.address);
            assign_if_some!(self.status_port, status.port);
        }
        Ok(())
    }
}

impl MergeOptions<Option<DockerRegistryOptions>> for AppSettings {
    fn try_merge(&mut self, opts: Option<DockerRegistryOptions>) -> Fallible<()> {
        if let Some(registry) = opts {
            assign_if_some!(self.registry, registry.url);
            assign_if_some!(self.repository, registry.repository);
            assign_if_some!(self.credentials_path, registry.credentials_path);
            assign_if_some!(self.manifestref_key, registry.manifestref_key);
            assign_if_some!(self.fetch_concurrency, registry.fetch_concurrency);
        }
        Ok(())
    }
}

pub fn de_duration_secs<'de, D>(deserializer: D) -> Result<Option<std::time::Duration>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    use serde::Deserialize;
    let secs = u64::deserialize(deserializer)?;
    Ok(Some(Duration::from_secs(secs)))
}

pub fn duration_from_secs<S>(num: S) -> Fallible<Duration>
where
    S: AsRef<str>,
{
    let secs: u64 = num.as_ref().parse()?;
    Ok(Duration::from_secs(secs))
}
