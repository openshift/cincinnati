//! Application settings for graph-builder.

use super::{cli, file};
use cincinnati::plugins::catalog::{build_plugins, PluginSettings};
use cincinnati::plugins::BoxedPlugin;
use commons::MergeOptions;
use failure::Fallible;
use std::collections::HashSet;
use std::net::{IpAddr, Ipv4Addr};
use std::path::PathBuf;
use std::time;
use structopt::StructOpt;

/// Runtime application settings (validated config).
#[derive(Debug, SmartDefault)]
pub struct AppSettings {
    /// Listening address for the main service.
    #[default(IpAddr::V4(Ipv4Addr::LOCALHOST))]
    pub address: IpAddr,

    /// Optional auth secrets for the registry scraper.
    pub credentials_path: Option<PathBuf>,

    /// Required client parameters for the main service.
    pub mandatory_client_parameters: HashSet<String>,

    /// Metadata key where to record the manifest-reference.
    #[default("io.openshift.upgrades.graph.release.manifestref")]
    pub manifestref_key: String,

    /// Endpoints namespace for the main service.
    pub path_prefix: String,

    /// Pause (in seconds) between registry scrapes.
    #[default(time::Duration::from_secs(30))]
    pub pause_secs: time::Duration,

    /// Listening port for the main service.
    #[default(8080)]
    pub port: u16,

    // TODO(lucab): split this in (TLS, hostname+port).
    /// Target host for the registry scraper.
    #[default(cincinnati::plugins::internal::release_scrape_dockerv2::DEFAULT_SCRAPE_REGISTRY.to_string())]
    pub registry: String,

    /// Target image for the registry scraper.
    #[default(cincinnati::plugins::internal::release_scrape_dockerv2::DEFAULT_SCRAPE_REPOSITORY.to_string())]
    pub repository: String,

    /// Listening address for the status service.
    #[default(IpAddr::V4(Ipv4Addr::LOCALHOST))]
    pub status_address: IpAddr,

    /// Listening port for the status service.
    #[default(9080)]
    pub status_port: u16,

    /// Global log level.
    #[default(log::LevelFilter::Warn)]
    pub verbosity: log::LevelFilter,

    /// Concurrency for graph fetching
    #[default(cincinnati::plugins::internal::release_scrape_dockerv2::DEFAULT_FETCH_CONCURRENCY)]
    pub fetch_concurrency: usize,

    /// Metrics which are required to be registered, to be specified without the `METRICS_PREFIX`.
    /// If these are not registered by the time all plugins have been loaded an error will be thrown.
    #[default([
        "graph_upstream_raw_releases",
    ].iter().cloned().map(Into::into).collect())]
    pub metrics_required: HashSet<String>,

    /// Plugin configuration.
    pub plugin_settings: Vec<Box<dyn PluginSettings>>,
}

impl AppSettings {
    /// Lookup all optional configs, merge them with defaults, and
    /// transform into valid runtime settings.
    pub fn assemble() -> Fallible<Self> {
        // Source options.
        let cli_opts = cli::CliOptions::from_args();
        let file_opts = match &cli_opts.config_path {
            Some(ref path) => Some(file::FileOptions::read_filepath(path)?),
            None => None,
        };
        let defaults = Self::default();

        // Combine options into a single config.
        let mut cfg = defaults;
        cfg.try_merge(cli_opts)?;
        cfg.try_merge(file_opts)?;

        // Validate and convert to settings.
        Self::try_validate(cfg)
    }

    /// Validate and return configured plugins.
    pub fn validate_and_build_plugins(
        &self,
        registry: Option<&prometheus::Registry>,
    ) -> Fallible<Vec<BoxedPlugin>> {
        let default_plugin_settings = self.default_openshift_plugin_settings()?;

        let plugin_settings: &Vec<Box<dyn PluginSettings>> = if self.plugin_settings.is_empty() {
            &default_plugin_settings
        } else {
            &self.plugin_settings
        };

        build_plugins(plugin_settings, registry)
    }

    /// Validate and build runtime settings.
    fn try_validate(self) -> Fallible<Self> {
        if self.pause_secs.as_secs() == 0 {
            bail!("unexpected 0s pause");
        }

        Ok(self)
    }

    fn default_openshift_plugin_settings(
        &self,
        // registry: Option<&prometheus::Registry>,
    ) -> Fallible<Vec<Box<dyn PluginSettings>>> {
        use cincinnati::plugins::internal::edge_add_remove::DEFAULT_REMOVE_ALL_EDGES_VALUE;
        use cincinnati::plugins::prelude::*;

        let plugins = vec![
            ReleaseScrapeDockerv2Settings::deserialize_config(toml::from_str(&format!(
                r#"
                    name = "{}"
                    registry = "{}"
                    repository = "{}"
                    manifestref_key = "{}"
                    fetch_concurrency = {}
                    {}
                "#,
                ReleaseScrapeDockerv2Plugin::PLUGIN_NAME,
                &self.registry,
                &self.repository,
                &self.manifestref_key,
                self.fetch_concurrency,
                self.credentials_path
                    .as_ref()
                    .map(|pathbuf| pathbuf.to_str())
                    .flatten()
                    .map(|path| format!("\ncredentials_path = {:?}", path))
                    .unwrap_or_default()
            ))?)?,
            plugin_config!(
                ("name", QuayMetadataFetchPlugin::PLUGIN_NAME),
                ("repository", &self.repository),
                ("manifestref_key", &self.manifestref_key),
                ("api-base", quay::v1::DEFAULT_API_BASE)
            )?,
            plugin_config!(
                ("name", NodeRemovePlugin::PLUGIN_NAME,),
                ("key_prefix", &self.manifestref_key)
            )?,
            plugin_config!(
                ("name", EdgeAddRemovePlugin::PLUGIN_NAME),
                ("key_prefix", &self.manifestref_key),
                ("remove_all_edges_value", DEFAULT_REMOVE_ALL_EDGES_VALUE)
            )?,
        ];

        Ok(plugins)
    }
}
