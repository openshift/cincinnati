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

extern crate dkregistry;
extern crate failure;
extern crate futures;
extern crate tokio_core;

use cincinnati;
use failure::Error;
use flate2::read::GzDecoder;
use registry::futures::future::Either;
use registry::futures::prelude::*;
use registry::tokio_core::reactor::Core;
use release;
use serde_json;
use std::{self, fs::File, io::Read, path::Path, path::PathBuf};
use tar::Archive;

#[derive(Debug, Clone)]
pub struct Release {
    pub source: String,
    pub metadata: release::Metadata,
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
    login_scope: std::string::String,
) -> impl futures::future::Future<Item = &dkregistry::v2::Client, Error = dkregistry::errors::Error>
{
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
    let mut tcore = Core::new()?;

    let registry_host = trim_protocol(&registry);

    let mut client = dkregistry::v2::Client::configure(&tcore.handle())
        .registry(registry_host)
        .insecure_registry(false)
        .username(username.map(|s| s.to_string()))
        .password(password.map(|s| s.to_string()))
        .build()
        .map_err(|e| format_err!("{}", e))?;

    let authenticated_client = tcore
        .run(authenticate_client(
            &mut client,
            format!("repository:{}:pull", &repo),
        ))
        .map_err(|e| format_err!("{}", e))?;

    let tags = tcore
        .run(
            authenticated_client
                // According to https://docs.docker.com/registry/spec/api/#listing-image-tags
                // the tags should be ordered lexically but they aren't
                .get_tags(&repo, Some(20))
                .map_err(|e| format_err!("{}", e))
                .collect(),
        )
        .map(|mut tags| {
            if tags.is_empty() {
                warn!("{}/{} has no tags", registry_host, repo)
            };
            tags.sort();
            tags
        })?;

    let releases = futures::stream::iter_ok(tags)
        .and_then(|tag| {
            trace!("processing: {}:{}", &repo, &tag);
            authenticated_client
                .has_manifest(&repo, &tag, None)
                .join(authenticated_client.get_manifest(&repo, &tag))
                .map_err(|e| format_err!("{}", e))
                .and_then(|(manifest_kind, manifest)| {
                    Ok((tag, get_layer_digests(&manifest_kind, &manifest)?))
                })
        })
        .and_then(|(tag, layer_digests)| {
            find_first_release(
                layer_digests,
                authenticated_client.clone(),
                registry_host.into(),
                repo.into(),
                tag,
            )
        })
        .collect();

    tcore.run(releases)
}

fn get_layer_digests(
    manifest_kind: &Option<dkregistry::mediatypes::MediaTypes>,
    manifest: &[u8],
) -> Result<Vec<String>, failure::Error> {
    use registry::dkregistry::mediatypes::MediaTypes::{ManifestV2S1Signed, ManifestV2S2};
    use registry::dkregistry::v2::manifest::{ManifestSchema1Signed, ManifestSchema2};

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

fn find_first_release(
    layer_digests: Vec<String>,
    authenticated_client: dkregistry::v2::Client,
    registry_host: String,
    repo: String,
    tag: String,
) -> impl futures::future::Future<Item = Release, Error = failure::Error> {
    futures::future::loop_fn(
        layer_digests.into_iter().peekable(),
        move |mut layer_digests_iter| {
            let layer_digest = {
                if let Some(layer_digest) = layer_digests_iter.next() {
                    layer_digest
                } else {
                    let no_release_found = futures::future::ok(continue_or_break_loop(
                        layer_digests_iter,
                        None,
                        Some(format_err!(
                            "no release found for tag '{}' and no more layers to examine",
                            tag
                        )),
                    ));
                    return Either::A(no_release_found);
                }
            };

            // FIXME: it would be nice to avoid this
            let (registry_host, repo, tag, layer_digest) = (
                registry_host.clone(),
                repo.clone(),
                tag.clone(),
                layer_digest.clone(),
            );

            trace!("Downloading layer {}...", &layer_digest);
            let examine_blobs = authenticated_client
                .get_blob(&repo, &layer_digest)
                .map_err(|e| format_err!("could not download blob: {}", e))
                .and_then(move |blob| {
                    let metadata_filename = "cincinnati.json";
                    trace!(
                        "{}: Looking for {} in archive {} with {} bytes",
                        tag,
                        metadata_filename,
                        layer_digest,
                        blob.len(),
                    );

                    match assemble_metadata(&blob, metadata_filename) {
                        Ok(metadata) => futures::future::ok(continue_or_break_loop(
                            layer_digests_iter,
                            Some(Release {
                                source: format!("{}/{}:{}", &registry_host, &repo, &tag),
                                metadata,
                            }),
                            None,
                        )),
                        Err(e) => futures::future::ok(continue_or_break_loop(
                            layer_digests_iter,
                            None,
                            Some(format_err!(
                                "could not assemble metadata from blob '{:?}': {}",
                                String::from_utf8_lossy(&blob),
                                e,
                            )),
                        )),
                    }
                });
            Either::B(examine_blobs)
        },
    )
    .flatten()
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

fn assemble_metadata(blob: &[u8], metadata_filename: &str) -> Result<release::Metadata, Error> {
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
            match serde_json::from_str::<release::Metadata>(&contents) {
                Ok(m) => Ok::<release::Metadata, Error>(m),
                Err(e) => bail!(format!("couldn't parse '{}': {}", metadata_filename, e)),
            }
        }
        None => bail!(format!("'{}' not found", metadata_filename)),
    }
    .map_err(Into::into)
}

fn continue_or_break_loop<I>(
    mut layer_digests_iter: std::iter::Peekable<I>,
    r: Option<Release>,
    e: Option<Error>,
) -> futures::future::Loop<Result<Release, Error>, std::iter::Peekable<I>>
where
    I: std::iter::Iterator<Item = String>,
{
    match (r, e) {
        (Some(r), _) => {
            trace!("Found release '{:?}'", r);
            futures::future::Loop::Break(Ok(r))
        }
        (_, Some(e)) => {
            warn!("{}", e);
            match layer_digests_iter.peek() {
                Some(_) => futures::future::Loop::Continue(layer_digests_iter),
                None => futures::future::Loop::Break(Err(e)),
            }
        }
        _ => continue_or_break_loop(
            layer_digests_iter,
            None,
            Some(format_err!(
                "continue_or_break called with unexpected condition"
            )),
        ),
    }
}
