//! Application settings for metadata-helper.

use super::{cli, file};
use commons::prelude_errors::*;
use custom_debug_derive::Debug as CustomDebug;
use std::net::{IpAddr, Ipv4Addr};
use std::time::Duration;
use structopt::StructOpt;


/// Runtime application settings (validated config).
#[derive(CustomDebug, SmartDefault)]
pub struct AppSettings {
    /// Global log level.
    #[default(log::LevelFilter::Warn)]
    pub verbosity: log::LevelFilter,

    /// Listening address for the main service.
    #[default(IpAddr::V4(Ipv4Addr::LOCALHOST))]
    pub address: IpAddr,

    /// Listening port for the main service.
    #[default(9081)]
    pub port: u16,

    /// Listening address for the status service.
    #[default(IpAddr::V4(Ipv4Addr::LOCALHOST))]
    pub status_address: IpAddr,

    /// Listening port for the status service.
    #[default(9091)]
    pub status_port: u16,

    /// Endpoints namespace for the main service.
    pub path_prefix: String,

    /// Jaeger host and port for tracing support
    pub tracing_endpoint: Option<String>,

    /// Actix-web maximum number of pending connections, defaults to 2048: https://docs.rs/actix-web/latest/actix_web/struct.HttpServer.html#method.backlog
    #[default(10)]
    pub backlog: u32,
    /// Actix-web per-worker number of concurrent connections, defaults to 25000: https://docs.rs/actix-web/latest/actix_web/struct.HttpServer.html#method.max_connections
    #[default(10)]
    pub max_connections: usize,
    /// Actix-web maximum per-worker concurrent connection establish process, defaults to 256: https://docs.rs/actix-web/latest/actix_web/struct.HttpServer.html#method.max_connections
    #[default(64)]
    pub max_connection_rate: usize,
    /// Actix-web server keepalive, defaults to 5s: https://docs.rs/actix-web/latest/actix_web/struct.HttpServer.html#method.keep_alive
    #[default(None)]
    pub keep_alive: Option<Duration>,
    /// Actix-web server client timeout for first request, defaults to 5s: https://docs.rs/actix-web/latest/actix_web/struct.HttpServer.html#method.client_timeout
    #[default(Duration::new(5, 0))]
    pub client_timeout: Duration,
}

impl AppSettings {
    /// Lookup all optional configs, merge them with defaults, and
    /// transform into valid runtime settings.
    pub fn assemble() -> Fallible<Self> {
        use commons::MergeOptions;

        let defaults = Self::default();

        // Source options.
        let cli_opts = cli::CliOptions::from_args();
        let file_opts = match &cli_opts.config_path {
            Some(ref path) => Some(file::FileOptions::read_filepath(path)?),
            None => None,
        };

        // Combine options into a single config.
        let mut cfg = defaults;
        cfg.try_merge(cli_opts)?;
        cfg.try_merge(file_opts)?;

        // Validate and convert to settings.
        Self::try_validate(cfg)
    }

    /// Validate and build runtime settings.
    fn try_validate(self) -> Fallible<Self> {
        if self.address == self.status_address && self.port == self.status_port {
            bail!("main and status service configured with the same address and port");
        }

        Ok(self)
    }

}
