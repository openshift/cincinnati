use super::registry;

use crate as cincinnati;

use self::cincinnati::plugins::prelude::*;
use self::cincinnati::plugins::prelude_plugin_impl::*;

use std::convert::TryInto;

/// Default registry to scrape.
pub static DEFAULT_SCRAPE_REGISTRY: &str = "quay.io";

/// Default repository to scrape.
pub static DEFAULT_SCRAPE_REPOSITORY: &str = "openshift-release-dev/ocp-release";

/// Default key for storing and retrieving the manifest reference from the metadata.
pub static DEFAULT_MANIFESTREF_KEY: &str = "io.openshift.upgrades.graph.release.manifestref";

/// Default fetch concurrency.
pub static DEFAULT_FETCH_CONCURRENCY: usize = 16;

/// Plugin settings.
#[derive(Clone, Debug, Deserialize, SmartDefault)]
#[serde(default)]
pub struct ReleaseScrapeDockerv2Settings {
    #[default(DEFAULT_SCRAPE_REGISTRY.to_string())]
    pub registry: String,

    #[default(DEFAULT_SCRAPE_REPOSITORY.to_string())]
    pub repository: String,

    /// Metadata key where to record the manifest-reference.
    #[default(DEFAULT_MANIFESTREF_KEY.to_string())]
    pub manifestref_key: String,

    #[default(DEFAULT_FETCH_CONCURRENCY)]
    pub fetch_concurrency: usize,

    /// Username for authenticating with the registry
    #[default(Option::None)]
    pub username: Option<String>,

    /// Password for authenticating with the registry
    #[default(Option::None)]
    pub password: Option<String>,

    /// File containing the credentials for authenticating with the registry.
    /// Takes precedence over username and password
    #[default(Option::None)]
    pub credentials_path: Option<PathBuf>,
}

impl PluginSettings for ReleaseScrapeDockerv2Settings {
    fn build_plugin(&self, registry: Option<&prometheus::Registry>) -> Fallible<BoxedPlugin> {
        let plugin = ReleaseScrapeDockerv2Plugin::try_new(self.clone(), None, registry)?;
        Ok(new_plugin!(InternalPluginWrapper(plugin)))
    }
}

impl ReleaseScrapeDockerv2Settings {
    /// Validate plugin configuration and fill in defaults.
    pub fn deserialize_config(cfg: toml::Value) -> Fallible<Box<dyn PluginSettings>> {
        let mut settings: Self = cfg.try_into()?;

        ensure!(!settings.repository.is_empty(), "empty repository");
        ensure!(!settings.registry.is_empty(), "empty registry");
        ensure!(
            !settings.manifestref_key.is_empty(),
            "empty manifestref_key prefix"
        );
        if let Some(credentials_path) = &settings.credentials_path {
            if credentials_path == &std::path::PathBuf::from("") {
                warn!("Settings contain an empty credentials path, setting to None");
                settings.credentials_path = None;
            }
        }

        Ok(Box::new(settings))
    }
}

/// Metadata fetcher for quay.io API.
#[derive(CustomDebug)]
pub struct ReleaseScrapeDockerv2Plugin {
    settings: ReleaseScrapeDockerv2Settings,
    registry: registry::Registry,
    cache: registry::cache::Cache,

    #[debug(skip)]
    graph_upstream_raw_releases: prometheus::IntGauge,
}

impl ReleaseScrapeDockerv2Plugin {
    /// Plugin name, for configuration.
    pub const PLUGIN_NAME: &'static str = "release-scrape-dockerv2";

    pub fn try_new(
        mut settings: ReleaseScrapeDockerv2Settings,
        cache: Option<registry::cache::Cache>,
        prometheus_registry: Option<&prometheus::Registry>,
    ) -> Fallible<Self> {
        use prometheus::IntGauge;
        let graph_upstream_raw_releases: IntGauge = IntGauge::new(
            "graph_upstream_raw_releases",
            "Number of releases fetched from upstream, before processing",
        )?;

        if let Some(prometheus_registry) = &prometheus_registry {
            prometheus_registry.register(Box::new(graph_upstream_raw_releases.clone()))?;
        }

        let registry = registry::Registry::try_from_str(&settings.registry)
            .context(format!("Parsing {} as Registry", &settings.registry))?;

        if let Some(credentials_path) = &settings.credentials_path {
            let (username, password) =
                registry::read_credentials(Some(&credentials_path), &registry.host_port_string())
                    .context(format!(
                    "Reading registry credentials from {:?}",
                    credentials_path
                ))?;

            settings.username = username;
            settings.password = password;
        }

        Ok(Self {
            settings,
            registry,
            cache: cache.unwrap_or_else(registry::cache::new),
            graph_upstream_raw_releases,
        })
    }
}

#[async_trait]
impl InternalPlugin for ReleaseScrapeDockerv2Plugin {
    const PLUGIN_NAME: &'static str = Self::PLUGIN_NAME;

    async fn run_internal(self: &Self, io: InternalIO) -> Fallible<InternalIO> {
        let releases = registry::fetch_releases(
            &self.registry,
            &self.settings.repository,
            self.settings.username.as_ref().map(String::as_ref),
            self.settings.password.as_ref().map(String::as_ref),
            self.cache.clone(),
            &self.settings.manifestref_key,
            self.settings.fetch_concurrency,
        )
        .await
        .context("failed to fetch all release metadata")?;

        if releases.is_empty() {
            warn!(
                "could not find any releases in {}/{}",
                &self.registry.host_port_string(),
                &self.settings.repository
            );
        };

        self.graph_upstream_raw_releases
            .set(releases.len().try_into()?);

        let graph = cincinnati::plugins::internal::graph_builder::release::create_graph(releases)?;

        Ok(InternalIO {
            graph,
            parameters: io.parameters,
        })
    }
}

#[cfg(test)]
#[cfg(feature = "test-net")]
mod network_tests;
