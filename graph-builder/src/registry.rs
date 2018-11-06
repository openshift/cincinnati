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
use failure::ResultExt;
use flate2::read::GzDecoder;
use release;
use serde_json;
use std;
use std::{fs::File, io::Read, path::Path, path::PathBuf};
use tar::Archive;

use failure::Error;
use registry::futures::prelude::*;
use registry::tokio_core::reactor::Core;

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

pub fn authenticate_client<'a>(
    client: &'a mut dkregistry::v2::Client,
    login_scope: std::string::String,
) -> impl futures::future::Future<Item = &'a dkregistry::v2::Client, Error = dkregistry::errors::Error>
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
        .and_then(|dclient| {
            dclient.is_auth(None).and_then(move |is_auth| {
                if is_auth {
                    Err("no login performed, but already authenticated".into())
                } else {
                    Ok(dclient)
                }
            })
        })
        .and_then(move |dclient| {
            dclient.login(&[&login_scope]).and_then(move |token| {
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
    credentials_path: &Option<PathBuf>,
) -> Result<Vec<Release>, Error> {
    let mut tcore = Core::new()?;

    let registry_host = trim_protocol(registry);

    let mut client = dkregistry::v2::Client::configure(&tcore.handle())
        .registry(registry_host)
        .insecure_registry(false)
        .read_credentials(|| -> Result<Box<dyn std::io::Read>, Error> {
            match credentials_path {
                Some(path) => match File::open(path) {
                    Ok(file) => Ok(Box::new(file)),
                    Err(e) => Err(e.into()),
                },
                None => Ok(Box::new(std::io::empty())),
            }
        }()?)
        .build()
        .map_err(|e| format_err!("{}", e))?;

    let authenticated_client = tcore
        .run(authenticate_client(
            &mut client,
            format!("repository:{}:pull", &repo),
        ))
        .map_err(|e| format_err!("{}", e))?;

    let layer_digests_tag = authenticated_client
        .get_tags(&repo, None)
        .and_then(|tag| {
            trace!("processing: {}:{}", &repo, &tag);

            let tag_clone0 = tag.clone(); // TODO(steveeJ): is there a way to avoid this?
            let manifest_kind_future = authenticated_client
                .has_manifest(&repo, &tag, None)
                .and_then(move |manifest_kind| match manifest_kind {
                    Some(manifest_kind) => Ok(manifest_kind),
                    None => {
                        Err(format!("{}:{} doesn't have a manifest", &repo, &tag_clone0).into())
                    }
                })
                .inspect(|manifest_kind| {
                    trace!("manifest_kind: {:?}", manifest_kind);
                });

            let manifest_future = authenticated_client.get_manifest(&repo, &tag);

            let layer_digests_future =
                manifest_kind_future
                    .join(manifest_future)
                    .map(|(manifest_kind, manifest)| match manifest_kind {
                        dkregistry::mediatypes::MediaTypes::ManifestV2S1Signed => {
                            let m: dkregistry::v2::manifest::ManifestSchema1Signed =
                                serde_json::from_slice(manifest.as_slice())?;
                            Ok((m.get_layers(), tag))
                        }
                        dkregistry::mediatypes::MediaTypes::ManifestV2S2 => {
                            let m: dkregistry::v2::manifest::ManifestSchema2 =
                            serde_json::from_slice(manifest.as_slice())?;
                            Ok((m.get_layers(), tag))
                        }
                        _ => Err(format_err!("unknown manifest_kind '{:?}'", manifest_kind)),
                    });

            layer_digests_future
        })
        .map_err(|e| format_err!("{}", e))
        .and_then(|f| f)
        .collect();

    let layer_digests_tag = tcore.run(layer_digests_tag)?;

    let releases = layer_digests_tag
        .into_iter()
        .rev()
        .filter_map(|(layer_digests, tag)| {
            let mut found = false;
            let mut map = layer_digests.into_iter().rev().filter_map(|layer_digest| {
                trace!("Downloading layer {}...", &layer_digest);
                if found {
                    return None;
                }

                match tcore.run(
                    authenticated_client
                        .get_blob(&repo, &layer_digest)
                        .map_err(|e| format_err!("{}", e))
                        .and_then(|blob| {
                            trace!("Layer has {} bytes.", blob.len());
                            let metadata =
                                extract_metadata_from_layer_blob(&blob, "cincinnati.json")?;
                            let release = Release {
                                source: format!("{}/{}:{}", &registry_host, &repo, &tag),
                                metadata: metadata,
                            };
                            trace!("Found release '{:?}'", release);
                            found = true;
                            Ok(release)
                        }),
                ) {
                    Ok(release) => Some(release),
                    Err(e) => {
                        debug!("No release in layer with digest {}: {}", &layer_digest, e);
                        None
                    }
                }
            });
            map.nth(0)
        })
        .collect::<Vec<Release>>();

    Ok(releases)
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

fn extract_metadata_from_layer_blob(
    blob: &[u8],
    metadata_filename: &str,
) -> Result<release::Metadata, Error> {
    trace!("Looking for {} in archive", metadata_filename);

    let mut archive = Archive::new(GzDecoder::new(blob.as_ref()));
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
            serde_json::from_str(&contents)
                .context(format!("failed to parse {}", metadata_filename))
        }
        None => bail!(format!("'{}' not found", metadata_filename)),
    }
    .map_err(Into::into)
}
