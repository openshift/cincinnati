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
use plugins::{InternalIO, InternalPlugin, InternalPluginWrapper, Plugin, PluginIO};
use std::collections::HashMap;
use toml::value::Value;
use ReleaseId;

pub static DEFAULT_QUAY_LABEL_FILTER: &str = "com.openshift.upgrades.graph";
pub static DEFAULT_QUAY_MANIFESTREF_KEY: &str = "com.openshift.upgrades.graph.release.manifestref";
pub static DEFAULT_QUAY_REPOSITORY: &str = "openshift";

pub struct QuayMetadataFetchPlugin {
    client: quay::v1::Client,
    repo: String,
    label_filter: String,
    manifestref_key: String,
}

/// Plugin settings.
#[derive(Deserialize)]
struct PluginSettings {
    api_base: String,
    api_credentials_path: Option<String>,
    repository: String,
    label_filter: String,
    manifestref_key: String,
}

impl QuayMetadataFetchPlugin {
    /// Plugin name, for configuration.
    pub(crate) const PLUGIN_NAME: &'static str = "quay-metadata";

    /// Validate plugin configuration and fill in defaults.
    pub fn sanitize_config(cfg: &mut HashMap<String, String>) -> Fallible<()> {
        let name = cfg.get("name").cloned().unwrap_or_default();
        ensure!(name == Self::PLUGIN_NAME, "unexpected plugin name");

        cfg.entry("api_base".to_string())
            .or_insert_with(|| quay::v1::DEFAULT_API_BASE.to_string());
        cfg.entry("repository".to_string())
            .or_insert_with(|| DEFAULT_QUAY_REPOSITORY.to_string());
        cfg.entry("label_filter".to_string())
            .or_insert_with(|| DEFAULT_QUAY_LABEL_FILTER.to_string());
        cfg.entry("manifestref_key".to_string())
            .or_insert_with(|| DEFAULT_QUAY_MANIFESTREF_KEY.to_string());
        // TODO(lucab): perform validation.

        Ok(())
    }

    /// Try to build a plugin from settings.
    pub fn from_settings(cfg: &HashMap<String, String>) -> Fallible<Box<Plugin<PluginIO>>> {
        let cfg: PluginSettings = Value::try_from(cfg)?.try_into()?;

        let plugin = Self::try_new(
            cfg.repository,
            cfg.label_filter,
            cfg.manifestref_key,
            cfg.api_credentials_path,
            cfg.api_base,
        )?;
        Ok(Box::new(InternalPluginWrapper(plugin)))
    }

    fn try_new(
        repo: String,
        label_filter: String,
        manifestref_key: String,
        api_token_path: Option<String>,
        api_base: String,
    ) -> Fallible<Self> {
        let api_token = match api_token_path {
            Some(p) => {
                let token =
                    quay::read_credentials(p).context("could not read quay API credentials")?;
                Some(token)
            }
            None => None,
        };

        let client: quay::v1::Client = quay::v1::Client::builder()
            .access_token(api_token)
            .api_base(Some(api_base))
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
