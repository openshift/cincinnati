// Copyright 2018 Alex Crawford
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use cincinnati;
use failure::Error;
use flate2::read::GzDecoder;
use futures::prelude::*;
use release::Metadata;
use serde_json;
use std::fs::File;
use std::io::Read;
use std::iter::Iterator;
use std::path::{Path, PathBuf};
use std::string::String;
use tar::Archive;
use tokio_core::reactor::Core;

#[derive(Debug, Clone)]
pub struct Release {
    pub source: String,
    pub metadata: Metadata,
}

impl Into<cincinnati::Release> for Release {
    fn into(self) -> cincinnati::Release {
        cincinnati::Release::Concrete(cincinnati::ConcreteRelease {
            version: self.metadata.version.to_string(),
            payload: self.source,
            metadata: self.metadata.metadata,
        })
    }
}

fn trim_protocol(src: &str) -> &str {
    src.trim_left_matches("https://")
        .trim_left_matches("http://")
}

pub fn read_credentials(
    credentials_path: Option<&PathBuf>,
    registry: &str,
) -> Result<(Option<String>, Option<String>), Error> {
    credentials_path.clone().map_or(Ok((None, None)), |path| {
        Ok(
            dkregistry::get_credentials(File::open(&path)?, trim_protocol(registry))
                .map_err(|e| format_err!("{}", e))?,
        )
    })
}

pub fn authenticate_client(
    client: &mut dkregistry::v2::Client,
    login_scope: String,
) -> impl Future<Item = &dkregistry::v2::Client, Error = dkregistry::errors::Error> {
    client
        .is_v2_supported()
        .and_then(move |v2_supported| {
            if !v2_supported {
                Err("API v2 not supported".into())
            } else {
                Ok(client)
            }
        })
        .and_then(move |dclient| {
            dclient.login(&[&login_scope]).and_then(|token| {
                dclient
                    .is_auth(Some(token.token()))
                    .and_then(move |is_auth| {
                        if !is_auth {
                            Err("login failed".into())
                        } else {
                            Ok(dclient.set_token(Some(token.token())))
                        }
                    })
            })
        })
}

/// Fetches a vector of all release metadata from the given repository, hosted on the given
/// registry.
pub fn fetch_releases(
    registry: &str,
    repo: &str,
    username: Option<&str>,
    password: Option<&str>,
) -> Result<Vec<Release>, Error> {
    let mut thread_runtime = tokio::runtime::current_thread::Runtime::new()?;

    let registry_host = trim_protocol(&registry);

    let mut client = dkregistry::v2::Client::configure(&Core::new()?.handle())
        .registry(registry_host)
        .insecure_registry(false)
        .username(username.map(|s| s.to_string()))
        .password(password.map(|s| s.to_string()))
        .build()
        .map_err(|e| format_err!("{}", e))?;

    let authenticated_client = thread_runtime
        .block_on(authenticate_client(
            &mut client,
            format!("repository:{}:pull", &repo),
        ))
        .map_err(|e| format_err!("{}", e))?;

    let releases = get_tags(repo.to_owned(), authenticated_client)
        .and_then(|tag| get_manifest_and_layers(tag, repo.to_owned(), authenticated_client))
        .and_then(|(tag, layer_digests)| {
            find_first_release(
                layer_digests,
                authenticated_client.to_owned(),
                registry_host.to_owned(),
                repo.to_owned(),
                tag.to_owned(),
            )
        })
        .collect();

    thread_runtime.block_on(releases)
}

// Get a stream of tags
fn get_tags(
    repo: String,
    authenticated_client: &dkregistry::v2::Client,
) -> impl Stream<Item = String, Error = Error> {
    authenticated_client
        // According to https://docs.docker.com/registry/spec/api/#listing-image-tags
        // the tags should be ordered lexically but they aren't
        .get_tags(&repo, Some(20))
        .map_err(|e| format_err!("{}", e))
}

fn get_manifest_and_layers(
    tag: String,
    repo: String,
    authenticated_client: &dkregistry::v2::Client,
) -> impl Future<Item = (String, Vec<String>), Error = failure::Error> {
    trace!("processing: {}:{}", &repo, &tag);
    authenticated_client
        .has_manifest(&repo, &tag, None)
        .join(authenticated_client.get_manifest(&repo, &tag))
        .map_err(|e| format_err!("{}", e))
        .and_then(|(manifest_kind, manifest)| {
            Ok((tag, get_layer_digests(&manifest_kind, &manifest)?))
        })
}

fn find_first_release(
    layer_digests: Vec<String>,
    authenticated_client: dkregistry::v2::Client,
    registry_host: String,
    repo: String,
    tag: String,
) -> impl Future<Item = Release, Error = Error> {
    let tag_for_error = tag.clone();

    let releases = layer_digests.into_iter().map(move |layer_digest| {
        trace!("Downloading layer {}...", &layer_digest);
        let (registry_host, repo, tag) = (registry_host.clone(), repo.clone(), tag.clone());

        authenticated_client
            .get_blob(&repo, &layer_digest)
            .map_err(|e| format_err!("{}", e))
            .into_stream()
            .filter_map(move |blob| {
                let metadata_filename = "release-metadata";

                trace!(
                    "{}: Looking for {} in archive {} with {} bytes",
                    &tag,
                    &metadata_filename,
                    &layer_digest,
                    &blob.len(),
                );

                match assemble_metadata(&blob, metadata_filename) {
                    Ok(metadata) => Some(Release {
                        source: format!("{}/{}:{}", registry_host, repo, &tag),
                        metadata,
                    }),
                    Err(e) => {
                        trace!(
                            "could not assemble metadata from layer ({}): {}",
                            &layer_digest,
                            e,
                        );
                        None
                    }
                }
            })
    });

    futures::stream::iter_ok::<_, Error>(releases)
        .flatten()
        .into_future()
        .map_err(|(e, _)| e)
        .and_then(move |(release, _)| match release {
            Some(release) => Ok(release),
            None => Err(format_err!(
                "could not find any release in tag {}",
                tag_for_error
            )),
        })
}

fn get_layer_digests(
    manifest_kind: &Option<dkregistry::mediatypes::MediaTypes>,
    manifest: &[u8],
) -> Result<Vec<String>, failure::Error> {
    use dkregistry::mediatypes::MediaTypes::{ManifestV2S1Signed, ManifestV2S2};
    use dkregistry::v2::manifest::{ManifestSchema1Signed, ManifestSchema2};

    match manifest_kind {
        Some(ManifestV2S1Signed) => serde_json::from_slice::<ManifestSchema1Signed>(manifest)
            .and_then(|m| {
                let mut l = m.get_layers();
                l.reverse();
                Ok(l)
            }),
        Some(ManifestV2S2) => serde_json::from_slice::<ManifestSchema2>(manifest).and_then(|m| {
            let mut l = m.get_layers();
            l.reverse();
            Ok(l)
        }),
        _ => bail!("unknown manifest_kind '{:?}'", manifest_kind),
    }
    .map_err(Into::into)
}

#[derive(Debug, Deserialize)]
struct Tags {
    name: String,
    tags: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct Manifest {
    #[serde(rename = "schemaVersion")]
    schema_version: usize,
    name: String,
    tag: String,
    architecture: String,
    #[serde(rename = "fsLayers")]
    fs_layers: Vec<Layer>,
}

#[derive(Debug, Deserialize)]
struct Layer {
    #[serde(rename = "blobSum")]
    blob_sum: String,
}

fn assemble_metadata(blob: &[u8], metadata_filename: &str) -> Result<Metadata, Error> {
    let mut archive = Archive::new(GzDecoder::new(blob));
    match archive
        .entries()?
        .filter_map(|entry| match entry {
            Ok(file) => Some(file),
            Err(err) => {
                debug!("failed to read archive entry: {}", err);
                None
            }
        })
        .find(|file| match file.header().path() {
            Ok(path) => path == Path::new(metadata_filename),
            Err(err) => {
                debug!("failed to read file header: {}", err);
                false
            }
        }) {
        Some(mut file) => {
            let mut contents = String::new();
            file.read_to_string(&mut contents)?;
            match serde_json::from_str::<Metadata>(&contents) {
                Ok(m) => Ok::<Metadata, Error>(m),
                Err(e) => bail!(format!("couldn't parse '{}': {}", metadata_filename, e)),
            }
        }
        None => bail!(format!("'{}' not found", metadata_filename)),
    }
    .map_err(Into::into)
}
