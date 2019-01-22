//! Command-line options for policy-engine.

use commons::{parse_params_set, parse_path_prefix};
use hyper::Uri;
use std::collections::HashSet;
use std::net::IpAddr;

#[derive(Debug, StructOpt)]
pub struct Options {
    /// Verbosity level
    #[structopt(short = "v", parse(from_occurrences))]
    pub verbosity: u64,

    /// URL for the upstream graph builder or policy engine
    #[structopt(long = "upstream", default_value = "http://localhost:8080/v1/graph")]
    pub upstream: Uri,

    /// Address on which the server will listen
    #[structopt(long = "address", default_value = "127.0.0.1")]
    pub address: IpAddr,

    /// Port to which the server will bind
    #[structopt(long = "port", default_value = "8081")]
    pub port: u16,

    /// Address on which the server will serve metrics.
    #[structopt(long = "metrics_address", default_value = "127.0.0.1")]
    pub metrics_address: IpAddr,

    /// Port to which the metrics server will bind.
    #[structopt(long = "metrics_port", default_value = "9081")]
    pub metrics_port: u16,

    /// Path prefix for all paths.
    #[structopt(
        long = "path-prefix",
        default_value = "",
        parse(from_str = "parse_path_prefix")
    )]
    pub path_prefix: String,

    /// Comma-separated set of mandatory client parameters.
    #[structopt(
        long = "mandatory-client-parameters",
        default_value = "",
        parse(from_str = "parse_params_set")
    )]
    pub mandatory_client_parameters: HashSet<String>,
}
