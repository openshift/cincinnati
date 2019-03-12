use super::{cli, file, unified};
use cincinnati::plugins;
use failure::Fallible;
use std::collections::{HashMap, HashSet};
use std::net::IpAddr;
use std::path::PathBuf;
use std::str::FromStr;

/// Runtime application settings (validated config).
#[derive(Debug)]
pub struct AppSettings {
    /// Listening address for the main service.
    pub address: IpAddr,
    /// Optional auth secrets for the registry scraper.
    pub credentials_path: Option<PathBuf>,
    /// Required client parameters for the main service.
    pub mandatory_client_parameters: HashSet<String>,
    /// Metadata key where to record the manifest-reference.
    pub manifestref_key: String,
    /// Endpoints namespace for the main service.
    pub path_prefix: String,
    /// Polling period for the registry scraper.
    pub period: std::time::Duration,
    /// Plugins configuration.
    pub plugins: Vec<HashMap<String, String>>,
    /// Listening port for the main service.
    pub port: u16,
    // TODO(lucab): split this in (TLS, hostname+port).
    /// Target host for the registry scraper.
    pub registry: String,
    /// Target image for the registry scraper.
    pub repository: String,
    /// Global log level.
    pub verbosity: log::LevelFilter,
}

impl AppSettings {
    /// Lookup all optional configs, merge them with defaults, and
    /// transform into valid runtime settings.
    pub fn assemble() -> Fallible<Self> {
        use structopt::StructOpt;

        let default_opts = unified::UnifiedConfig::default();
        let cli_opts = cli::CliOptions::from_args();

        let file_opts = match &cli_opts.config_path {
            Some(ref path) => Some(file::FileOptions::read_filepath(path)?),
            None => None,
        };

        let merged_config = default_opts
            .merge_file_config(file_opts)?
            .merge_cli_config(cli_opts)?;

        Self::validate_config(merged_config)
    }

    /// Try to validate configuration and build application settings from it.
    fn validate_config(cfg: unified::UnifiedConfig) -> Fallible<Self> {
        let address = IpAddr::from_str(&cfg.address)?;

        let credentials_path = match cfg.credentials_path.as_str() {
            "" => None,
            s => Some(PathBuf::from(s)),
        };

        let verbosity = match cfg.verbosity {
            0 => log::LevelFilter::Warn,
            1 => log::LevelFilter::Info,
            2 => log::LevelFilter::Debug,
            _ => log::LevelFilter::Trace,
        };

        let period = match cfg.period {
            0 => bail!("unexpected 0s refresh period"),
            period => std::time::Duration::from_secs(period),
        };

        let mut plugins = cfg.plugins;
        for plugin_cfg in &mut plugins {
            plugins::sanitize_config(plugin_cfg)?;
        }

        let opts = Self {
            address,
            credentials_path,
            mandatory_client_parameters: cfg.mandatory_client_parameters,
            manifestref_key: cfg.manifestref_key,
            path_prefix: cfg.path_prefix,
            period,
            plugins,
            port: cfg.port,
            registry: cfg.registry_url,
            repository: cfg.repository,
            verbosity,
        };
        Ok(opts)
    }
}
