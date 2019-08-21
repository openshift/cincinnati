//! This plugin implements the fetching of dynamic metadata from quay.io.
//!
//! The fetch process is all or nohting, i.e. it fails in these cases:
//! * a Release doesn't contain the manifestref in its metadata
//! * the dynamic metadata can't be fetched for a single manifestref

extern crate futures;
extern crate quay;
extern crate tokio;

use crate::plugins::{InternalIO, InternalPlugin, InternalPluginWrapper, PluginSettings};
use crate::ReleaseId;
use failure::{Error, Fallible, ResultExt};
use futures::future::Future;
use plugins::AsyncIO;
use plugins::BoxedPlugin;
use std::path::PathBuf;

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
#[derive(Debug)]
pub struct QuayMetadataFetchPlugin {
    client: quay::v1::Client,
    repo: String,
    label_filter: String,
    manifestref_key: String,
}

impl PluginSettings for QuayMetadataSettings {
    fn build_plugin(&self) -> Fallible<BoxedPlugin> {
        let cfg = self.clone();
        let plugin = QuayMetadataFetchPlugin::try_new(
            cfg.repository,
            cfg.label_filter,
            cfg.manifestref_key,
            cfg.api_credentials_path,
            cfg.api_base,
        )?;
        Ok(new_plugin!(InternalPluginWrapper(plugin)))
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
    fn run_internal(self: &Self, io: InternalIO) -> AsyncIO<InternalIO> {
        let (mut graph, parameters) = (io.graph, io.parameters);

        trace!("fetching metadata from quay labels...");

        let release_manifestrefs: Vec<(ReleaseId, String, String)> =
            graph.find_by_metadata_key(&self.manifestref_key);

        if release_manifestrefs.is_empty() {
            warn!(
                "no release has a manifestref at metadata key '{}'",
                &self.manifestref_key
            );
        }

        let future_all_labels = release_manifestrefs
            .into_iter()
            .map(|(release_id, release_version, manifestref)| {
                let (client, repo, label_filter) = (
                    self.client.clone(),
                    self.repo.clone(),
                    self.label_filter.clone(),
                );

                client
                    .get_labels(
                        repo.clone(),
                        manifestref.clone(),
                        Some(label_filter.clone()),
                    )
                    .and_then(|quay_labels| {
                        let labels = quay_labels
                            .into_iter()
                            .map(Into::into)
                            .collect::<Vec<(String, String)>>();
                        Ok((labels, (release_id, release_version)))
                    })
                    .map_err(Error::from)
            })
            .collect::<Vec<_>>();

        let future_finalio =
            futures::future::join_all(future_all_labels).and_then(|labels_with_releaseinfo| {
                for (labels, (release_id, release_version)) in labels_with_releaseinfo {
                    let metadata = graph
                        .get_metadata_as_ref_mut(&release_id)
                        .context("trying to find metadata for release")?;
                    for (key, value) in labels {
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
                }

                Ok(InternalIO { graph, parameters })
            });

        Box::new(future_finalio)
    }
}

#[cfg(test)]
#[cfg(feature = "test-net")]
mod tests_net {
    use super::*;
    use commons::testing::init_runtime;
    use std::collections::HashMap;

    fn input_metadata_labels_test_annoated(
        manifestrefs: HashMap<usize, &str>,
    ) -> HashMap<usize, HashMap<String, String>> {
        metadata_labels_test_annoated(manifestrefs, true)
    }

    fn expected_metadata_labels_test_annoated(
        manifestrefs: HashMap<usize, &str>,
    ) -> HashMap<usize, HashMap<String, String>> {
        metadata_labels_test_annoated(manifestrefs, false)
    }

    fn metadata_labels_test_annoated(
        manifestrefs: HashMap<usize, &str>,
        input: bool,
    ) -> HashMap<usize, HashMap<String, String>> {
        [
            (0, HashMap::new()),
            (
                1,
                if input {
                    vec![(
                        String::from(DEFAULT_QUAY_MANIFESTREF_KEY),
                        manifestrefs
                            .get(&1)
                            .expect("expected manifestref")
                            .to_string(),
                    )]
                } else {
                    vec![
                        (
                            String::from(DEFAULT_QUAY_MANIFESTREF_KEY),
                            manifestrefs
                                .get(&1)
                                .expect("expected manifestref")
                                .to_string(),
                        ),
                        (
                            String::from("io.openshift.upgrades.graph.previous.remove"),
                            String::from("0.0.0"),
                        ),
                    ]
                }
                .iter()
                .cloned()
                .collect(),
            ),
            (
                2,
                if input {
                    vec![(
                        String::from(DEFAULT_QUAY_MANIFESTREF_KEY),
                        manifestrefs
                            .get(&2)
                            .expect("expected manifestref")
                            .to_string(),
                    )]
                } else {
                    vec![
                        (
                            String::from(DEFAULT_QUAY_MANIFESTREF_KEY),
                            manifestrefs
                                .get(&2)
                                .expect("expected manifestref")
                                .to_string(),
                        ),
                        (
                            String::from("io.openshift.upgrades.graph.release.remove"),
                            String::from("true"),
                        ),
                    ]
                }
                .iter()
                .cloned()
                .collect(),
            ),
            (
                3,
                if input {
                    vec![(
                        String::from(DEFAULT_QUAY_MANIFESTREF_KEY),
                        manifestrefs
                            .get(&3)
                            .expect("expected manifestref")
                            .to_string(),
                    )]
                } else {
                    vec![
                        (
                            String::from(DEFAULT_QUAY_MANIFESTREF_KEY),
                            manifestrefs
                                .get(&3)
                                .expect("expected manifestref")
                                .to_string(),
                        ),
                        (
                            String::from("io.openshift.upgrades.graph.previous.add"),
                            String::from("0.0.1,0.0.0"),
                        ),
                    ]
                }
                .iter()
                .cloned()
                .collect(),
            ),
        ]
        .iter()
        .cloned()
        .collect()
    }

    #[test]
    fn metadata_fetch_from_public_quay_succeeds() -> Fallible<()> {
        let mut runtime = init_runtime()?;

        let manifestrefs: HashMap<usize, &str> = [
            (0, ""),
            (
                1,
                "sha256:0275e5e316373faaabea9f13dfc27541e3c6e301b08bd92f443e987195faa9d6",
            ),
            (
                2,
                "sha256:e6077b9aee2bb5dae2d90d91ce2165cee802d84ce1af45e281cba47950a37f39",
            ),
            (
                3,
                "sha256:9ad8330c3b697d0631083edf72634ddf2ad1d50982d7090faf36c4a1f7eae10f",
            ),
        ]
        .iter()
        .cloned()
        .collect();

        let input_metadata = input_metadata_labels_test_annoated(manifestrefs.clone());

        let expected_metadata = expected_metadata_labels_test_annoated(manifestrefs);

        let input_graph: crate::Graph =
            crate::tests::generate_custom_graph(0, input_metadata.len(), input_metadata, None);

        let expected_graph: crate::Graph = crate::tests::generate_custom_graph(
            0,
            expected_metadata.len(),
            expected_metadata,
            None,
        );

        let future_processed_graph = Box::new(
            QuayMetadataFetchPlugin::try_new(
                "redhat/openshift-cincinnati-test-labels-public-manual".to_string(),
                DEFAULT_QUAY_LABEL_FILTER.to_string(),
                DEFAULT_QUAY_MANIFESTREF_KEY.to_string(),
                None,
                quay::v1::DEFAULT_API_BASE.to_string(),
            )
            .expect("could not initialize the QuayMetadataPlugin"),
        )
        .run_internal(InternalIO {
            graph: input_graph,
            parameters: Default::default(),
        })
        .and_then(|internal_io| Ok(internal_io.graph));

        let processed_graph = runtime
            .block_on(future_processed_graph)
            .expect("plugin run failed");

        assert_eq!(expected_graph, processed_graph);

        Ok(())
    }

    #[cfg(feature = "test-net-private")]
    #[test]
    fn metadata_fetch_from_private_quay_succeeds() -> Fallible<()> {
        let mut runtime = init_runtime()?;

        let token_file = std::env::var("CINCINNATI_TEST_QUAY_API_TOKEN_PATH")
            .expect("CINCINNATI_TEST_QUAY_API_TOKEN_PATH missing");

        let manifestrefs: HashMap<usize, &str> = [
            (0, ""),
            (
                1,
                "sha256:278fd5a7193c183f2b71523fa605b3319bfa58e4230a725a6518f1b4bd5f1437",
            ),
            (
                2,
                "sha256:9867bd09390bcfed9d21b61083def233bc9451984b11de725597939240146424",
            ),
            (
                3,
                "sha256:0f4446af92a57308180017404db3d8cf0ca20101c0d83ae76e1fc14def338399",
            ),
        ]
        .iter()
        .cloned()
        .collect();

        let input_metadata = input_metadata_labels_test_annoated(manifestrefs.clone());
        let input_graph: crate::Graph =
            crate::tests::generate_custom_graph(0, input_metadata.len(), input_metadata, None);

        let expected_metadata = expected_metadata_labels_test_annoated(manifestrefs);
        let expected_graph: crate::Graph = crate::tests::generate_custom_graph(
            0,
            expected_metadata.len(),
            expected_metadata,
            None,
        );

        let future_processed_graph = Box::new(
            QuayMetadataFetchPlugin::try_new(
                "redhat/openshift-cincinnati-test-labels-private-manual".to_string(),
                DEFAULT_QUAY_LABEL_FILTER.to_string(),
                DEFAULT_QUAY_MANIFESTREF_KEY.to_string(),
                Some(token_file.into()),
                quay::v1::DEFAULT_API_BASE.to_string(),
            )
            .context("could not initialize the QuayMetadataPlugin")?,
        )
        .run_internal(InternalIO {
            graph: input_graph,
            parameters: Default::default(),
        });

        let processed_graph = runtime
            .block_on(future_processed_graph)
            .context("plugin run failed")?
            .graph;

        assert_eq!(expected_graph, processed_graph);
        Ok(())
    }
}
