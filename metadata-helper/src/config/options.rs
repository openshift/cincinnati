//! Options shared by CLI and TOML.

use super::AppSettings;
use commons::prelude_errors::*;
use commons::{de_path_prefix, parse_path_prefix, MergeOptions};
use std::net::IpAddr;
use std::time::Duration;

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

impl MergeOptions<Option<StatusOptions>> for AppSettings {
    fn try_merge(&mut self, opts: Option<StatusOptions>) -> Fallible<()> {
        if let Some(status) = opts {
            assign_if_some!(self.status_address, status.address);
            assign_if_some!(self.status_port, status.port);
        }
        Ok(())
    }
}

// Options for Signatures Service
#[derive(Debug, Deserialize, Serialize, StructOpt)]
pub struct SignaturesOptions {
    /// directory where the sigatures are stored
    #[structopt(name = "signatures_dir", long = "signatures.dir")]
    pub dir: Option<String>,
}

impl MergeOptions<Option<SignaturesOptions>> for AppSettings {
    fn try_merge(&mut self, opts: Option<SignaturesOptions>) -> Fallible<()> {
        if let Some(signatures) = opts {
            assign_if_some!(self.signatures_dir, signatures.dir);
        }
        Ok(())
    }
}

/// Options for the main Cincinnati service.
#[derive(Debug, Deserialize, Serialize, StructOpt)]
pub struct ServiceOptions {
    /// Address on which the server will listen
    #[structopt(name = "service_address", long = "service.address")]
    pub address: Option<IpAddr>,

    /// Port to which the server will bind
    #[structopt(name = "service_port", long = "service.port")]
    pub port: Option<u16>,

    /// Namespace prefix for all service endpoints (e.g. '/<prefix>/graph')
    #[structopt(long = "service.path_prefix", parse(from_str = parse_path_prefix))]
    #[serde(default = "Option::default", deserialize_with = "de_path_prefix")]
    pub path_prefix: Option<String>,

    /// Optional tracing endpoint
    #[structopt(name = "tracing_endpoint", long = "service.tracing_endpoint")]
    pub tracing_endpoint: Option<String>,

    #[structopt(name = "backlog", long = "service.backlog")]
    pub backlog: Option<u32>,
    #[structopt(name = "max_connections", long = "service.max_connections")]
    pub max_connections: Option<usize>,
    #[structopt(name = "max_connection_rate", long = "service.max_connection_rate")]
    pub max_connection_rate: Option<usize>,
    #[structopt(name = "keep_alive", long = "service.keep_alive")]
    pub keep_alive: Option<u64>,
    #[structopt(name = "client_timeout", long = "service.client_timeout")]
    pub client_timeout: Option<u64>,
}

impl MergeOptions<Option<ServiceOptions>> for AppSettings {
    fn try_merge(&mut self, opts: Option<ServiceOptions>) -> Fallible<()> {
        if let Some(service) = opts {
            assign_if_some!(self.address, service.address);
            assign_if_some!(self.port, service.port);
            assign_if_some!(self.path_prefix, service.path_prefix);
            assign_if_some!(self.tracing_endpoint, service.tracing_endpoint);
            assign_if_some!(self.backlog, service.backlog);
            assign_if_some!(self.max_connections, service.max_connections);
            assign_if_some!(self.max_connection_rate, service.max_connection_rate);
            self.keep_alive = match service.keep_alive {
                Some(x) => Some(Duration::new(x, 0)),
                None => None,
            };
            if let Some(duration) = service.client_timeout {
                self.client_timeout = Duration::new(duration, 0);
            }
        }
        Ok(())
    }
}
