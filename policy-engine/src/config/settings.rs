//! Application settings for policy-engine.

use super::{cli, file};
use cincinnati::plugins::{BoxedPlugin, PluginSettings};
use failure::Fallible;
use hyper::Uri;
use std::collections::HashSet;
use std::net::{IpAddr, Ipv4Addr};
use structopt::StructOpt;

/// Default URL to upstream graph provider.
pub static DEFAULT_UPSTREAM_URL: &str = "http://localhost:8080/v1/graph";

/// Runtime application settings (validated config).
#[derive(Debug, SmartDefault)]
pub struct AppSettings {
    /// Global log level.
    #[default(log::LevelFilter::Warn)]
    pub verbosity: log::LevelFilter,

    /// URL for the upstream graph builder or policy engine
    #[default(Uri::from_static(DEFAULT_UPSTREAM_URL))]
    pub upstream: Uri,

    /// Listening address for the main service.
    #[default(IpAddr::V4(Ipv4Addr::LOCALHOST))]
    pub address: IpAddr,

    /// Listening port for the main service.
    #[default(8081)]
    pub port: u16,

    /// Listening address for the status service.
    #[default(IpAddr::V4(Ipv4Addr::LOCALHOST))]
    pub status_address: IpAddr,

    /// Listening port for the status service.
    #[default(9081)]
    pub status_port: u16,

    /// Endpoints namespace for the main service.
    pub path_prefix: String,

    /// Policy plugins configuration.
    pub policies: Vec<Box<PluginSettings>>,

    /// Required client parameters for the main service.
    pub mandatory_client_parameters: HashSet<String>,
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

    /// Validate and return policy plugins.
    pub fn policy_plugins(&self) -> Fallible<Vec<BoxedPlugin>> {
        let mut plugins = Vec::with_capacity(self.policies.len());
        for conf in &self.policies {
            let plugin = conf.build_plugin()?;
            plugins.push(plugin);
        }

        // TODO(lucab): drop this as soon as all config-maps are in place (prod & staging).
        if plugins.is_empty() {
            plugins = default_openshift_plugins();
        }

        Ok(plugins)
    }

    /// Validate and build runtime settings.
    fn try_validate(self) -> Fallible<Self> {
        if self.address == self.status_address && self.port == self.status_port {
            bail!("main and status service configured with the same address and port");
        }

        Ok(self)
    }
}

fn default_openshift_plugins() -> Vec<BoxedPlugin> {
    // TODO(lucab): drop this as soon as all config-maps are in place (prod & staging).
    use cincinnati::plugins::internal::channel_filter::ChannelFilterPlugin;
    use cincinnati::plugins::internal::metadata_fetch_quay::DEFAULT_QUAY_LABEL_FILTER;
    use cincinnati::plugins::prelude::*;

    new_plugins!(InternalPluginWrapper(ChannelFilterPlugin {
        key_prefix: String::from(DEFAULT_QUAY_LABEL_FILTER),
        key_suffix: String::from("release.channels"),
    }))
}
