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
use std::collections::HashMap;
use std::{fs::File, io::Read, path::Path, path::PathBuf};
use tar::Archive;

use failure::Error;
use registry::futures::prelude::*;
use registry::tokio_core::reactor::Core;

#[derive(Debug, Clone, PartialEq)]
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

    let tags = match tcore.run(
        authenticated_client
            // According to https://docs.docker.com/registry/spec/api/#listing-image-tags
            // the tags should be ordered lexically but they aren't
            .get_tags(&repo, Some(20))
            .map_err(|e| format_err!("{}", e))
            .collect(),
    ) {
        Ok(mut tags) => {
            if tags.len() == 0 {
                bail!("{}/{} has no tags", registry, repo);
            }
            tags.sort();
            Ok(tags)
        }
        Err(e) => Err(e),
    }?;

    let releases = futures::stream::iter_ok(tags.to_owned().into_iter())
        .and_then(|tag| {
            trace!("processing: {}:{}", &repo, &tag);

            let layer_digests_future = authenticated_client
                .has_manifest(&repo, &tag, None)
                .join(authenticated_client.get_manifest(&repo, &tag))
                .map(|(manifest_kind, manifest)| match manifest_kind {
                    Some(dkregistry::mediatypes::MediaTypes::ManifestV2S1Signed) => {
                        let m: dkregistry::v2::manifest::ManifestSchema1Signed =
                                serde_json::from_slice(manifest.as_slice())?;
                        Ok((
                            tag,
                            {
                                let mut l = m.get_layers();
                                l.reverse();
                                l
                            },
                            m.get_labels(0),
                        ))
                    }
                    Some(dkregistry::mediatypes::MediaTypes::ManifestV2S2) => {
                        let m: dkregistry::v2::manifest::ManifestSchema2 =
                            serde_json::from_slice(manifest.as_slice())?;
                        Ok((
                            tag,
                            {
                                let mut l = m.get_layers();
                                l.reverse();
                                l
                            },
                            None,
                        ))
                    }
                    _ => bail!("unknown manifest_kind '{:?}'", manifest_kind),
                });

            layer_digests_future
        })
        .map_err(|e| format_err!("{}", e))
        .and_then(|tag_layer_digests_labels| {
            let (tag, layer_digests, labels) = match tag_layer_digests_labels {
                Ok(ok) => ok,
                Err(e) => bail!("{}", e),
            };

            let mut layer_digests_iter = layer_digests.into_iter();
            let inner_loop = futures::future::loop_fn(
                (
                    layer_digests_iter
                        .next()
                        .ok_or(format_err!("layer_digest_iter yielded none"))?,
                    layer_digests_iter,
                ),
                move |(layer_digest, mut layer_digests_iter)| {
                    // FIXME: it would be nice to avoid this
                    let (registry_host, repo, tag, labels, layer_digest) = (
                        registry_host.clone(),
                        repo.clone(),
                        tag.clone(),
                        labels.clone(),
                        layer_digest.clone(),
                    );

                    trace!("Downloading layer {}...", &layer_digest);
                    authenticated_client
                        .get_blob(&repo, &layer_digest)
                        .map_err(|e| format_err!("{}", e))
                        .and_then(move |blob| {
                            let metadata_filename = "cincinnati.json";
                            trace!(
                                "{}: Looking for {} in archive {} with {} bytes",
                                tag,
                                metadata_filename,
                                layer_digest,
                                blob.len(),
                            );

                            match assemble_metadata(&blob, metadata_filename, labels) {
                                Ok(metadata) => {
                                    let release = Release {
                                        source: format!("{}/{}:{}", &registry_host, &repo, &tag),
                                        metadata: metadata,
                                    };
                                    trace!("Found release '{:?}'", release);
                                    Ok(futures::future::Loop::Break(Ok(release)))
                                }
                                Err(e) => {
                                    warn!("{}", e);
                                    match layer_digests_iter.next() {
                                        Some(layer_digest) => Ok(futures::future::Loop::Continue(
                                            (layer_digest, layer_digests_iter),
                                        )),
                                        None => Ok(futures::future::Loop::Break(Err(format_err!(
                                            "Could not find {} in any layer of tag {}",
                                            &metadata_filename,
                                            &tag
                                        )))),
                                    }
                                }
                            }
                        })
                },
            );
            Ok(inner_loop)
        })
        .map_err(|e| format_err!("{}", e))
        // FIXME: there must be a more generic way to flatten this (`flatten()` doesn't work)
        .and_then(|f| f)
        .and_then(|f| f)
        .collect();

    let releases = tcore.run(releases)?;
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

fn assemble_metadata(
    blob: &[u8],
    metadata_filename: &str,
    labels: Option<HashMap<String, String>>,
) -> Result<release::Metadata, Error> {
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
            match serde_json::from_str::<release::Metadata>(&contents) {
                Ok(mut m) => {
                    m.manifest_labels = labels.to_owned();
                    Ok::<release::Metadata, Error>(m)
                }
                Err(e) => bail!(format!("couldn't parse '{}': {}", metadata_filename, e)),
            }
        }
        None => bail!(format!("'{}' not found", metadata_filename)),
    }
    .map_err(Into::into)
}

#[cfg(test)]
mod tests {
    use super::*;
    use release::Metadata;
    use release::MetadataKind::V0;
    use semver::Version;

    fn init_logger() {
        let _ = env_logger::try_init_from_env(env_logger::Env::default());
    }

    #[test]
    fn fetch_release_with_credentials_must_succeed() {
        init_logger();

        let registry = "https://quay.io";
        let repo = "steveej/cincinnati-test";
        let credentials_path = Some(PathBuf::from(r"tests/net/quay_credentials.json"));
        let releases =
            fetch_releases(&registry, &repo, &credentials_path).expect("fetch_releases failed: ");
        assert_eq!(2, releases.len());

        let metadata0 = std::collections::HashMap::new();
        let mut metadata1 = std::collections::HashMap::new();
        metadata1.insert(String::from("kind"), String::from("test"));

        assert_eq!(
            vec![
                Release {
                    source: "quay.io/steveej/cincinnati-test:0.0.0".to_string(),
                    metadata: Metadata {
                        kind: V0,
                        version: Version {
                            major: 0,
                            minor: 0,
                            patch: 0,
                            pre: vec![],
                            build: vec![],
                        },
                        previous: vec![],
                        next: vec![Version {
                            major: 0,
                            minor: 0,
                            patch: 1,
                            pre: vec![],
                            build: vec![],
                        }],
                        metadata: metadata0,
                        manifest_labels: None
                    },
                },
                Release {
                    source: "quay.io/steveej/cincinnati-test:0.0.1".to_string(),
                    metadata: Metadata {
                        kind: V0,
                        version: Version {
                            major: 0,
                            minor: 0,
                            patch: 1,
                            pre: vec![],
                            build: vec![],
                        },
                        previous: vec![Version {
                            major: 0,
                            minor: 0,
                            patch: 0,
                            pre: vec![],
                            build: vec![],
                        }],
                        next: vec![],
                        metadata: metadata1,
                        manifest_labels: None
                    },
                },
            ],
            releases
        )
    }

    #[test]
    fn fetch_release_without_credentials_must_fail() {
        init_logger();

        let registry = "https://quay.io";
        let repo = "steveej/cincinnati-test";
        let credentials_path = None;
        let releases = fetch_releases(&registry, &repo, &credentials_path);
        assert_eq!(true, releases.is_err());
        assert_eq!(
            true,
            releases
                .err()
                .unwrap()
                .to_string()
                .contains("401 Unauthorized")
        );
    }

    #[test]
    fn fetch_release_with_labels() {
        init_logger();

        let registry = "https://quay.io";
        let repo = "steveej/cincinnati-test-labels";
        let credentials_path = Some(PathBuf::from(r"tests/net/quay_credentials.json"));
        let releases =
            fetch_releases(&registry, &repo, &credentials_path).expect("fetch_releases failed: ");
        assert_eq!(2, releases.len());

        assert_eq!(
            vec![
                Release {
                    source: "quay.io/steveej/cincinnati-test-labels:0.0.0".to_string(),
                    metadata: Metadata {
                        kind: V0,
                        version: Version {
                            major: 0,
                            minor: 0,
                            patch: 0,
                            pre: vec![],
                            build: vec![],
                        },
                        previous: vec![],
                        next: vec![],
                        metadata: [("layer".to_string(), "1".to_string())]
                            .iter()
                            .cloned()
                            .collect(),
                        manifest_labels: Some(
                            [("channel".into(), "alpha".into())]
                                .iter()
                                .cloned()
                                .collect()
                        ),
                    },
                },
                Release {
                    source: "quay.io/steveej/cincinnati-test-labels:0.0.1".to_string(),
                    metadata: Metadata {
                        kind: V0,
                        version: Version {
                            major: 0,
                            minor: 0,
                            patch: 1,
                            pre: vec![],
                            build: vec![],
                        },
                        previous: vec![Version {
                            major: 0,
                            minor: 0,
                            patch: 0,
                            pre: vec![],
                            build: vec![],
                        }],
                        next: vec![],
                        metadata: [("layer".to_string(), "1".to_string())]
                            .iter()
                            .cloned()
                            .collect(),
                        manifest_labels: Some(
                            [("channel".into(), "beta".into())]
                                .iter()
                                .cloned()
                                .collect()
                        ),
                    },
                },
            ],
            releases
        )
    }

    #[test]
    fn fetch_release_with_non_existing_json_must_error_gracefully() {
        init_logger();

        let registry = "https://quay.io";
        let repo = "steveej/cincinnati-test-nojson";
        let credentials_path = None;
        let releases = fetch_releases(&registry, &repo, &credentials_path);
        assert_eq!(true, releases.is_err());
        assert_eq!(
            true,
            releases
                .err()
                .unwrap()
                .to_string()
                .contains("Could not find")
        );
    }
}
