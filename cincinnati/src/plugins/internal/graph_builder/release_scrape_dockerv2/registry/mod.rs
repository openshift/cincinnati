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

use failure::{bail, ensure, format_err, Error, Fallible, ResultExt};
use flate2::read::GzDecoder;
use futures::lock::Mutex as FuturesMutex;
use futures::prelude::*;
use futures::TryStreamExt;
use log::{debug, error, trace, warn};
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
pub mod release_cache {
    use super::cincinnati::plugins::internal::graph_builder::release::Release;
    use std::collections::HashMap;
    use std::sync::Arc;
    use tokio::sync::RwLock as FuturesRwLock;

    /// The key type of the cache
    type Key = String;

    /// The values of the cache
    type Value = Option<Release>;

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

/// Module for registry cache
pub mod registry_cache {
    use std::collections::HashMap;
    use std::sync::Arc;
    use tokio::sync::RwLock as FuturesRwLock;

    /// The key type of the cache - registry tag
    type Key = String;

    /// The values of the cache - manifestref and the manifest
    type Value = (String, dkregistry::v2::manifest::Manifest);

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

/// Fetches a vector of all release metadata from the given repository, hosted on the given
/// registry.
pub async fn fetch_releases(
    registry: &Registry,
    repo: &str,
    username: Option<&str>,
    password: Option<&str>,
    release_cache: release_cache::Cache,
    registry_cache: registry_cache::Cache,
    manifestref_key: &str,
    concurrency: usize,
) -> Result<Vec<cincinnati::plugins::internal::graph_builder::release::Release>, Error> {
    let authenticated_client = dkregistry::v2::Client::configure()
        .registry(&registry.host_port_string())
        .insecure_registry(registry.insecure)
        .username(username.map(ToString::to_string))
        .password(password.map(ToString::to_string))
        .build()
        .map_err(|e| format_err!("{}", e))?
        .authenticate(format!("repository:{}:pull", &repo))
        .await
        .map_err(|e| format_err!("{}", e))?;

    let authenticated_client_get_tags = authenticated_client.clone();
    let tags = Box::pin(get_tags(repo, &authenticated_client_get_tags).await);

    let releases = {
        let estimated_releases = match tags.size_hint() {
            (_, Some(upper)) => upper,
            (lower, None) => lower,
        };
        Arc::new(FuturesMutex::new(Vec::with_capacity(estimated_releases)))
    };

    tags.try_for_each_concurrent(concurrency, |tag| {
        let authenticated_client = authenticated_client.clone();
        let release_cache = release_cache.clone();
        let registry_cache = registry_cache.clone();
        let releases = releases.clone();

        async move {
            trace!("[{}] Fetching release", tag);
            let (tag, manifest, manifestref) =
                get_manifest_and_ref(tag, repo.to_owned(), registry_cache, &authenticated_client)
                    .await?;

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
                authenticated_client.to_owned(),
                registry.host.to_owned().to_string(),
                repo.to_owned(),
                tag.to_owned(),
                &release_cache,
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
    authenticated_client: dkregistry::v2::Client,
    registry_host: String,
    repo: String,
    tag: String,
    cache: &release_cache::Cache,
    manifestref: String,
    manifestref_key: String,
    arch: Option<String>,
) -> Fallible<Option<cincinnati::plugins::internal::graph_builder::release::Release>> {
    if let Some(release) = cache.read().await.get(&manifestref) {
        trace!(
            "[{}] Using cached release metadata for manifestref {}",
            &tag,
            &manifestref
        );
        return Ok(release.clone());
    }

    let release = find_first_release(
        layer_digests,
        authenticated_client,
        registry_host,
        repo,
        tag.clone(),
        manifestref.clone(),
    )
    .await
    .context("failed to find first release")?
    .map(|mut release| {
        // Attach the manifestref this release was found in for further processing
        release
            .metadata
            .metadata
            .insert(manifestref_key, manifestref.clone());

        // Process the manifest architecture if given
        if let Some(arch) = arch {
            // Encode the architecture as SemVer information
            release.metadata.version.build = vec![semver::Identifier::AlphaNumeric(arch.clone())];

            // Attach the architecture for later processing
            release
                .metadata
                .metadata
                .insert("io.openshift.upgrades.graph.release.arch".to_owned(), arch);
        };

        release
    });

    trace!("[{}] Caching release metadata", &tag);
    cache.write().await.insert(manifestref, release.clone());
    Ok(release)
}

// Get a stream of tags
async fn get_tags<'a, 'b: 'a>(
    repo: &'b str,
    authenticated_client: &'b dkregistry::v2::Client,
) -> impl TryStreamExt<Item = Fallible<String>> + 'a {
    authenticated_client
        // According to https://docs.docker.com/registry/spec/api/#listing-image-tags
        // the tags should be ordered lexically but they aren't
        .get_tags(repo, Some(20))
        .map_err(|e| format_err!("{}", e))
}

async fn get_manifest_and_ref(
    tag: String,
    repo: String,
    cache: registry_cache::Cache,
    authenticated_client: &dkregistry::v2::Client,
) -> Result<(String, dkregistry::v2::manifest::Manifest, String), failure::Error> {
    let (tag, repo) = (tag.clone(), repo.clone());
    trace!("[{}] Processing {}", &tag, &repo);

    let manifestref = authenticated_client
        .get_manifestref(&repo, &tag)
        .map_err(|e| format_err!("{}", e))
        .await?
        .ok_or_else(|| format_err!("no manifestref found for {}:{}", &repo, &tag))?;

    if let Some((cached_manifestref, manifest)) = cache.read().await.get(&tag) {
        if cached_manifestref == &manifestref {
            trace!(
                "[{}] Using cached manifest metadata for manifestref {}",
                &tag,
                &manifestref
            );
            return Ok((tag, (*manifest).clone(), manifestref.clone()));
        }
    }

    let (manifest, _) = authenticated_client
        .get_manifest_and_ref(&repo, &tag)
        .map_err(|e| format_err!("{}", e))
        .await?;

    cache
        .write()
        .await
        .insert(tag.clone(), (manifestref.clone(), manifest.clone()));

    Ok((tag, manifest, manifestref))
}

async fn find_first_release(
    layer_digests: Vec<String>,
    authenticated_client: dkregistry::v2::Client,
    registry_host: String,
    repo: String,
    tag: String,
    manifestref: String,
) -> Fallible<Option<cincinnati::plugins::internal::graph_builder::release::Release>> {
    for layer_digest in layer_digests {
        trace!("[{}] Downloading layer {}", &tag, &layer_digest);
        let (registry_host, repo, tag) = (registry_host.clone(), repo.clone(), tag.clone());

        let blob = authenticated_client
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
                // Specify the source by manifestref
                let source = format!("{}/{}@{}", registry_host, repo, &manifestref);

                return Ok(Some(
                    cincinnati::plugins::internal::graph_builder::release::Release {
                        source,
                        metadata,
                    },
                ));
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
                .unwrap_or_else(|_| panic!("could not parse {} to registry", input));
            assert_eq!(registry, expected);
        }
    }
}

#[cfg(test)]
#[cfg(feature = "test-net")]
mod network_tests;
