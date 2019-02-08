//! This module implements the fetching of dynamic metadata

use failure::{Error, Fallible};
use futures::future::Future;
use quay;
use registry;
use std::path::PathBuf;

pub static DEFAULT_QUAY_LABEL_FILTER: &str = "com.openshift.upgrades.graph";
pub static MANIFESTREF_KEY: &str = "com.openshift.upgrades.graph.release.manifestref";

pub trait AsyncMetadataFetcher<T, U, E> {
    fn fetch_metadata(&self, fetch_args: U) -> Box<Future<Item = T, Error = E>>;
}

/// Convenience type for a metadata entry
type MetadataEntry = (String, String);

/// Convenience type for fetch_args()'s metadata_fetcher argument
pub type MetadataFetcher = Box<AsyncMetadataFetcher<Vec<MetadataEntry>, String, Error>>;

pub struct QuayMetadataFetcher {
    client: quay::v1::Client,
    label_filter: String,
    repo: String,
}
impl AsyncMetadataFetcher<Vec<MetadataEntry>, String, Error> for QuayMetadataFetcher {
    fn fetch_metadata(
        &self,
        manifestref: String,
    ) -> Box<Future<Item = Vec<MetadataEntry>, Error = Error>> {
        Box::new(
            self.client
                .get_labels(
                    self.repo.clone(),
                    manifestref,
                    Some(self.label_filter.clone()),
                )
                .map(|labels| labels.into_iter().map(Into::into).collect()),
        )
    }
}

impl QuayMetadataFetcher {
    pub fn try_new(
        label_filter: String,
        api_token_path: Option<&PathBuf>,
        api_base: String,
        repo: String,
    ) -> Fallible<Box<Self>> {
        let api_token =
            quay::read_credentials(api_token_path).expect("could not read quay API credentials");

        let client: quay::v1::Client = quay::v1::Client::builder()
            .access_token(api_token.map(|s| s.to_string()))
            .api_base(Some(api_base.to_string()))
            .build()?;

        Ok(Box::new(QuayMetadataFetcher {
            client,
            label_filter,
            repo,
        }))
    }
}

/// Asynchronously fetches and populates the dynamic metadata for the given releases
///
/// This method is all or nothing and fails in these cases:
/// * a Release doesn't contain the manifestref in its metadata
/// * the dynamic metadata can't be fetched for a manifestref
pub fn fetch_and_populate_dynamic_metadata<'a>(
    metadata_fetcher: &'a MetadataFetcher,
    releases: Vec<registry::Release>,
) -> impl Future<Item = Vec<registry::Release>, Error = Error> + 'a {
    let populated_releases = releases.into_iter().map(move |mut release| {
        futures::future::ok(release.metadata.metadata.remove(MANIFESTREF_KEY))
            .and_then(move |manifestref_value| match manifestref_value {
                Some(manifestref) => Ok((release, manifestref)),
                None => Err(format_err!(
                    "metadata of release '{}' doesn't contain the manifestref",
                    release.source
                )),
            })
            .and_then(move |(mut release, manifestref)| {
                metadata_fetcher
                    .fetch_metadata(manifestref.to_string())
                    .and_then(|dynamic_metadata| {
                        release.metadata.metadata = release
                            .metadata
                            .metadata
                            .into_iter()
                            .chain(dynamic_metadata)
                            .collect();

                        Ok(release)
                    })
            })
    });

    futures::future::join_all(populated_releases)
}
