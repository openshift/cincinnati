use crate as cincinnati;
use crate::plugins::internal::dkrv2_openshift_secondary_metadata_scraper::gpg;
use crate::plugins::internal::graph_builder::commons::get_certs_from_dir;
use crate::plugins::internal::release_scrape_dockerv2::registry;
use commons::{DEFAULT_ROOT_CERT_DIR, GRAPH_DATA_DIR_PARAM_KEY, SECONDARY_METADATA_PARAM_KEY};
use reqwest::{Certificate, Client, ClientBuilder};
use std::path::{Path, PathBuf};
use std::time::Duration;
use tempfile::TempDir;
use url::Url;

use self::cincinnati::plugins::prelude::*;
use self::cincinnati::plugins::prelude_plugin_impl::*;

use tokio::sync::Mutex as FuturesMutex;

pub static DEFAULT_OUTPUT_ALLOWLIST: &[&str] = &[
    "/LICENSE$",
    "/channels/.+\\.ya+ml$",
    "/blocked-edges/.+\\.ya+ml$",
    "/raw/metadata.json$",
    "/version$",
];

pub static DEFAULT_METADATA_IMAGE_REGISTRY: &str = "";
pub static DEFAULT_METADATA_IMAGE_REPOSITORY: &str = "";
pub static DEFAULT_METADATA_IMAGE_TAG: &str = "latest";
pub static DEFAULT_GRAPH_DATA_PATH: &str = "/";
pub static DEFAULT_SIGNATURE_BASEURL: &str =
    "https://mirror.openshift.com/pub/openshift-v4/signatures/openshift/release/";
pub static DEFAULT_SIGNATURE_FETCH_TIMEOUT_SECS: u64 = 30;

/// Plugin settings.
#[derive(Debug, SmartDefault, Clone, Deserialize)]
#[serde(default)]
pub struct DkrV2OpenshiftSecondaryMetadataScraperSettings {
    /// Directory where the image will be unpacked. Will be created if it doesn't exist.
    output_directory: PathBuf,

    /// Path inside the graph data image in which to find the graph data.
    /// Defaults to "/"
    #[default(DEFAULT_GRAPH_DATA_PATH.into())]
    graph_data_path: PathBuf,

    /// Vector of regular expressions used as a positive output filter.
    /// An empty vector is regarded as a configuration error.
    #[default(DEFAULT_OUTPUT_ALLOWLIST.iter().map(|s| (*s).to_string()).collect())]
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

    /// File containing the root certificates.
    /// Accepts PEM encoded root certificates.
    #[default(PathBuf::from(DEFAULT_ROOT_CERT_DIR.to_string()))]
    root_certificate_dir: PathBuf,

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
                settings.public_keys_path.is_some(),
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
    cached_graph_data_path: Option<PathBuf>,
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

        let mut ns_registry = settings.registry.clone();
        ns_registry.push_str(&format!("/{}", settings.repository));

        let registry = registry::Registry::try_from_str(&ns_registry)
            .context(format!("trying to extract Registry from {}", &ns_registry))?;

        if let Some(credentials_path) = &settings.credentials_path {
            let (username, password) = registry::read_credentials(
                Some(credentials_path),
                &registry.host_port_namespaced_string(),
            )
            .context(format!(
                "Reading registry credentials from {:?}",
                credentials_path
            ))?;

            settings.username = username;
            settings.password = password;
        }

        let mut http_client_builder: reqwest::ClientBuilder = ClientBuilder::new()
            .gzip(true)
            .timeout(Duration::from_secs(DEFAULT_SIGNATURE_FETCH_TIMEOUT_SECS))
            .use_native_tls();

        if settings.root_certificate_dir.exists() {
            let root_certs = get_certs_from_dir(&settings.root_certificate_dir);
            if root_certs.is_err() {
                debug!(
                    "unable to read root certs form dir: {}, {}",
                    &settings.root_certificate_dir.to_str().unwrap_or_default(),
                    root_certs.unwrap_err()
                );
            } else {
                for cert in root_certs.unwrap() {
                    http_client_builder = http_client_builder.add_root_certificate(cert);
                }
            }
        };

        let http_client = http_client_builder
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

    async fn run_internal(&self, mut io: InternalIO) -> Fallible<InternalIO> {
        let mut certificates: Vec<Certificate> = Vec::new();
        if self.settings.root_certificate_dir.exists() {
            let root_certs = get_certs_from_dir(&self.settings.root_certificate_dir);
            if root_certs.is_err() {
                debug!(
                    "unable to read root certs form dir: {}, {}",
                    &self
                        .settings
                        .root_certificate_dir
                        .to_str()
                        .unwrap_or_default(),
                    root_certs.unwrap_err()
                );
            } else {
                certificates = root_certs.unwrap();
            }
        };

        let registry_client = registry::new_registry_client(
            &self.registry,
            &self.settings.repository,
            self.settings.username.as_deref(),
            self.settings.password.as_deref(),
            Some(certificates),
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

            let keyring = gpg::load_public_keys(public_keys)?;
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
                .map(|layer| registry_client.get_blob(&self.settings.repository, layer))
                .collect::<futures::stream::FuturesOrdered<_>>()
                .try_collect::<Vec<_>>()
                .await?
        };

        // wrap the blocking filesystem operations so that they don't block the runtime
        let data_dir = tokio::task::block_in_place(|| async {
            let data_dir = self.create_data_dir(&mut io)?;
            self.unpack_layers(layers_blobs.as_slice(), &data_dir)?;

            Result::<_, Error>::Ok(data_dir)
        })
        .await?;

        let graph_data_dir = data_dir.path().to_path_buf();
        let graph_data_path = PathBuf::from(
            self.settings
                .graph_data_path
                .strip_prefix("/")
                .unwrap_or(&self.settings.graph_data_path),
        );
        let graph_data_path = graph_data_dir.join(graph_data_path);

        self.update_cache_state(layers, data_dir, graph_data_path.to_owned())
            .await;
        self.set_io_graph_data_dir(&mut io, &graph_data_path)?;

        let graph_data_tar_path = self.settings.output_directory.join("graph-data.tar.gz");
        let signatures_path = graph_data_path.as_path().join("signatures");
        let signatures_symlink = self.settings.output_directory.join("signatures");

        // create a symlink to signatures directory for metadata-helper
        // the secondary metadata scraper creates a temp directory to extract graph-data
        // other containers wont have context of the directory that is being used.
        // signatures symlink is at /signatures in the graph-data directory and will keep
        // updating as newer graph-data is scraped.
        if signatures_path
            .try_exists()
            .expect("Can't check if signatures exist")
        {
            tokio::fs::symlink(signatures_path, signatures_symlink).await?;
        }

        commons::create_tar(
            graph_data_tar_path.clone().into_boxed_path(),
            graph_data_path.clone().into(),
        )
        .await
        .context("creating graph-data tar")?;

        io.parameters.insert(
            SECONDARY_METADATA_PARAM_KEY.to_string(),
            graph_data_tar_path
                .to_str()
                .ok_or_else(|| format_err!("secondary_metadata path cannot be converted to str"))?
                .to_string(),
        );

        Ok(io)
    }
}

impl DkrV2OpenshiftSecondaryMetadataScraperPlugin {
    async fn are_layers_cached(&self, layers: &[String], io: &mut InternalIO) -> Fallible<bool> {
        let state = self.state.lock().await;
        match &*state {
            State {
                cached_layers: Some(cached_layers),
                cached_data_dir: Some(_cached_data_dir),
                cached_graph_data_path: Some(cached_graph_data_path),
            } if cached_layers.as_slice() == layers => {
                trace!("Using cached data directory for tag {}", self.settings.tag);
                self.set_io_graph_data_dir(io, cached_graph_data_path)?;
                Ok(true)
            }

            _ => Ok(false),
        }
    }

    fn create_data_dir(&self, io: &mut InternalIO) -> Fallible<TempDir> {
        let data_dir = tempfile::tempdir_in(self.data_dir.path())?;
        self.set_io_graph_data_dir(io, &data_dir)?;

        trace!(
            "Using data directory {:?} for tag {}",
            data_dir,
            self.settings.tag
        );

        Ok(data_dir)
    }

    fn set_io_graph_data_dir<P: AsRef<Path>>(&self, io: &mut InternalIO, dir: P) -> Fallible<()> {
        io.parameters.insert(
            GRAPH_DATA_DIR_PARAM_KEY.to_string(),
            dir.as_ref()
                .to_str()
                .ok_or_else(|| format_err!("data_dir cannot be converted to str"))?
                .to_string(),
        );

        Ok(())
    }

    fn unpack_layers<P>(&self, layers_blobs: &[Vec<u8>], data_dir: P) -> Fallible<()>
    where
        P: AsRef<Path>,
        P: std::fmt::Debug,
    {
        let filter = |path: &Path| {
            if let Some(path_str) = path.to_str() {
                self.output_allowlist.iter().any(|re| re.is_match(path_str))
            } else {
                false
            }
        };

        dkregistry::render::filter_unpack(layers_blobs, data_dir.as_ref(), filter)?;
        trace!(
            "Unpacked {}/{} with {} layers to {:?}",
            self.settings.registry,
            self.settings.repository,
            layers_blobs.len(),
            data_dir,
        );

        Ok(())
    }

    async fn update_cache_state(
        &self,
        layers: Vec<String>,
        data_dir: TempDir,
        graph_data_path: PathBuf,
    ) {
        let mut state = self.state.lock().await;
        state.cached_layers = Some(layers);
        state.cached_data_dir = Some(data_dir);
        state.cached_graph_data_path = Some(graph_data_path);
    }
}

#[cfg(test)]
#[cfg(feature = "test-net")]
mod network_tests {
    use super::*;
    use mockito;
    use std::collections::HashSet;

    #[tokio::test(flavor = "multi_thread")]
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
            DEFAULT_OUTPUT_ALLOWLIST
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

            let regexes = DEFAULT_OUTPUT_ALLOWLIST
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
                    regexes.iter().any(|re| re.is_match(path)),
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
