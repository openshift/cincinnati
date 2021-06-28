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

use crate as cincinnati;

use self::cincinnati::plugins::internal::graph_builder::release::Metadata;
use self::cincinnati::plugins::internal::graph_builder::release::MetadataKind;
use self::cincinnati::plugins::prelude_plugin_impl::*;

use flate2::read::GzDecoder;
use futures::lock::Mutex as FuturesMutex;
use futures::prelude::*;
use futures::TryStreamExt;
use log::{debug, error, trace, warn};
use semver::Version;
use serde::Deserialize;
use serde_json;
use std::fs::File;
use std::io::Read;
use std::iter::Iterator;
use std::path::{Path, PathBuf};
use std::string::String;
use std::sync::Arc;
use tar::Archive;

/// Module for the release cache
pub mod cache {
    use super::cincinnati::plugins::internal::graph_builder::release::Metadata;
    use std::collections::HashMap;
    use std::sync::Arc;
    use tokio::sync::RwLock as FuturesRwLock;

    /// The key type of the cache
    type Key = String;

    /// The values of the cache
    type Value = Option<Metadata>;

    /// The sync cache to hold the `Release` cache
    type CacheSync = HashMap<Key, Value>;

    /// The async wrapper for `Cache`
    type CacheAsync<T> = FuturesRwLock<T>;

    /// The cache to hold the `Release` cache
    pub type Cache = Arc<CacheAsync<CacheSync>>;

    /// Instantiate a new cache
    pub fn new() -> Cache {
        Arc::new(CacheAsync::new(CacheSync::new()))
    }
}

#[derive(Debug, Clone, PartialEq, Default, Deserialize)]
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

    /// Format the registry to qualified string.ToOwned
    ///
    /// Even though it creates an invalid registry reference according to Docker
    /// and podman, this includes the scheme if the registry is set to insecure,
    /// because we have no other way to indicate this in the URL.
    pub fn host_port_string(&self) -> String {
        format!(
            "{}{}{}",
            if self.insecure && !self.scheme.is_empty() {
                format!("{}://", self.scheme)
            } else {
                "".to_string()
            },
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

pub async fn new_registry_client(
    registry: &Registry,
    repo: &str,
    username: Option<&str>,
    password: Option<&str>,
) -> Result<dkregistry::v2::Client, Error> {
    let client = {
        let client_builder = dkregistry::v2::Client::configure()
            .registry(&registry.host_port_string())
            .insecure_registry(registry.insecure);
        let scope = format!("repository:{}:pull", &repo);

        if username.is_some() && password.is_some() {
            client_builder
                .username(username.map(ToString::to_string))
                .password(password.map(ToString::to_string))
                .build()?
                .authenticate(&[&scope])
                .await?
        } else {
            let client = client_builder.build()?;

            if client
                .is_v2_supported_and_authorized()
                .await
                .map(|(_, authorized)| authorized)?
            {
                client
            } else {
                debug!("registry not authorized, attempting anonymous authorization");
                client.authenticate(&[&scope]).await?
            }
        }
    };

    Ok(client)
}

/// Fetches a vector of all release metadata from the given repository, hosted on the given
/// registry.
pub async fn fetch_releases(
    registry: &Registry,
    repo: &str,
    username: Option<&str>,
    password: Option<&str>,
    cache: cache::Cache,
    manifestref_key: &str,
    concurrency: usize,
) -> Result<Vec<cincinnati::plugins::internal::graph_builder::release::Release>, Error> {
    let registry_client = new_registry_client(registry, repo, username, password).await?;

    let registry_client_get_tags = registry_client.clone();
    let tags = Box::pin(get_tags(repo, &registry_client_get_tags).await);

    let releases = {
        let estimated_releases = match tags.size_hint() {
            (_, Some(upper)) => upper,
            (lower, None) => lower,
        };
        Arc::new(FuturesMutex::new(Vec::with_capacity(estimated_releases)))
    };

    tags.try_for_each_concurrent(concurrency, |tag| {
        let registry_client = registry_client.clone();
        let cache = cache.clone();
        let releases = releases.clone();

        async move {
            trace!("[{}] Fetching release", tag);
            let (tag, manifest, manifestref) =
                get_manifest_and_ref(tag, repo.to_owned(), &registry_client).await?;

            // Try to read the architecture from the manifest
            let arch = match manifest.architectures() {
                Ok(archs) => {
                    // We don't support ManifestLists now, so we expect only 1
                    // architecture for the given manifest
                    ensure!(
                        archs.len() == 1,
                        "[{}] broke assumption of exactly one architecture per tag: {:?}",
                        tag,
                        archs
                    );
                    archs.first().map(std::string::ToString::to_string)
                }
                Err(e) => {
                    error!(
                        "could not get architecture from manifest for tag {}: {}",
                        tag, e
                    );
                    None
                }
            };

            let layers_digests = manifest
                .layers_digests(arch.as_ref().map(String::as_str))
                .map_err(|e| format_err!("{}", e))
                .context(format!(
                    "[{}] could not get layers_digests from manifest",
                    tag
                ))?
                // Reverse the order to start with the top-most layer
                .into_iter()
                .rev()
                .collect();

            let release = match lookup_or_fetch(
                layers_digests,
                registry_client.to_owned(),
                registry.to_owned(),
                repo.to_owned(),
                tag.to_owned(),
                &cache,
                manifestref.clone(),
                manifestref_key.to_string(),
                arch,
            )
            .await?
            {
                Some(release) => release,
                None => {
                    // Reminder: this means the layer_digests point to layers
                    // without any release and we've cached this before
                    return Ok(());
                }
            };

            releases.lock().await.push(release);

            Ok(())
        }
    })
    .await?;

    let releases = Arc::<
        FuturesMutex<Vec<cincinnati::plugins::internal::graph_builder::release::Release>>,
    >::try_unwrap(releases)
    .map_err(|_| format_err!("Unwrapping the shared Releases vector. This must not fail."))?
    .into_inner();

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
/// is keyed on the manifest reference.
#[allow(clippy::too_many_arguments)]
async fn lookup_or_fetch(
    layer_digests: Vec<String>,
    registry_client: dkregistry::v2::Client,
    registry: Registry,
    repo: String,
    tag: String,
    cache: &cache::Cache,
    manifestref: String,
    manifestref_key: String,
    arch: Option<String>,
) -> Fallible<Option<cincinnati::plugins::internal::graph_builder::release::Release>> {
    let cached_metadata = {
        // Nest the guard in a scope to guarantee that the cache isn't locked when trying to write to it later
        cache.read().await.get(&manifestref).map(Clone::clone)
    };

    let metadata = match cached_metadata {
        Some(cached_metadata) => {
            trace!(
                "[{}] Using cached release metadata for manifestref {}",
                &tag,
                &manifestref
            );
            cached_metadata.clone()
        }
        None => {
            let placeholder = Option::from(Metadata {
                kind: MetadataKind::V0,
                version: Version::new(0, 0, 0),
                previous: vec![],
                next: vec![],
                metadata: Default::default(),
            });
            cache.write().await.insert(manifestref.clone(), placeholder);

            let metadata = find_first_release_metadata(
                layer_digests,
                registry_client,
                repo.clone(),
                tag.clone(),
            )
            .await
            .context("failed to find first release")?
            .map(|mut metadata| {
                // Attach the manifestref this release was found in for further processing
                metadata
                    .metadata
                    .insert(manifestref_key, manifestref.clone());

                // Process the manifest architecture if given
                if let Some(arch) = arch {
                    // Encode the architecture as SemVer information
                    metadata.version.build = vec![semver::Identifier::AlphaNumeric(arch.clone())];

                    // Attach the architecture for later processing
                    metadata
                        .metadata
                        .insert("io.openshift.upgrades.graph.release.arch".to_owned(), arch);
                };

                metadata
            });

            trace!("[{}] Caching release metadata", &tag);
            cache
                .write()
                .await
                .insert(manifestref.clone(), metadata.clone());

            metadata
        }
    };

    Ok(metadata.map(|metadata| {
        let source = format_release_source(&registry, &repo, &manifestref);
        cincinnati::plugins::internal::graph_builder::release::Release { source, metadata }
    }))
}

// Get a stream of tags
async fn get_tags<'a, 'b: 'a>(
    repo: &'b str,
    registry_client: &'b dkregistry::v2::Client,
) -> impl TryStreamExt<Item = Fallible<String>> + 'a {
    registry_client
        // According to https://docs.docker.com/registry/spec/api/#listing-image-tags
        // the tags should be ordered lexically but they aren't
        .get_tags(repo, Some(20))
        .map_err(|e| format_err!("{}", e))
}

async fn get_manifest_and_ref(
    tag: String,
    repo: String,
    registry_client: &dkregistry::v2::Client,
) -> Result<(String, dkregistry::v2::manifest::Manifest, String), Error> {
    trace!("[{}] Processing {}", &tag, &repo);
    let (manifest, manifestref) = registry_client
        .get_manifest_and_ref(&repo, &tag)
        .map_err(|e| format_err!("{}", e))
        .await?;

    let manifestref =
        manifestref.ok_or_else(|| format_err!("no manifestref found for {}:{}", &repo, &tag))?;

    Ok((tag, manifest, manifestref))
}

fn format_release_source(registry: &Registry, repo: &str, manifestref: &str) -> String {
    format!("{}/{}@{}", registry.host_port_string(), repo, manifestref)
}

async fn find_first_release_metadata(
    layer_digests: Vec<String>,
    registry_client: dkregistry::v2::Client,
    repo: String,
    tag: String,
) -> Fallible<Option<Metadata>> {
    for layer_digest in layer_digests {
        trace!("[{}] Downloading layer {}", &tag, &layer_digest);
        let (repo, tag) = (repo.clone(), tag.clone());

        let blob = registry_client
            .get_blob(&repo, &layer_digest)
            .map_err(|e| format_err!("{}", e))
            .await?;

        let metadata_filename = "release-manifests/release-metadata";

        trace!(
            "[{}] Looking for {} in archive {} with {} bytes",
            &tag,
            &metadata_filename,
            &layer_digest,
            &blob.len(),
        );

        match tokio::task::spawn_blocking(move || assemble_metadata(&blob, metadata_filename))
            .await?
        {
            Ok(metadata) => {
                return Ok(Some(metadata));
            }
            Err(e) => {
                debug!(
                    "[{}] Could not assemble metadata from layer ({}): {}",
                    &tag, &layer_digest, e,
                );
            }
        }
    }

    warn!("[{}] Could not find any release", tag);
    Ok(None)
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
        ];

        for (input, expected) in tests {
            let registry: Registry = Registry::try_from_str(input)
                .unwrap_or_else(|_| panic!("could not parse {} to registry", input));
            assert_eq!(registry, expected);
            assert_eq!(input, registry.host_port_string());
        }
    }
}
