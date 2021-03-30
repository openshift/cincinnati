use crate as cincinnati;
use crate::plugins::internal::dkrv2_openshift_secondary_metadata_scraper::gpg;
use crate::plugins::internal::release_scrape_dockerv2::registry;
use reqwest::{Client, ClientBuilder};
use std::path::{Path, PathBuf};
use std::time::Duration;
use tempfile::TempDir;
use url::Url;

use self::cincinnati::plugins::prelude::*;
use self::cincinnati::plugins::prelude_plugin_impl::*;

use tokio::sync::Mutex as FuturesMutex;

pub static DEFAULT_OUTPUT_WHITELIST: &[&str] = &[
    "channels/.+\\.ya+ml",
    "blocked-edges/.+\\.ya+ml",
    "raw/metadata.json",
    "version",
];

pub static DEFAULT_METADATA_IMAGE_REGISTRY: &str = "";
pub static DEFAULT_METADATA_IMAGE_REPOSITORY: &str = "";
pub static DEFAULT_METADATA_IMAGE_TAG: &str = "latest";
pub static DEFAULT_SIGNATURE_BASEURL: &str =
    "https://mirror.openshift.com/pub/openshift-v4/signatures/openshift/release/";
pub static DEFAULT_SIGNATURE_FETCH_TIMEOUT_SECS: u64 = 30;

// Defines the key for placing the data directory path in the IO parameters
pub static GRAPH_DATA_DIR_PARAM_KEY: &str = "io.openshift.upgrades.secondary_metadata.directory";

/// Plugin settings.
#[derive(Debug, SmartDefault, Clone, Deserialize)]
#[serde(default)]
pub struct DkrV2OpenshiftSecondaryMetadataScraperSettings {
    /// Directory where the image will be unpacked. Will be created if it doesn't exist.
    output_directory: PathBuf,

    /// Vector of regular expressions used as a positive output filter.
    /// An empty vector is regarded as a configuration error.
    #[default(DEFAULT_OUTPUT_WHITELIST.iter().map(|s| (*s).to_string()).collect())]
    output_allowlist: Vec<String>,

    /// The image registry.
    #[default(DEFAULT_METADATA_IMAGE_REGISTRY.to_string())]
    registry: String,

    /// The image repository.
    #[default(DEFAULT_METADATA_IMAGE_REPOSITORY.to_string())]
    repository: String,

    /// The image tag.
    #[default(DEFAULT_METADATA_IMAGE_TAG.to_string())]
    tag: String,

    /// Username for authenticating with the registry
    #[default(Option::None)]
    username: Option<String>,

    /// Password for authenticating with the registry
    #[default(Option::None)]
    password: Option<String>,

    /// File containing the credentials for authenticating with the registry.
    /// Takes precedence over username and password
    #[default(Option::None)]
    credentials_path: Option<PathBuf>,

    /// Ensure signatures are verified
    #[default(false)]
    verify_signature: bool,

    /// Base URL for signature verification
    #[default(DEFAULT_SIGNATURE_BASEURL.to_string())]
    signature_baseurl: String,

    /// Public keys for signature verification
    #[default(Option::None)]
    public_keys_path: Option<PathBuf>,
}

impl DkrV2OpenshiftSecondaryMetadataScraperSettings {
    /// Validate plugin configuration and fill in defaults.
    pub fn deserialize_config(cfg: toml::Value) -> Fallible<Box<dyn PluginSettings>> {
        let mut settings: Self = cfg
            .clone()
            .try_into()
            .context(format!("Deserializing {:#?}", &cfg))?;

        ensure!(
            !settings
                .output_directory
                .to_str()
                .unwrap_or_default()
                .is_empty(),
            "empty output_directory"
        );

        ensure!(
            !settings.output_allowlist.is_empty(),
            "empty output_allowlist"
        );

        ensure!(!settings.registry.is_empty(), "empty registry");
        ensure!(!settings.repository.is_empty(), "empty repository");
        ensure!(!settings.tag.is_empty(), "empty tag");

        if let Some(credentials_path) = &settings.credentials_path {
            if credentials_path == &PathBuf::from("") {
                warn!("Settings contain an empty credentials path, setting to None");
                settings.credentials_path = None;
            }
        }

        if settings.verify_signature {
            ensure!(
                !settings.signature_baseurl.is_empty(),
                "empty signature base url",
            );
            ensure!(
                Url::parse(settings.signature_baseurl.as_str()).is_ok(),
                "invalid signature base url",
            );
            ensure!(
                !settings.public_keys_path.is_none(),
                "empty public keys path",
            );
        }

        Ok(Box::new(settings))
    }
}

#[derive(Debug, Default)]
pub struct State {
    cached_layers: Option<Vec<String>>,
    cached_data_dir: Option<TempDir>,
}

/// This plugin implements downloading the secondary metadata container image
/// from a given registry/repository location that is compatible with the Docker
/// V2 protocol.
#[derive(Debug)]
pub struct DkrV2OpenshiftSecondaryMetadataScraperPlugin {
    settings: DkrV2OpenshiftSecondaryMetadataScraperSettings,
    output_allowlist: Vec<regex::Regex>,
    data_dir: TempDir,
    state: FuturesMutex<State>,
    http_client: Client,
    registry: registry::Registry,
}

impl DkrV2OpenshiftSecondaryMetadataScraperPlugin {
    pub(crate) const PLUGIN_NAME: &'static str = "dkrv2-secondary-metadata-scrape";

    /// Instantiate a new instance of `Self`.
    pub fn try_new(mut settings: DkrV2OpenshiftSecondaryMetadataScraperSettings) -> Fallible<Self> {
        let output_allowlist: Vec<regex::Regex> = settings
            .output_allowlist
            .iter()
            .try_fold(
                Vec::with_capacity(settings.output_allowlist.len()),
                |mut acc, cur| -> Fallible<_> {
                    let re = regex::Regex::new(cur)?;
                    acc.push(re);
                    Ok(acc)
                },
            )
            .context("Parsing output allowlist strings as regex")?;

        // Create the output directory if it doesn't exist
        std::fs::create_dir_all(&settings.output_directory).context(format!(
            "Creating directory {:?}",
            &settings.output_directory
        ))?;

        let data_dir = tempfile::tempdir_in(&settings.output_directory)?;

        let registry = registry::Registry::try_from_str(&settings.registry)
            .context(format!("Parsing {} as Registry", &settings.registry))?;

        if let Some(credentials_path) = &settings.credentials_path {
            let (username, password) =
                registry::read_credentials(Some(&credentials_path), &registry.host_port_string())
                    .context(format!(
                    "Reading registry credentials from {:?}",
                    credentials_path
                ))?;

            settings.username = username;
            settings.password = password;
        }
        let http_client = ClientBuilder::new()
            .gzip(true)
            .timeout(Duration::from_secs(DEFAULT_SIGNATURE_FETCH_TIMEOUT_SECS))
            .build()
            .context("Building reqwest client")?;

        Ok(Self {
            settings,
            output_allowlist,
            data_dir,
            http_client,
            registry,
            state: FuturesMutex::new(State::default()),
        })
    }
}

impl PluginSettings for DkrV2OpenshiftSecondaryMetadataScraperSettings {
    fn build_plugin(&self, _: Option<&prometheus::Registry>) -> Fallible<BoxedPlugin> {
        let plugin = DkrV2OpenshiftSecondaryMetadataScraperPlugin::try_new(self.clone())?;
        Ok(new_plugin!(InternalPluginWrapper(plugin)))
    }
}

#[async_trait]
impl InternalPlugin for DkrV2OpenshiftSecondaryMetadataScraperPlugin {
    const PLUGIN_NAME: &'static str = Self::PLUGIN_NAME;

    async fn run_internal(self: &Self, mut io: InternalIO) -> Fallible<InternalIO> {
        let registry_client = registry::new_registry_client(
            &self.registry,
            &self.settings.repository,
            self.settings.username.as_deref(),
            self.settings.password.as_deref(),
        )
        .await?;

        let (manifest, reference) = registry_client
            .get_manifest_and_ref(&self.settings.repository, &self.settings.tag)
            .await?;
        trace!("manifest: {:?}, reference: {:?}", manifest, reference);

        if self.settings.verify_signature {
            let reference = reference.ok_or_else(|| {
                format_err!(
                    "no manifestref found for {}:{}",
                    &self.settings.repository,
                    &self.settings.tag
                )
            })?;

            let public_keys = self.settings.public_keys_path.as_ref().unwrap();
            let base_url = Url::parse(self.settings.signature_baseurl.as_str()).unwrap();

            let keyring = gpg::load_public_keys(&public_keys)?;
            gpg::verify_signatures_for_digest(&self.http_client, &base_url, &keyring, &reference)
                .await?;
        }

        let layers = manifest.layers_digests(None)?;
        trace!("layers: {:?}", &layers);

        if self.are_layers_cached(&layers, &mut io).await? {
            return Ok(io);
        }

        let layers_blobs = {
            use futures::TryStreamExt;
            layers
                .iter()
                .map(|layer| registry_client.get_blob(&self.settings.repository, &layer))
                .collect::<futures::stream::FuturesOrdered<_>>()
                .try_collect::<Vec<_>>()
                .await?
        };

        // wrap the blocking filesystem operations so that they don't block the runtime
        let data_dir = tokio::task::block_in_place(|| async {
            let data_dir = self.create_data_dir(&mut io)?;
            self.unpack_layers(layers_blobs.as_slice(), &data_dir)?;
            self.remove_disallowed_files(&data_dir)?;

            Result::<_, Error>::Ok(data_dir)
        })
        .await?;

        self.update_cache_state(layers, data_dir).await;

        Ok(io)
    }
}

impl DkrV2OpenshiftSecondaryMetadataScraperPlugin {
    async fn are_layers_cached(&self, layers: &[String], io: &mut InternalIO) -> Fallible<bool> {
        let state = self.state.lock().await;
        match (&state.cached_layers, &state.cached_data_dir) {
            (Some(cached_layers), Some(cached_data_dir)) if cached_layers.as_slice() == layers => {
                trace!("Using cached data directory for tag {}", self.settings.tag);
                io.parameters.insert(
                    GRAPH_DATA_DIR_PARAM_KEY.to_string(),
                    cached_data_dir
                        .path()
                        .to_str()
                        .ok_or_else(|| format_err!("data_dir cannot be converted to str"))?
                        .to_string(),
                );
                Ok(true)
            }

            _ => Ok(false),
        }
    }

    fn create_data_dir(&self, io: &mut InternalIO) -> Fallible<TempDir> {
        let data_dir = tempfile::tempdir_in(self.data_dir.path())?;

        io.parameters.insert(
            GRAPH_DATA_DIR_PARAM_KEY.to_string(),
            data_dir
                .path()
                .to_str()
                .ok_or_else(|| format_err!("data_dir cannot be converted to str"))?
                .to_string(),
        );

        trace!(
            "Using data directory {:?} for tag {}",
            data_dir,
            self.settings.tag
        );

        Ok(data_dir)
    }

    fn unpack_layers<P>(&self, layers_blobs: &[Vec<u8>], data_dir: P) -> Fallible<()>
    where
        P: AsRef<Path>,
        P: std::fmt::Debug,
    {
        dkregistry::render::unpack(&layers_blobs, data_dir.as_ref())?;
        trace!(
            "Unpacked {}/{} with {} layers to {:?}",
            self.settings.registry,
            self.settings.repository,
            layers_blobs.len(),
            data_dir,
        );

        Ok(())
    }

    fn remove_disallowed_files<P>(&self, data_dir: P) -> Fallible<()>
    where
        P: AsRef<Path>,
        P: Copy,
    {
        walkdir::WalkDir::new(data_dir)
            .into_iter()
            .collect::<Vec<_>>()
            .into_iter()
            // start removing files from the leave and walk back to the root
            .rev()
            .try_for_each(|entry_result| -> Fallible<()> {
                let entry = entry_result?;
                let path = entry.path();
                let path_stripped = path.strip_prefix(&data_dir)?;
                if let Some(path_stripped_str) = path_stripped.to_str() {
                    if !path_stripped_str.is_empty()
                        && !self
                            .output_allowlist
                            .iter()
                            .any(|re| re.is_match(&path_stripped_str))
                    {
                        let ty = entry.file_type();
                        if ty.is_file() || ty.is_symlink() {
                            trace!("removing file at '{}'", &path_stripped_str);
                            std::fs::remove_file(path)?;
                        } else if ty.is_dir() {
                            let readdir = std::fs::read_dir(path)?;
                            if readdir.count() == 0 {
                                trace!("removing empty directory at '{}'", &path_stripped_str);
                                std::fs::remove_dir(path)?;
                            }
                        }
                    }
                }

                Ok(())
            })
    }

    async fn update_cache_state(&self, layers: Vec<String>, data_dir: TempDir) {
        let mut state = self.state.lock().await;
        state.cached_layers = Some(layers);
        state.cached_data_dir = Some(data_dir);
    }
}

#[cfg(test)]
#[cfg(feature = "test-net")]
mod network_tests {
    use super::*;
    use mockito;
    use std::collections::HashSet;

    #[tokio::test(threaded_scheduler)]
    async fn openshift_secondary_metadata_extraction() -> Fallible<()> {
        let fixtures = PathBuf::from(
            "./src/plugins/internal/graph_builder/dkrv2_openshift_secondary_metadata_scraper/test_fixtures",
        );
        let mut public_keys_path = fixtures.clone();
        public_keys_path.push("public_keys");

        // Prepare mocked signature URL
        let mut signature_path = fixtures.clone();
        signature_path.push("signatures/signature-3");

        let _m = mockito::mock(
            "GET",
            "/sha256=3d8d70c6090d4b843f885c8a0c80d01c5fb78dd7c8d16e20929ffc32a15e2fde/signature-3",
        )
        .with_status(200)
        .with_body_from_file(signature_path.canonicalize()?)
        .create();

        let tmpdir = tempfile::tempdir()?;

        let config = &format!(
            r#"
                registry = "registry.ci.openshift.org"
                repository = "cincinnati-ci-public/cincinnati-graph-data"
                tag = "6420f7fbf3724e1e5e329ae8d1e2985973f60c14"
                output_allowlist = [ {} ]
                output_directory = {:?}
                verify_signature = true
                signature_baseurl = {:?}
                public_keys_path = {:?}
            "#,
            DEFAULT_OUTPUT_WHITELIST
                .iter()
                .map(|s| format!(r#"{:?}"#, s))
                .collect::<Vec<_>>()
                .join(", "),
            &tmpdir.path(),
            mockito::server_url(),
            &public_keys_path.canonicalize()?,
        );

        let settings = DkrV2OpenshiftSecondaryMetadataScraperSettings::deserialize_config(
            toml::Value::from_str(config)?,
        )?;

        debug!("Settings: {:#?}", &settings);

        let plugin = Box::leak(Box::new(settings.build_plugin(None)?));

        let mut data_dirs_counter: std::collections::HashMap<PathBuf, usize> = Default::default();

        for _ in 0..2 {
            let io = tokio::task::spawn({
                plugin.run(cincinnati::plugins::PluginIO::InternalIO(InternalIO {
                    graph: Default::default(),
                    parameters: Default::default(),
                }))
            })
            .await??;

            let regexes = DEFAULT_OUTPUT_WHITELIST
                .iter()
                .map(|s| regex::Regex::new(s).unwrap())
                .collect::<Vec<regex::Regex>>();
            assert!(!regexes.is_empty(), "no regexes compiled");

            let data_dir = if let cincinnati::plugins::PluginIO::InternalIO(iio) = io {
                iio.parameters
                    .get(GRAPH_DATA_DIR_PARAM_KEY)
                    .map(PathBuf::from)
                    .unwrap()
            } else {
                bail!("expected plugin to return InternalIO");
            };

            assert!(
                data_dir.starts_with(tmpdir.path()),
                "ensure the plugin reports a directory which is on our tmpdir"
            );

            let extracted_paths: HashSet<String> = walkdir::WalkDir::new(&data_dir)
                .into_iter()
                .map(Result::unwrap)
                .filter(|entry| entry.file_type().is_file())
                .filter_map(|file| {
                    let path = file.path();
                    path.to_str().map(str::to_owned)
                })
                .collect();
            assert!(!extracted_paths.is_empty(), "no files were extracted");

            // ensure all files match the configured regexes
            extracted_paths.iter().for_each(|path| {
                assert!(
                    regexes.iter().any(|re| re.is_match(&path)),
                    "{} doesn't match any of the regexes: {:#?}",
                    path,
                    regexes
                )
            });

            // ensure every regex matches at least one file
            regexes.iter().for_each(|re| {
                assert!(
                    extracted_paths.iter().any(|path| re.is_match(path)),
                    "regex {} didn't match a file",
                    &re
                );
            });

            data_dirs_counter
                .entry(data_dir)
                .and_modify(|v| *v += 1)
                .or_insert(1);
        }

        assert_eq!(
            data_dirs_counter.len(),
            1,
            "more than one data_dir encountered"
        );
        assert_eq!(
            data_dirs_counter.values().next().unwrap(),
            &2usize,
            "first data_dir was not reused"
        );

        Ok(())
    }
}
