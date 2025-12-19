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
use reqwest::Certificate;
use semver::Version;
use serde::Deserialize;
use serde_json;
use std::fs::File;
use std::io::Read;
use std::iter::Iterator;
use std::path::{Path, PathBuf};
use std::string::String;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tar::Archive;
use tokio::time::sleep;
use url::Url;

use dkregistry::mediatypes::MediaTypes::{ManifestList, ManifestV2S1Signed, ManifestV2S2};
use dkregistry::v2::Client;

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
    pub(crate) namespace: String,
    pub(crate) port: Option<u16>,
}

impl Registry {
    pub fn try_from_str(src: &str) -> Fallible<Self> {
        match parse_url(src) {
            Ok((scheme, host, port, namespace)) => {
                let mut registry = Registry {
                    host,
                    namespace,
                    port,
                    ..Default::default()
                };

                if scheme != "" {
                    registry.insecure = Registry::insecure_scheme(&scheme)?;
                    registry.scheme = scheme;
                }

                Ok(registry)
            }
            Err(e) => bail!("unable to parse registry {}: {}", src, e),
        }
    }

    pub fn try_new(
        scheme: String,
        host: String,
        port: Option<u16>,
        namespace: String,
    ) -> Fallible<Self> {
        Ok(Registry {
            host,
            port,
            namespace,
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

    /// host port string with namespace included in the uri
    pub fn host_port_namespaced_string(&self) -> String {
        format!(
            "{}{}{}{}",
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
            },
            format!("{}", self.namespace),
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
        let mut registry: String = registry_host.to_string();
        // we will not be checking the entire registry_host in credentials with the assumption that
        // the last part after `/` is the repository
        while registry.rfind('/').is_some() {
            match registry.rfind('/') {
                Some(index) => {
                    // Split the string into two parts: before and after the last '/'
                    let (before, _) = registry.split_at(index);

                    debug!("checking namespaced credentials for {}", &registry);
                    let file = File::open(&path).context(format!("could not open '{:?}'", path))?;
                    let creds = dkregistry::get_credentials(file, &registry);
                    if creds.is_ok() {
                        debug!("got credentials for registry '{}'", &registry);
                        return creds.map_err(|e| format_err!("{}", e));
                    }
                    // Update `registry` to be the part before the last '/'
                    registry = before.to_string();
                }
                None => break,
            }
        }
        debug!("getting credentials for {}", &registry);
        let file = File::open(&path).context(format!("could not open '{:?}'", path))?;
        dkregistry::get_credentials(&file, &registry).map_err(|e| format_err!("{}", e))
    })
}

pub async fn new_registry_client(
    registry: &Registry,
    repo: &str,
    username: Option<&str>,
    password: Option<&str>,
    root_certificates: Option<Vec<Certificate>>,
) -> Result<dkregistry::v2::Client, Error> {
    let client = {
        let mut client_builder = dkregistry::v2::Client::configure()
            .registry(&registry.host_port_string())
            .insecure_registry(registry.insecure)
            .accepted_types(Some(vec![
                (ManifestV2S2, None),
                (ManifestV2S1Signed, Some(0.8)),
                (ManifestList, Some(0.5)),
            ]));
        let scope = format!("repository:{}:pull", &repo);

        if root_certificates.is_some() {
            for cert in root_certificates.unwrap() {
                client_builder = client_builder.add_root_certificate(cert);
            }
        }

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

// get the architecture, manifestref and layers_digest for images with tag/digest
async fn get_manifest_layers(
    tag: String,
    repo: &str,
    registry_client: &Client,
) -> Result<(Option<String>, String, Vec<String>), Error> {
    trace!("[{}] Fetching release", tag);
    let (tag, manifest, manifestref) =
        get_manifest_and_ref(tag, repo.to_owned(), &registry_client).await?;

    // Try to read the architecture from the manifest
    let arch = match manifest.architectures() {
        Ok(archs) => {
            if archs.len() == 1 {
                archs.first().map(std::string::ToString::to_string)
            } else {
                Some(String::from("multi"))
            }
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
        .layers_digests(arch.as_deref())
        .map_err(|e| format_err!("{}", e))
        .context(format!(
            "[{}] could not get layers_digests from manifest",
            tag
        ))?
        // Reverse the order to start with the top-most layer
        .into_iter()
        .rev()
        .collect();

    Ok((arch, manifestref, layers_digests))
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
    certificates: Option<Vec<Certificate>>,
) -> Result<Vec<cincinnati::plugins::internal::graph_builder::release::Release>, Error> {
    let registry_client =
        new_registry_client(registry, repo, username, password, certificates).await?;

    let registry_client_get_tags = registry_client.clone();
    let tags = Box::pin(get_tags(repo, &registry_client_get_tags).await);

    let releases = {
        let estimated_releases = match tags.size_hint() {
            (_, Some(upper)) => upper,
            (lower, None) => lower,
        };
        Arc::new(FuturesMutex::new(Vec::with_capacity(estimated_releases)))
    };

    // releases containing signatures that we want to ignore
    let ignored_sig_releases = Arc::new(AtomicUsize::new(0));
    // releases that had some issues and were not included in the graph
    let ignored_releases = Arc::new(AtomicUsize::new(0));
    // images that were retrived after not being found in the cache
    let cache_misses = Arc::new(AtomicUsize::new(0));
    tags.try_for_each_concurrent(concurrency, |tag| {
        let registry_client = registry_client.clone();
        let cache = cache.clone();
        let releases = releases.clone();
        let sig_releases = ignored_sig_releases.clone();
        let skip_releases = ignored_releases.clone();
        let misses = cache_misses.clone();

        async move {
            let (arch, manifestref, mut layers_digests) =
                match get_manifest_layers(tag.to_owned(), &repo, &registry_client).await {
                    Ok(result) => result,
                    Err(e) => {
                        if tag.contains(".sig") {
                            debug!(
                                "encountered a signature for {}:{}: {}, ignoring this image",
                                &repo, &tag, e
                            );
                            sig_releases.fetch_add(1, Ordering::SeqCst);
                            return Ok(());
                        }
                        error!(
                            "fetching manifest and manifestref for {}:{}: {}",
                            &repo, &tag, e
                        );
                        skip_releases.fetch_add(1, Ordering::SeqCst);
                        return Ok(());
                    }
                };

            // if the image is multi arch, we will have to get one image from the manifest list and
            // use its metadata, because manifest lists are just collections of manifests and don't
            // have their own layers with metadata files.
            if arch.as_ref().unwrap() == "multi" {
                let digest = layers_digests
                    .first()
                    .map(std::string::ToString::to_string)
                    .expect(
                        format!("no images referenced in ManifestList ref:{}", manifestref)
                            .as_str(),
                    );
                // TODO: destructured assignments are unstable in current rust, after updating rust
                // change this to (_,_,layers_digests) and remove separate assignment from below.
                let (_ml_arch, _ml_manifestref, ml_layers_digests) =
                    get_manifest_layers(digest, &repo, &registry_client).await?;
                layers_digests = ml_layers_digests;
            }

            let release = match lookup_or_fetch(
                layers_digests,
                registry_client.to_owned(),
                registry.to_owned(),
                repo.to_owned(),
                tag.to_owned(),
                &cache,
                &misses,
                manifestref.clone(),
                manifestref_key.to_string(),
                arch,
            )
            .await?
            {
                Some(mut release) => {
                    let (layer_metadata_arch, registry_manifest_arch) =
                        release.metadata.metadata_arch_id();
                    if layer_metadata_arch == "multi" && registry_manifest_arch != "multi" {
                        // single arch shard of multiarch release. dont want it
                        return Ok(());
                    } else {
                        release
                    }
                }
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

    let ignored_sig_count = ignored_sig_releases.load(Ordering::SeqCst);
    if ignored_sig_count > 0 {
        info!(
            "ignored {} releases containing only signatures",
            ignored_sig_count
        );
    }

    let ignored_release_count = ignored_releases.load(Ordering::SeqCst);
    if ignored_release_count > 0 {
        warn!(
            "ignored {} releases and were not included in graph",
            ignored_release_count
        );
    }

    let miss_count = cache_misses.load(Ordering::SeqCst);
    if miss_count > 0 {
        info!("{} cache misses while fetching releases", miss_count);
    }

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
    cache_misses: &AtomicUsize,
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
            cached_metadata
        }
        None => {
            cache_misses.fetch_add(1, Ordering::SeqCst);
            cache.write().await.insert(manifestref.clone(), None);

            let metadata = match find_first_release_metadata(
                layer_digests,
                registry_client,
                repo.clone(),
                tag.clone(),
            )
            .await
            {
                Ok(Some(mut metadata)) => {
                    // Attach the manifestref this release was found in for further processing
                    metadata
                        .metadata
                        .insert(manifestref_key, manifestref.clone());

                    // Process the manifest architecture if given
                    if let Some(arch) = arch {
                        // Encode the architecture as SemVer information
                        metadata.version.build =
                            vec![semver::Identifier::AlphaNumeric(arch.clone())];

                        // Attach the architecture for later processing
                        metadata
                            .metadata
                            .insert("io.openshift.upgrades.graph.release.arch".to_owned(), arch);
                    };

                    Some(metadata)
                }
                Ok(None) => None,
                Err(e) => {
                    cache.write().await.remove(&manifestref);
                    error!("Failed to find release metadata for {}: {}", &tag, e);
                    return Err(e);
                }
            };

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

    let (manifest, manifestref) = {
        let mut attempts = 0;
        const MAX_ATTEMPTS: u32 = 3;
        const RETRY_DELAY: Duration = Duration::from_secs(1);

        loop {
            attempts += 1;

            match registry_client
                .get_manifest_and_ref(&repo, &tag)
                .map_err(|e| {
                    format_err!(
                        "fetching manifest and manifestref for {}:{}: {}",
                        &repo,
                        &tag,
                        e
                    )
                })
                .await
            {
                Ok(manifest_and_ref) => break manifest_and_ref,
                Err(e) => {
                    // signatures are not identified by dkregistry and not useful for cincinnati graph, dont retry and return error
                    if tag.contains(".sig") {
                        return Err(e);
                    }

                    if attempts >= MAX_ATTEMPTS {
                        return Err(e);
                    }

                    warn!(
                        "getting manifest and manifestref failed (attempt {}/{}): {}, retrying...",
                        attempts, MAX_ATTEMPTS, e
                    );

                    // Wait before retrying
                    sleep(RETRY_DELAY).await;
                }
            }
        }
    };

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

        let blob = {
            let mut attempts = 0;
            const MAX_ATTEMPTS: u32 = 3;
            const RETRY_DELAY: Duration = Duration::from_secs(1);
            loop {
                attempts += 1;
                match registry_client
                    .get_blob(&repo, &layer_digest)
                    .map_err(|e| {
                        format_err!(
                            "fetching blob for repo {} with layer_digest {}: {}",
                            &repo,
                            &layer_digest,
                            e
                        )
                    })
                    .await
                {
                    Ok(layer_blob) => break layer_blob,
                    Err(e) => {
                        if attempts >= MAX_ATTEMPTS {
                            return Err(e);
                        }

                        warn!(
                            "getting blob failed (attempt {}/{}): {}, retrying...",
                            attempts, MAX_ATTEMPTS, e
                        );

                        // Wait before retrying
                        sleep(RETRY_DELAY).await;
                    }
                }
            }
        };

        let metadata_filename = "release-manifests/release-metadata";

        trace!(
            "[{}] Looking for {} in archive {} with {} bytes",
            &tag,
            &metadata_filename,
            &layer_digest,
            &blob.len(),
        );

        match tokio::task::spawn_blocking(move || assemble_metadata(&blob, metadata_filename))
            .await
            .context(format!(
                "[{}] Could not assemble metadata from layer ({})",
                &tag, &layer_digest
            ))? {
            Ok(metadata) => {
                return Ok(Some(metadata));
            }
            Err(e) => {
                return Err(format_err!(
                    "[{}] Could not assemble metadata from layer ({}): {}",
                    &tag,
                    &layer_digest,
                    e
                ))
            }
        }
    }

    warn!("[{}] Could not find any release", tag);
    Ok(None)
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
struct Tags {
    name: String,
    tags: Vec<String>,
}

#[allow(dead_code)]
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
    #[allow(dead_code)]
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

// Parse the url and returns its components
fn parse_url(
    input: &str,
) -> Result<(String, String, Option<u16>, String), Box<dyn std::error::Error>> {
    // Check if the URL has a scheme. If not, prepend "http://" to it.
    // This is done so that the URL library correctly parses the URL.
    let mut modified_input = String::from(input);
    let mut modified_input_flag = false;
    if !input.contains("://") {
        modified_input.insert_str(0, "http://");
        modified_input_flag = true;
    }

    let url = Url::parse(&modified_input).context("parsing registry url")?;

    let mut scheme = url.scheme().to_string();
    let host = url
        .host_str()
        .ok_or("Host is missing from the URL")?
        .to_string();
    let port = url.port();
    let path = url.path().to_string();

    if modified_input_flag {
        scheme = "".to_string();
    }
    Ok((scheme, host, port, path))
}

#[cfg(test)]
mod tests {
    use super::*;

    struct TestCase {
        name: String,
        input: String,
        expected_registry: Registry,
        expected_registry_host_port_string: String,
        expected_registry_host_port_namespaced_string: String,
    }

    fn get_test_cases() -> Vec<TestCase> {
        let mut test_cases = vec![];
        let hosts = vec![
            "127.0.0.1",
            "localhost",
            "quay.io",
            "sat-r220-02.lab.eng.rdu2.redhat.com",
        ];
        for host in hosts {
            test_cases.push(TestCase {
                name: format!("host: {} - no scheme, no port, no path", host),
                input: host.to_string(),
                expected_registry: Registry {
                    scheme: "".to_string(),
                    insecure: false,
                    host: host.to_string(),
                    port: None,
                    namespace: "/".to_string(),
                },
                expected_registry_host_port_string: host.to_string(),
                expected_registry_host_port_namespaced_string: format!("{}/", host),
            });
            test_cases.push(TestCase {
                name: format!("host: {} - http scheme, no port, no path", host),
                input: format!("http://{}", host),
                expected_registry: Registry {
                    scheme: "http".to_string(),
                    insecure: true,
                    host: host.to_string(),
                    port: None,
                    namespace: "/".to_string(),
                },
                expected_registry_host_port_string: format!("http://{}", host),
                expected_registry_host_port_namespaced_string: format!("http://{}/", host),
            });
            test_cases.push(TestCase {
                name: format!("host: {} - https scheme, no port, no path", host),
                input: format!("https://{}", host),
                expected_registry: Registry {
                    scheme: "https".to_string(),
                    insecure: false,
                    host: host.to_string(),
                    port: None,
                    namespace: "/".to_string(),
                },
                expected_registry_host_port_string: host.to_string(),
                expected_registry_host_port_namespaced_string: format!("{}/", host),
            });
            test_cases.push(TestCase {
                name: format!("host: {} - no scheme, 8080 port, no path", host),
                input: format!("{}:8080", host),
                expected_registry: Registry {
                    scheme: "".to_string(),
                    insecure: false,
                    host: host.to_string(),
                    port: Some(8080),
                    namespace: "/".to_string(),
                },
                expected_registry_host_port_string: format!("{}:8080", host),
                expected_registry_host_port_namespaced_string: format!("{}:8080/", host),
            });
            test_cases.push(TestCase {
                name: format!("host: {} - http scheme, 8080 port, no path", host),
                input: format!("http://{}:8080", host),
                expected_registry: Registry {
                    scheme: "http".to_string(),
                    insecure: true,
                    host: host.to_string(),
                    port: Some(8080),
                    namespace: "/".to_string(),
                },
                expected_registry_host_port_string: format!("http://{}:8080", host),
                expected_registry_host_port_namespaced_string: format!("http://{}:8080/", host),
            });
            test_cases.push(TestCase {
                name: format!("host: {} - https scheme, 8080 port, no path", host),
                input: format!("https://{}:8080", host),
                expected_registry: Registry {
                    scheme: "https".to_string(),
                    insecure: false,
                    host: host.to_string(),
                    port: Some(8080),
                    namespace: "/".to_string(),
                },
                expected_registry_host_port_string: format!("{}:8080", host),
                expected_registry_host_port_namespaced_string: format!("{}:8080/", host),
            });
            test_cases.push(TestCase {
                name: format!("host: {} - no scheme, no port, / path", host),
                input: format!("{}/", host),
                expected_registry: Registry {
                    scheme: "".to_string(),
                    insecure: false,
                    host: host.to_string(),
                    port: None,
                    namespace: "/".to_string(),
                },
                expected_registry_host_port_string: host.to_string(),
                expected_registry_host_port_namespaced_string: format!("{}/", host),
            });
            test_cases.push(TestCase {
                name: format!("host: {} - http scheme, no port, / path", host),
                input: format!("http://{}/", host),
                expected_registry: Registry {
                    scheme: "http".to_string(),
                    insecure: true,
                    host: host.to_string(),
                    port: None,
                    namespace: "/".to_string(),
                },
                expected_registry_host_port_string: format!("http://{}", host),
                expected_registry_host_port_namespaced_string: format!("http://{}/", host),
            });
            test_cases.push(TestCase {
                name: format!("host: {} - https scheme, no port, / path", host),
                input: format!("https://{}/", host),
                expected_registry: Registry {
                    scheme: "https".to_string(),
                    insecure: false,
                    host: host.to_string(),
                    port: None,
                    namespace: "/".to_string(),
                },
                expected_registry_host_port_string: host.to_string(),
                expected_registry_host_port_namespaced_string: format!("{}/", host),
            });
            test_cases.push(TestCase {
                name: format!("host: {} - no scheme, 8080 port, / path", host),
                input: format!("{}:8080/", host),
                expected_registry: Registry {
                    scheme: "".to_string(),
                    insecure: false,
                    host: host.to_string(),
                    port: Some(8080),
                    namespace: "/".to_string(),
                },
                expected_registry_host_port_string: format!("{}:8080", host),
                expected_registry_host_port_namespaced_string: format!("{}:8080/", host),
            });
            test_cases.push(TestCase {
                name: format!("host: {} - http scheme, 8080 port, / path", host),
                input: format!("http://{}:8080/", host),
                expected_registry: Registry {
                    scheme: "http".to_string(),
                    insecure: true,
                    host: host.to_string(),
                    port: Some(8080),
                    namespace: "/".to_string(),
                },
                expected_registry_host_port_string: format!("http://{}:8080", host),
                expected_registry_host_port_namespaced_string: format!("http://{}:8080/", host),
            });
            test_cases.push(TestCase {
                name: format!("host: {} - https scheme, 8080 port, / path", host),
                input: format!("https://{}:8080/", host),
                expected_registry: Registry {
                    scheme: "https".to_string(),
                    insecure: false,
                    host: host.to_string(),
                    port: Some(8080),
                    namespace: "/".to_string(),
                },
                expected_registry_host_port_string: format!("{}:8080", host),
                expected_registry_host_port_namespaced_string: format!("{}:8080/", host),
            });
            test_cases.push(TestCase {
                name: format!("host: {} - no scheme, no port, /ns1/ns2 path", host),
                input: format!("{}/ns1/ns2", host),
                expected_registry: Registry {
                    scheme: "".to_string(),
                    insecure: false,
                    host: host.to_string(),
                    port: None,
                    namespace: "/ns1/ns2".to_string(),
                },
                expected_registry_host_port_string: host.to_string(),
                expected_registry_host_port_namespaced_string: format!("{}/ns1/ns2", host),
            });
            test_cases.push(TestCase {
                name: format!("host: {} - http scheme, no port, /ns1/ns2 path", host),
                input: format!("http://{}/ns1/ns2", host),
                expected_registry: Registry {
                    scheme: "http".to_string(),
                    insecure: true,
                    host: host.to_string(),
                    port: None,
                    namespace: "/ns1/ns2".to_string(),
                },
                expected_registry_host_port_string: format!("http://{}", host),
                expected_registry_host_port_namespaced_string: format!("http://{}/ns1/ns2", host),
            });
            test_cases.push(TestCase {
                name: format!("host: {} - https scheme, no port, /ns1/ns2 path", host),
                input: format!("https://{}/ns1/ns2", host),
                expected_registry: Registry {
                    scheme: "https".to_string(),
                    insecure: false,
                    host: host.to_string(),
                    port: None,
                    namespace: "/ns1/ns2".to_string(),
                },
                expected_registry_host_port_string: host.to_string(),
                expected_registry_host_port_namespaced_string: format!("{}/ns1/ns2", host),
            });
            test_cases.push(TestCase {
                name: format!("host: {} - no scheme, 8080 port, /ns1/ns2 path", host),
                input: format!("{}:8080/ns1/ns2", host),
                expected_registry: Registry {
                    scheme: "".to_string(),
                    insecure: false,
                    host: host.to_string(),
                    port: Some(8080),
                    namespace: "/ns1/ns2".to_string(),
                },
                expected_registry_host_port_string: format!("{}:8080", host),
                expected_registry_host_port_namespaced_string: format!("{}:8080/ns1/ns2", host),
            });
            test_cases.push(TestCase {
                name: format!("host: {} - http scheme, 8080 port, /ns1/ns2 path", host),
                input: format!("http://{}:8080/ns1/ns2", host),
                expected_registry: Registry {
                    scheme: "http".to_string(),
                    insecure: true,
                    host: host.to_string(),
                    port: Some(8080),
                    namespace: "/ns1/ns2".to_string(),
                },
                expected_registry_host_port_string: format!("http://{}:8080", host),
                expected_registry_host_port_namespaced_string: format!(
                    "http://{}:8080/ns1/ns2",
                    host
                ),
            });
            test_cases.push(TestCase {
                name: format!("host: {} - https scheme, 8080 port, /ns1/ns2 path", host),
                input: format!("https://{}:8080/ns1/ns2", host),
                expected_registry: Registry {
                    scheme: "https".to_string(),
                    insecure: false,
                    host: host.to_string(),
                    port: Some(8080),
                    namespace: "/ns1/ns2".to_string(),
                },
                expected_registry_host_port_string: format!("{}:8080", host),
                expected_registry_host_port_namespaced_string: format!("{}:8080/ns1/ns2", host),
            });
        }
        test_cases
    }

    #[test]
    fn registry_try_parse_valid() {
        let tests = get_test_cases();
        for test in tests {
            let registry: Registry = Registry::try_from_str(&test.input)
                .unwrap_or_else(|_| panic!("could not parse {} to registry", &test.input));
            assert_eq!(test.expected_registry, registry, "test case: {}", test.name);
        }
    }

    #[test]
    fn registry_host_port_string() {
        let tests = get_test_cases();
        for test in tests {
            let registry: Registry = Registry::try_from_str(&test.input)
                .unwrap_or_else(|_| panic!("could not parse {} to registry", &test.input));
            assert_eq!(
                test.expected_registry_host_port_string,
                registry.host_port_string(),
                "test case: {}",
                test.name
            );
        }
    }

    #[test]
    fn registry_host_port_namespaced_string() {
        let tests = get_test_cases();
        for test in tests {
            let registry: Registry = Registry::try_from_str(&test.input)
                .unwrap_or_else(|_| panic!("could not parse {} to registry", &test.input));
            assert_eq!(
                test.expected_registry_host_port_namespaced_string,
                registry.host_port_namespaced_string(),
                "test case: {}",
                test.name
            );
        }
    }
}
