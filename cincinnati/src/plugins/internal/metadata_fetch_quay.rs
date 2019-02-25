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
use plugins::{InternalIO, InternalPlugin};
use std::path::PathBuf;
use ReleaseId;

pub static DEFAULT_QUAY_LABEL_FILTER: &str = "com.openshift.upgrades.graph";
pub static DEFAULT_QUAY_MANIFESTREF_KEY: &str = "com.openshift.upgrades.graph.release.manifestref";

pub struct QuayMetadataFetchPlugin {
    client: quay::v1::Client,
    repo: String,
    label_filter: String,
    manifestref_key: String,
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

impl QuayMetadataFetchPlugin {
    pub fn try_new(
        repo: String,
        label_filter: String,
        manifestref_key: String,
        api_token_path: Option<&PathBuf>,
        api_base: String,
    ) -> Fallible<Self> {
        let api_token =
            quay::read_credentials(api_token_path).expect("could not read quay API credentials");

        let client: quay::v1::Client = quay::v1::Client::builder()
            .access_token(api_token.map(|s| s.to_string()))
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
