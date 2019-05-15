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
use failure::{Error, Fallible, ResultExt};
use flate2::read::GzDecoder;
use futures::prelude::*;
use crate::release::Metadata;
use serde_json;
use std::collections::HashMap;
use std::fs::File;
use std::io::Read;
use std::iter::Iterator;
use std::path::{Path, PathBuf};
use std::string::String;
use tar::Archive;

#[derive(Debug, Clone, PartialEq)]
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

#[derive(Debug, Clone, PartialEq, Default)]
pub struct Registry {
    pub(crate) scheme: String,
    pub(crate) insecure: bool,
    pub(crate) host: String,
    pub(crate) port: Option<u16>,
}

impl Registry {
    pub fn try_from_str(src: &str) -> Fallible<Self> {
        macro_rules! process_regex {
            ($a:ident, $r:expr, $b:tt) => {
                if let Some($a) = regex::Regex::new($r)
                    .context(format!("could not compile regex pattern {}", $r))?
                    .captures(src)
                {
                    $b
                };
            };
        }

        process_regex!(
            capture,
            // match scheme://h.o.s.t:port
            r"^(?P<scheme>[a-z]+)(:/{2})(?P<host>([0-9a-zA-Z]+\.)*([0-9a-zA-Z]+)):(?P<port>[0-9]+)$",
            {
                let scheme = capture["scheme"].to_string();
                return Ok(Registry {
                    insecure: Registry::insecure_scheme(&scheme)?,
                    scheme,
                    host: capture["host"].to_string(),
                    port: Some(
                        capture["port"]
                            .parse()
                            .expect("could not parse port as a number"),
                    ),
                });
            }
        );

        process_regex!(
            capture,
            // match scheme://h.o.s.t
            r"^(?P<scheme>[a-z]+)(:/{2})(?P<host>([0-9a-zA-Z]+\.)*([0-9a-zA-Z]+))$",
            {
                let scheme = capture["scheme"].to_string();
                return Ok(Registry {
                    insecure: Registry::insecure_scheme(&scheme)?,
                    scheme,
                    host: capture["host"].to_string(),
                    ..Default::default()
                });
            }
        );

        process_regex!(
            capture,
            // match h.o.s.t:port
            r"^(?P<host>([0-9a-zA-Z\-]+\.)+([0-9a-zA-Z]+)):(?P<port>[0-9]+)$",
            {
                return Ok(Registry {
                    host: capture["host"].to_string(),
                    port: Some(
                        capture["port"]
                            .parse()
                            .expect("could not parse port as a number"),
                    ),
                    ..Default::default()
                });
            }
        );

        process_regex!(
            capture,
            // match h.o.s.t
            r"^(?P<host>([0-9a-zA-Z\-]+\.)*([0-9a-zA-Z]+))$",
            {
                return Ok(Registry {
                    host: capture["host"].to_string(),
                    ..Default::default()
                });
            }
        );

        bail!("unsupported registry format {}", src)
    }

    pub fn try_new(scheme: String, host: String, port: Option<u16>) -> Fallible<Self> {
        Ok(Registry {
            host,
            port,
            insecure: Self::insecure_scheme(&scheme)?,
            scheme,
        })
    }

    pub fn host_port_string(&self) -> String {
        format!(
            "{}{}",
            self.host,
            if let Some(port) = self.port {
                format!(":{}", port)
            } else {
                "".to_string()
            }
        )
    }

    fn insecure_scheme(scheme: &str) -> Fallible<bool> {
        match scheme {
            "https" => Ok(false),
            "http" => Ok(true),
            scheme => bail!("unsupported url scheme specified '{}'", scheme),
        }
    }
}

pub fn read_credentials(
    credentials_path: Option<&PathBuf>,
    registry_host: &str,
) -> Result<(Option<String>, Option<String>), Error> {
    credentials_path.map_or(Ok((None, None)), |path| {
        let file = File::open(&path).context(format!("could not open '{:?}'", path))?;

        Ok(dkregistry::get_credentials(file, &registry_host).map_err(|e| format_err!("{}", e))?)
    })
}

pub fn authenticate_client(
    dclient: &mut dkregistry::v2::Client,
    login_scope: String,
) -> impl Future<Item = &dkregistry::v2::Client, Error = dkregistry::errors::Error> {
    dclient.clone().ensure_v2_registry().and_then(move |_| {
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
    registry: &Registry,
    repo: &str,
    username: Option<&str>,
    password: Option<&str>,
    cache: &mut HashMap<u64, Option<Release>>,
    manifestref_key: &str,
) -> Result<Vec<Release>, Error> {
    let mut thread_runtime = tokio::runtime::current_thread::Runtime::new()?;

    let mut client = dkregistry::v2::Client::configure()
        .registry(&registry.host_port_string())
        .insecure_registry(registry.insecure)
        .username(username.map(|s| s.to_string()))
        .password(password.map(|s| s.to_string()))
        .build()
        .map_err(|e| format_err!("{}", e))?;

    let authenticated_client = {
        let is_auth =
            thread_runtime.block_on(client.is_auth(None).map_err(|e| format_err!("{}", e)))?;

        if is_auth {
            &client
        } else {
            thread_runtime.block_on(
                authenticate_client(&mut client, format!("repository:{}:pull", &repo))
                    .map_err(|e| format_err!("{}", e)),
            )?
        }
    };

    let tags = get_tags(repo.to_owned(), authenticated_client)
        .and_then(|tag| get_manifest_and_layers(tag, repo.to_owned(), authenticated_client))
        .collect();
    let tagged_layers = thread_runtime.block_on(tags)?;

    let mut releases = Vec::with_capacity(tagged_layers.len());

    for (tag, manifestref, layer_digests) in tagged_layers {
        let mut release = match cache_release(
            layer_digests,
            authenticated_client.to_owned(),
            registry.host.to_owned().to_string(),
            repo.to_owned(),
            tag.to_owned(),
            cache,
        ) {
            Ok(Some(release)) => release,
            Ok(None) => {
                // Reminder: this means the layer_digests point to layers
                // without any release and we've cached this before
                continue;
            }
            Err(e) => bail!(e),
        };

        if let Some(manifestref) = manifestref {
            // Replace the tag specifier with the manifestref
            release.source = {
                let mut source_split: Vec<&str> = release.source.split(':').collect();
                let _ = source_split.pop();

                format!("{}@{}", source_split.join(":"), manifestref)
            };

            // Attach the manifestref this release was found in for further processing
            release
                .metadata
                .metadata
                .insert(manifestref_key.to_owned(), manifestref);
        }

        releases.push(release);
    }
    releases.shrink_to_fit();

    Ok(releases)
}

/// Look up release metadata for a specific tag, and cache it.
///
/// Each tagged release is looked up at most once and both
/// positive (Some metadata) and negative (None) results cached
/// indefinitely.
///
/// Update Images with release metadata should be immutable, but
/// tags on registry can be mutated at any time. Thus, the cache
/// is keyed on the hash of tag layers.
fn cache_release(
    layer_digests: Vec<String>,
    authenticated_client: dkregistry::v2::Client,
    registry_host: String,
    repo: String,
    tag: String,
    cache: &mut HashMap<u64, Option<Release>>,
) -> Fallible<Option<Release>> {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    // TODO(lucab): get rid of this synchronous lookup, by
    // introducing a dedicated actor which owns the cache
    // and handles queries and insertions.
    let mut thread_runtime = tokio::runtime::current_thread::Runtime::new()?;

    let hashed_tag_layers = {
        let mut hasher = DefaultHasher::new();
        layer_digests.hash(&mut hasher);
        hasher.finish()
    };

    if let Some(release) = cache.get(&hashed_tag_layers) {
        trace!("Using cached release metadata for tag {}", &tag);
        return Ok(release.clone());
    }

    let tagged_release = find_first_release(
        layer_digests,
        authenticated_client,
        registry_host,
        repo,
        tag,
    );
    let (tag, release) = thread_runtime
        .block_on(tagged_release)
        .context("failed to find first release")?;

    trace!("Caching release metadata for new tag {}", &tag);
    cache.insert(hashed_tag_layers, release.clone());
    Ok(release)
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
) -> impl Future<Item = (String, Option<String>, Vec<String>), Error = failure::Error> {
    trace!("processing: {}:{}", &repo, &tag);
    authenticated_client
        .has_manifest(&repo, &tag, None)
        .join(authenticated_client.get_manifest_and_ref(&repo, &tag))
        .map_err(|e| format_err!("{}", e))
        .and_then(|(manifest_kind, (manifest, manifestref))| {
            Ok((
                tag,
                manifestref,
                get_layer_digests(&manifest_kind, &manifest)?,
            ))
        })
}

fn find_first_release(
    layer_digests: Vec<String>,
    authenticated_client: dkregistry::v2::Client,
    registry_host: String,
    repo: String,
    repo_tag: String,
) -> impl Future<Item = (String, Option<Release>), Error = Error> {
    let tag = repo_tag.clone();

    let releases = layer_digests.into_iter().map(move |layer_digest| {
        trace!("Downloading layer {}...", &layer_digest);
        let (registry_host, repo, tag) = (registry_host.clone(), repo.clone(), repo_tag.clone());

        authenticated_client
            .get_blob(&repo, &layer_digest)
            .map_err(|e| format_err!("{}", e))
            .into_stream()
            .filter_map(move |blob| {
                let metadata_filename = "release-manifests/release-metadata";

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
                        debug!(
                            "could not assemble metadata from layer ({}) of tag '{}': {}",
                            &layer_digest, &tag, e,
                        );
                        None
                    }
                }
            })
    });

    futures::stream::iter_ok::<_, Error>(releases)
        .flatten()
        .take(1)
        .collect()
        .map(move |mut releases| {
            if releases.is_empty() {
                warn!("could not find any release in tag {}", tag);
                (tag, None)
            } else {
                (tag, Some(releases.remove(0)))
            }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registry_try_parse_valid() {
        let tests = vec![
            (
                "http://localhost:8080",
                Registry {
                    scheme: "http".to_string(),
                    insecure: true,
                    host: "localhost".to_string(),
                    port: Some(8080),
                },
            ),
            (
                "127.0.0.1",
                Registry {
                    scheme: "".to_string(),
                    insecure: false,
                    host: "127.0.0.1".to_string(),
                    port: None,
                },
            ),
            (
                "sat-r220-02.lab.eng.rdu2.redhat.com:5000",
                Registry {
                    scheme: "".to_string(),
                    insecure: false,
                    host: "sat-r220-02.lab.eng.rdu2.redhat.com".to_string(),
                    port: Some(5000),
                },
            ),
            (
                "quay.io",
                Registry {
                    scheme: "".to_string(),
                    insecure: false,
                    host: "quay.io".to_string(),
                    port: None,
                },
            ),
            (
                "https://quay.io",
                Registry {
                    scheme: "https".to_string(),
                    insecure: false,
                    host: "quay.io".to_string(),
                    port: None,
                },
            ),
        ];

        for (input, expected) in tests {
            let registry: Registry = Registry::try_from_str(input)
                .expect(&format!("could not parse {} to registry", input));
            assert_eq!(registry, expected);
        }
    }
}
