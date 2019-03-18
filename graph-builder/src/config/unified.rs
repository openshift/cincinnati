//! Unified configuration.

use super::cli::CliOptions;
use super::file::FileOptions;
use failure::Fallible;
use std::collections::{HashMap, HashSet};

static DEFAULT_VERBOSITY: u8 = 0;

static DEFAULT_SERV_ADDR: &str = "127.0.0.1";
static DEFAULT_SERV_PORT: u16 = 8080;
static DEFAULT_SERV_PREFIX: &str = "";

static DEFAULT_REGISTRY_PERIOD: u64 = 30;
static DEFAULT_REGISTRY_CREDENTIALS_PATH: &str = "";
static DEFAULT_REGISTRY_MANIFESTREF_KEY: &str = "com.openshift.upgrades.graph.release.manifestref";
static DEFAULT_REGISTRY_URL: &str = "http://localhost:5000";
static DEFAULT_REGISTRY_REPO: &str = "openshift";

static DEFAULT_STATUS_ADDR: &str = "127.0.0.1";
static DEFAULT_STATUS_PORT: u16 = 9080;

macro_rules! maybe_assign {
    ( $dst:expr, $src:expr ) => {{
        if let Some(x) = $src {
            $dst = x;
        };
    }};
}

/// Top-level configuration (before semantic validation).
#[derive(Debug)]
pub struct UnifiedConfig {
    pub address: String,
    pub credentials_path: String,
    pub mandatory_client_parameters: HashSet<String>,
    pub manifestref_key: String,
    pub path_prefix: String,
    pub period: u64,
    pub plugins: Vec<HashMap<String, String>>,
    pub port: u16,
    pub registry_url: String,
    pub repository: String,
    pub status_address: String,
    pub status_port: u16,
    pub verbosity: u8,
}

impl Default for UnifiedConfig {
    fn default() -> Self {
        Self {
            address: DEFAULT_SERV_ADDR.to_string(),
            credentials_path: DEFAULT_REGISTRY_CREDENTIALS_PATH.to_string(),
            mandatory_client_parameters: vec![].into_iter().collect(),
            manifestref_key: DEFAULT_REGISTRY_MANIFESTREF_KEY.to_string(),
            path_prefix: DEFAULT_SERV_PREFIX.to_string(),
            period: DEFAULT_REGISTRY_PERIOD,
            plugins: vec![],
            port: DEFAULT_SERV_PORT,
            registry_url: DEFAULT_REGISTRY_URL.to_string(),
            repository: DEFAULT_REGISTRY_REPO.to_string(),
            status_address: DEFAULT_STATUS_ADDR.to_string(),
            status_port: DEFAULT_STATUS_PORT,
            verbosity: DEFAULT_VERBOSITY,
        }
    }
}

impl UnifiedConfig {
    /// Merge command-line options into unified configuration.
    pub(crate) fn merge_cli_config(self, cfg: CliOptions) -> Fallible<Self> {
        let mut merged_cfg = self;

        // Top-level options.
        if cfg.verbosity > 0 {
            merged_cfg.verbosity = cfg.verbosity;
        }

        // Main service options.
        maybe_assign!(merged_cfg.address, cfg.service.address);
        maybe_assign!(merged_cfg.port, cfg.service.port);
        maybe_assign!(merged_cfg.path_prefix, cfg.service.path_prefix);
        if let Some(params) = cfg.service.mandatory_client_parameters {
            merged_cfg.mandatory_client_parameters = commons::parse_params_set(&params);
        }

        // Status service options.
        maybe_assign!(merged_cfg.status_address, cfg.status.address);
        maybe_assign!(merged_cfg.status_port, cfg.status.port);

        // Registry upstream scraper options.
        maybe_assign!(merged_cfg.period, cfg.registry.period);
        maybe_assign!(merged_cfg.registry_url, cfg.registry.url);
        maybe_assign!(merged_cfg.repository, cfg.registry.repository);
        maybe_assign!(merged_cfg.credentials_path, cfg.registry.credentials_path);
        maybe_assign!(merged_cfg.manifestref_key, cfg.registry.manifestref_key);

        Ok(merged_cfg)
    }

    /// Merge TOML options into unified configuration.
    pub(crate) fn merge_file_config(self, file_cfg: Option<FileOptions>) -> Fallible<Self> {
        let mut merged_cfg = self;

        // Eaarly-return without updates if there is no configuration file.
        let cfg = match file_cfg {
            Some(c) => c,
            None => return Ok(merged_cfg),
        };

        // Top-level options.
        maybe_assign!(merged_cfg.verbosity, cfg.verbosity);

        // Main service options.
        if let Some(service) = cfg.service {
            maybe_assign!(merged_cfg.address, service.address);
            maybe_assign!(merged_cfg.port, service.port);
            maybe_assign!(merged_cfg.path_prefix, service.path_prefix);

            if let Some(params) = service.mandatory_client_parameters {
                merged_cfg.mandatory_client_parameters.extend(params);
            }
        }

        // Registry upstream scraper options.
        if let Some(upstream) = cfg.upstream {
            // TODO(lucab): drop once fedora-coreos usptream is implemented.
            if let Some(method) = upstream.method {
                ensure!(method == "registry", "unknown upstream method");
            }

            if let Some(registry) = upstream.registry {
                maybe_assign!(merged_cfg.credentials_path, registry.credentials_path);
                maybe_assign!(merged_cfg.period, registry.period);
                maybe_assign!(merged_cfg.registry_url, registry.url);
                maybe_assign!(merged_cfg.repository, registry.repository);
                maybe_assign!(merged_cfg.manifestref_key, registry.manifestref_key);
            }
        }

        // Status service options.
        if let Some(status) = cfg.status {
            maybe_assign!(merged_cfg.status_address, status.address);
            maybe_assign!(merged_cfg.status_port, status.port);
        }

        // Plugins options. Order is relevant too.
        if let Some(plugins) = cfg.plugins {
            for entry in plugins {
                merged_cfg.plugins.push(entry);
            }
        }

        Ok(merged_cfg)
    }
}
