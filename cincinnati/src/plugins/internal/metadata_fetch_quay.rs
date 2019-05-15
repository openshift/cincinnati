//! This plugin implements the fetching of dynamic metadata from quay.io.
//!
//! The fetch process is all or nohting, i.e. it fails in these cases:
//! * a Release doesn't contain the manifestref in its metadata
//! * the dynamic metadata can't be fetched for a single manifestref

extern crate futures;
extern crate quay;
extern crate tokio;

use self::futures::future::Future;
use failure::{Fallible, ResultExt};
use crate::plugins::{
    InternalIO, InternalPlugin, InternalPluginWrapper, Plugin, PluginIO, PluginSettings,
};
use std::path::PathBuf;
use crate::ReleaseId;

pub static DEFAULT_QUAY_LABEL_FILTER: &str = "io.openshift.upgrades.graph";
pub static DEFAULT_QUAY_MANIFESTREF_KEY: &str = "io.openshift.upgrades.graph.release.manifestref";
pub static DEFAULT_QUAY_REPOSITORY: &str = "openshift";

/// Plugin settings.
#[derive(Clone, Debug, Deserialize, SmartDefault)]
#[serde(default)]
struct QuayMetadataSettings {
    #[default(quay::v1::DEFAULT_API_BASE.to_string())]
    api_base: String,

    #[default(Option::None)]
    api_credentials_path: Option<PathBuf>,

    #[default(DEFAULT_QUAY_REPOSITORY.to_string())]
    repository: String,

    #[default(DEFAULT_QUAY_LABEL_FILTER.to_string())]
    label_filter: String,

    #[default(DEFAULT_QUAY_MANIFESTREF_KEY.to_string())]
    manifestref_key: String,
}

/// Metadata fetcher for quay.io API.
pub struct QuayMetadataFetchPlugin {
    client: quay::v1::Client,
    repo: String,
    label_filter: String,
    manifestref_key: String,
}

impl PluginSettings for QuayMetadataSettings {
    fn build_plugin(&self) -> Fallible<Box<Plugin<PluginIO>>> {
        let cfg = self.clone();
        let plugin = QuayMetadataFetchPlugin::try_new(
            cfg.repository,
            cfg.label_filter,
            cfg.manifestref_key,
            cfg.api_credentials_path,
            cfg.api_base,
        )?;
        Ok(Box::new(InternalPluginWrapper(plugin)))
    }
}

impl QuayMetadataFetchPlugin {
    /// Plugin name, for configuration.
    pub(crate) const PLUGIN_NAME: &'static str = "quay-metadata";

    /// Validate plugin configuration and fill in defaults.
    pub fn deserialize_config(cfg: toml::Value) -> Fallible<Box<PluginSettings>> {
        let settings: QuayMetadataSettings = cfg.try_into()?;

        ensure!(!settings.repository.is_empty(), "empty repository");
        ensure!(!settings.label_filter.is_empty(), "empty label_filter");

        Ok(Box::new(settings))
    }

    pub fn try_new(
        repo: String,
        label_filter: String,
        manifestref_key: String,
        api_token_path: Option<PathBuf>,
        api_base: String,
    ) -> Fallible<Self> {
        let api_token = api_token_path
            .map(quay::read_credentials)
            .transpose()
            .context("could not read quay API credentials")?;

        let client: quay::v1::Client = quay::v1::Client::builder()
            .access_token(api_token)
            .api_base(Some(api_base.to_string()))
            .build()?;

        Ok(Self {
            client,
            repo,
            label_filter,
            manifestref_key,
        })
    }
}

impl InternalPlugin for QuayMetadataFetchPlugin {
    fn run_internal(&self, io: InternalIO) -> Fallible<InternalIO> {
        let (mut graph, parameters) = (io.graph, io.parameters);

        trace!("fetching metadata from quay labels...");

        let release_manifestrefs: Vec<(ReleaseId, String, String)> =
            graph.find_by_metadata_key(&self.manifestref_key);

        if release_manifestrefs.is_empty() {
            warn!(
                "no release has a manifestref at metadata key '{}'",
                self.manifestref_key
            );
        }

        release_manifestrefs.into_iter().try_for_each(
            |(release_id, release_version, manifestref): (ReleaseId, String, String)| -> Fallible<()> {
                let mut rt = self::tokio::runtime::current_thread::Runtime::new()
                    .context("could not create a Runtime")?;
                let quay_labels: Vec<(String, String)> = rt
                    .block_on(
                        self.client
                            .get_labels(
                                self.repo.clone(),
                                manifestref.clone(),
                                Some(self.label_filter.clone()),
                            )
                            .map(|labels| labels.into_iter().map(Into::into).collect()),
                    )
                    .context(format!(
                        "[{}] could not fetch quay labels",
                        &release_version
                    ))?;

                info!(
                    "[{}] received {} label(s)",
                    &release_version,
                    &quay_labels.len()
                );

                let metadata = graph.get_metadata_as_ref_mut(&release_id)?;

                for (key, value) in quay_labels {
                    let warn_msg = if metadata.contains_key(&key) {
                        Some(format!(
                            "[{}] key '{}' already exists. overwriting with value '{}'. ",
                            &release_version, &key, &value
                        ))
                    } else {
                        None
                    };

                    trace!(
                        "[{}] inserting ('{}', '{}')",
                        &release_version,
                        &key,
                        &value
                    );

                    if let Some(previous_value) = metadata.insert(key, value) {
                        warn!(
                            "{}previous value: '{}'",
                            warn_msg.unwrap_or_default(),
                            previous_value
                        );
                    };
                }

                Ok(())
            },
        )?;

        Ok(InternalIO { graph, parameters })
    }
}
