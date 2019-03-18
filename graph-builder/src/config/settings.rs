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
    /// Listening address for the status service.
    pub status_address: IpAddr,
    /// Listening port for the status service.
    pub status_port: u16,
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
        let service_address = IpAddr::from_str(&cfg.address)?;
        let status_address = IpAddr::from_str(&cfg.status_address)?;

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
        // TODO(lucab): drop default plugins after migration.
        compat_hardcoded_plugins(&mut plugins);
        for plugin_cfg in &mut plugins {
            plugins::sanitize_config(plugin_cfg)?;
        }

        let opts = Self {
            address: service_address,
            credentials_path,
            mandatory_client_parameters: cfg.mandatory_client_parameters,
            manifestref_key: cfg.manifestref_key,
            path_prefix: cfg.path_prefix,
            period,
            plugins,
            port: cfg.port,
            registry: cfg.registry_url,
            repository: cfg.repository,
            status_address,
            status_port: cfg.status_port,
            verbosity,
        };
        Ok(opts)
    }
}

// Fill-in hardcoded default plugins for retro-compatibility.
fn compat_hardcoded_plugins(plugins: &mut Vec<HashMap<String, String>>) {
    if !plugins.is_empty() {
        return;
    }

    let mut quay_meta_fetch = HashMap::new();
    quay_meta_fetch.insert("name".to_string(), "quay-metadata".to_string());
    quay_meta_fetch.insert(
        "repository".to_string(),
        "openshift-release-dev/ocp-release".to_string(),
    );
    plugins.push(quay_meta_fetch);

    let mut node_remove = HashMap::new();
    node_remove.insert("name".to_string(), "node-remove".to_string());
    plugins.push(node_remove);

    let mut edge_add_remove = HashMap::new();
    edge_add_remove.insert("name".to_string(), "edge-add-remove".to_string());
    plugins.push(edge_add_remove);
}
