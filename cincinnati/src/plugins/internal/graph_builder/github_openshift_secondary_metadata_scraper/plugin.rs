use super::github_v3;
use std::convert::{TryFrom, TryInto};

use crate as cincinnati;

use self::cincinnati::plugins::prelude::*;
use self::cincinnati::plugins::prelude_plugin_impl::*;

use tokio::sync::Mutex as FuturesMutex;

pub static DEFAULT_OUTPUT_WHITELIST: &[&str] = &[
    "version",
    "channels/.+\\.ya+ml",
    "blocked-edges/.+\\.ya+ml",
    "raw/metadata.json",
];

// Defines the key for placing the data directory path in the IO parameters
pub static GRAPH_DATA_DIR_PARAM_KEY: &str = "io.openshift.upgrades.secondary_metadata.directory";

lazy_static::lazy_static! {
    pub static ref DEFAULT_REFERENCE_BRANCH: Option<String> = Some(String::from("master"));
}

/// Environment variable name for the Oauth token path
pub static GITHUB_SCRAPER_TOKEN_PATH_ENV: &str = "CINCINNATI_GITHUB_SCRAPER_OAUTH_TOKEN_PATH";

static USER_AGENT: &str = "openshift/cincinnati";

/// Models the scrape mode
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "lowercase")]
pub enum Reference {
    Branch(String),
    Revision(String),
}

impl Reference {
    fn get_inner(&self) -> &String {
        match self {
            Self::Branch(s) => s,
            Self::Revision(s) => s,
        }
    }
}

impl TryFrom<(Option<&String>, Option<&String>)> for Reference {
    type Error = Error;

    fn try_from(options: (Option<&String>, Option<&String>)) -> Fallible<Self> {
        let reference = match (options.0, options.1) {
            (Some(branch), Some(revision)) => {
                bail!(
                    "only one of reference_branch or reference_revision can be set. got {:?} and {:?}",
                    branch,
                    revision,
                );
            }
            (None, None) => Reference::Branch(DEFAULT_REFERENCE_BRANCH.clone().unwrap()),
            (Some(branch), None) => Reference::Branch(branch.to_string()),
            (None, Some(revision)) => Reference::Revision(revision.to_string()),
        };

        Ok(reference)
    }
}

/// Plugin settings.
#[derive(Debug, SmartDefault, Clone, Deserialize)]
#[serde(default)]
pub struct GithubOpenshiftSecondaryMetadataScraperSettings {
    github_org: String,
    github_repo: String,
    output_directory: PathBuf,

    /// Defines the reference branch to be scraped.
    reference_branch: Option<String>,

    /// Defines the reference revision to be scraped.
    reference_revision: Option<String>,

    /// Defines the reference to be scraped according to the `Reference` enum.
    ///
    /// For now, this cannot be provided via TOML due to missing support for
    /// deserializing a `toml::Value` to enum newtype variants.
    #[serde(skip)]
    reference: Option<Reference>,

    /// Vector of regular expressions used as a positive output filter.
    /// An empty vector is regarded as a configuration error.
    #[default(DEFAULT_OUTPUT_WHITELIST.iter().map(|s| (*s).to_string()).collect())]
    output_allowlist: Vec<String>,
    oauth_token_path: Option<PathBuf>,
}

impl GithubOpenshiftSecondaryMetadataScraperSettings {
    /// Validate plugin configuration and fill in defaults.
    pub fn deserialize_config(cfg: toml::Value) -> Fallible<Box<dyn PluginSettings>> {
        let mut settings: Self = cfg
            .clone()
            .try_into()
            .context(format!("Deserializing {:#?}", &cfg))?;

        ensure!(!settings.github_org.is_empty(), "empty github_org");
        ensure!(!settings.github_repo.is_empty(), "empty github_repo");

        let reference: Reference = (
            settings.reference_branch.as_ref(),
            settings.reference_revision.as_ref(),
        )
            .try_into()?;
        ensure!(!reference.get_inner().is_empty(), "empty reference");
        settings.reference = Some(reference);

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

        Ok(Box::new(settings))
    }
}

#[derive(Debug, Default)]
pub struct State {
    commit_wanted: Option<github_v3::Commit>,
    commit_completed: Option<github_v3::Commit>,
}

/// Plugin.
#[derive(Debug)]
pub struct GithubOpenshiftSecondaryMetadataScraperPlugin {
    settings: GithubOpenshiftSecondaryMetadataScraperSettings,
    output_allowlist: Vec<regex::Regex>,

    reference: Reference,

    state: FuturesMutex<State>,
    oauth_token: Option<String>,

    client: reqwest::Client,
    data_dir: tempfile::TempDir,
}

impl GithubOpenshiftSecondaryMetadataScraperPlugin {
    pub(crate) const PLUGIN_NAME: &'static str = "github-secondary-metadata-scrape";

    /// Instantiate a new instance of `Self`.
    pub fn try_new(settings: GithubOpenshiftSecondaryMetadataScraperSettings) -> Fallible<Self> {
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

        let oauth_token = (&settings.oauth_token_path)
            .clone()
            .map(|path| {
                std::fs::read_to_string(&path)
                    .context(format!("Reading Oauth token from {:?}", &path))
            })
            .transpose()?
            .map(|token| {
                token
                    .lines()
                    .next()
                    .map(|first_line| first_line.trim().to_owned())
            })
            .flatten();

        // Create the output directory if it doesn't exist
        std::fs::create_dir_all(&settings.output_directory).context(format!(
            "Creating directory {:?}",
            &settings.output_directory
        ))?;

        let data_dir = tempfile::tempdir_in(&settings.output_directory)?;

        Ok(Self {
            reference: settings
                .reference
                .clone()
                .ok_or_else(|| format_err!("settings don't contain a 'reference'"))?,
            settings,
            output_allowlist,
            oauth_token,
            data_dir,

            state: FuturesMutex::new(State::default()),
            client: reqwest::Client::default(),
        })
    }

    /// Lookup the latest commit on the given branch.
    async fn get_commit_wanted_branch(&self, branch_wanted: &str) -> Fallible<github_v3::Commit> {
        let url = github_v3::branches_url(&self.settings.github_org, &self.settings.github_repo);

        trace!("Getting branches from {}", &url);

        let request = {
            let request = self
                .client
                .get(&url)
                .header(reqwest::header::USER_AGENT, USER_AGENT)
                .header(reqwest::header::ACCEPT, "application/vnd.github.v3+json");
            if let Some(token) = &self.oauth_token {
                request.header(reqwest::header::AUTHORIZATION, format!("token {}", token))
            } else {
                request
            }
        };

        let bytes = request
            .send()
            .await
            .context(format!("Getting branches from {}", &url))?
            .bytes()
            .await
            .context(format!("Getting bytes from request to {}", &url))?;

        let json = std::str::from_utf8(&bytes).context("Parsing body as string")?;

        let branches = serde_json::from_str::<Vec<github_v3::Branch>>(&json)
            .context(format!("Parsing {} to Vec<Branch>", &json))?;

        let latest_commit = branches
            .iter()
            .filter_map(|branch| {
                if branch.name == branch_wanted {
                    Some(branch.commit.clone())
                } else {
                    None
                }
            })
            .nth(0)
            .ok_or_else(|| {
                format_err!(format!(
                    "{}/{} does not have branch {}: {:#?}",
                    &self.settings.github_org,
                    &self.settings.github_repo,
                    &branch_wanted,
                    &branches
                ))
            })?;

        trace!(
            "Latest commit on branch {}: {:?}",
            &branch_wanted,
            &latest_commit
        );

        Ok(latest_commit)
    }

    /// Construct a github_v3::Commit from the given revision
    async fn get_commit_wanted_revision(&self, revision: &str) -> github_v3::Commit {
        github_v3::Commit {
            url: github_v3::commit_url(
                &self.settings.github_org,
                &self.settings.github_repo,
                &revision,
            ),
            sha: revision.to_owned(),
        }
    }

    /// Refresh `self.state.commit_wanted` and determine if an update is required.
    async fn refresh_commit_wanted(&self) -> Fallible<bool> {
        let commit_wanted = match &self.reference {
            Reference::Revision(revision) => self.get_commit_wanted_revision(revision).await,
            Reference::Branch(branch) => self.get_commit_wanted_branch(branch).await?,
        };

        let mut state = self.state.lock().await;

        let should_update = match &state.commit_completed {
            Some(commit_completed) => commit_completed != &commit_wanted,
            None => true,
        };

        (*state).commit_wanted = Some(commit_wanted);

        Ok(should_update)
    }

    /// Fetch the tarball for the latest wanted commit and extract it to the output directory.
    async fn download_wanted(&self) -> Fallible<(github_v3::Commit, Box<[u8]>)> {
        let commit_wanted = {
            let state = &self.state.lock().await;
            state
                .commit_wanted
                .clone()
                .ok_or_else(|| format_err!("commit_wanted unset"))?
        };

        let url = github_v3::tarball_url(
            &self.settings.github_org,
            &self.settings.github_repo,
            &commit_wanted,
        );

        trace!("Downloading {:?} from {}", &commit_wanted, &url);
        reqwest::Client::new()
            .get(&url)
            .header(reqwest::header::USER_AGENT, USER_AGENT)
            .header(reqwest::header::ACCEPT, "application/vnd.github.v3.raw")
            .send()
            .await
            .context(format!("Updating from tarball at {}", &url))?
            .bytes()
            .await
            .context(format!(
                "Getting bytes from the request response to {}",
                &url,
            ))
            .map_err(Into::into)
            .map(|bytes| (commit_wanted, bytes.to_vec().into_boxed_slice()))
    }

    /// Extract a given blob to the output directory, adhering to the output allowlist, and finally update the completed commit state.
    async fn extract(&self, commit: github_v3::Commit, bytes: Box<[u8]>) -> Fallible<()> {
        // Use a tempdir as intermediary extraction target, and later rename to the destination
        let tmpdir = tempfile::tempdir_in(&self.settings.output_directory)?;

        {
            let commit = commit.clone();
            let output_allowlist = self.output_allowlist.clone();
            let tmpdir = tmpdir.path().to_owned();

            tokio::task::spawn_blocking(move || -> Fallible<()> {
                use flate2::read::GzDecoder;
                use tar::Archive;

                let mut archive = Archive::new(GzDecoder::new(bytes.as_ref()));

                archive
                    .entries()?
                    .filter_map(move |entry_result| match entry_result {
                        Ok(entry) => {
                            trace!("Processing entry {:?}", &entry.path());
                            Some(entry)
                        }

                        Err(e) => {
                            warn!(
                                "Could not process entry in tarball from commit {:?}: {}",
                                &commit, e
                            );
                            None
                        }
                    })
                    .try_for_each(|mut entry| -> Fallible<_> {
                        let path = entry
                            .path()
                            .context(format!(
                                "Getting path from entry {:?}",
                                &entry.header().clone().path().unwrap_or_default()
                            ))?
                            .to_str()
                            .ok_or_else(|| format_err!("Could not get string from entry"))?
                            .to_owned();
                        trace!("Processing entry with path {:?}", &path);

                        if output_allowlist
                            .iter()
                            .any(|allowlist_regex| allowlist_regex.is_match(&path))
                        {
                            debug!("Unpacking {:?} to {:?}", &path, &tmpdir);
                            entry
                                .unpack_in(&tmpdir)
                                .context(format!("Unpacking {:?} to {:?}", &path, &tmpdir))?;
                        };

                        Ok(())
                    })
            })
            .await??
        };

        {
            // Move all files from the archive specific subdirectory to the output directory.
            let rename_from = tmpdir.path().join(github_v3::archive_entry_directory_name(
                &self.settings.github_org,
                &self.settings.github_repo,
                &commit,
            ));

            // Append a directory for safety reasons, so we don't wipe the given output directory if it already exists
            let rename_to = &self.data_dir;

            // Remove the target directory if it exists
            if tokio::fs::OpenOptions::new()
                .read(true)
                .write(false)
                .create(false)
                .open(&rename_to)
                .await
                .is_ok()
            {
                let msg = format!("Removing pre-existing directory {:?}", &rename_to);
                debug!("{}", &msg);
                tokio::fs::remove_dir_all(&rename_to).await.context(msg)?;
            }

            let msg = format!("Renaming {:?} -> {:?}", &rename_from, &rename_to);

            // Acquire the state lock as we're going to move files from the
            // commit specific directory into the output directory.
            let mut state_guard = self.state.lock().await;

            debug!("{}", &msg);
            tokio::fs::rename(&rename_from, &rename_to)
                .await
                .context(msg)?;

            // Set commit_completed to the one we've extracted.
            state_guard.commit_completed = Some(commit);
        }

        Ok(())
    }
}

impl PluginSettings for GithubOpenshiftSecondaryMetadataScraperSettings {
    fn build_plugin(&self, _: Option<&prometheus::Registry>) -> Fallible<BoxedPlugin> {
        let plugin = GithubOpenshiftSecondaryMetadataScraperPlugin::try_new(self.clone())?;
        Ok(new_plugin!(InternalPluginWrapper(plugin)))
    }
}

#[async_trait]
impl InternalPlugin for GithubOpenshiftSecondaryMetadataScraperPlugin {
    const PLUGIN_NAME: &'static str = Self::PLUGIN_NAME;

    async fn run_internal(self: &Self, mut io: InternalIO) -> Fallible<InternalIO> {
        io.parameters.insert(
            GRAPH_DATA_DIR_PARAM_KEY.to_string(),
            self.data_dir
                .path()
                .to_str()
                .ok_or_else(|| format_err!("data_dir cannot be converted to str"))?
                .to_string(),
        );

        let should_update = self
            .refresh_commit_wanted()
            .await
            .context("Checking for new commit")?;

        if should_update {
            let (commit, blob) = self
                .download_wanted()
                .await
                .context("Downloading tarball")?;
            self.extract(commit, blob)
                .await
                .context("Extracting tarball")?;
        };

        Ok(io)
    }
}

#[cfg(test)]
#[cfg(feature = "test-net")]
mod network_tests {
    use super::*;
    use std::collections::HashSet;
    #[test]
    fn openshift_secondary_metadata_extraction() -> Fallible<()> {
        let runtime = commons::testing::init_runtime()?;

        let tmpdir = tempfile::tempdir()?;

        let oauth_token_path = std::env::var(GITHUB_SCRAPER_TOKEN_PATH_ENV)?;

        let settings = GithubOpenshiftSecondaryMetadataScraperSettings::deserialize_config(
            toml::Value::from_str(&format!(
                r#"
                    github_org = "openshift"
                    github_repo = "cincinnati-graph-data"
                    reference = {{ revision = "6420f7fbf3724e1e5e329ae8d1e2985973f60c14" }}
                    output_allowlist = [ {} ]
                    output_directory = {:?}
                    oauth_token_path = {:?}
                "#,
                DEFAULT_OUTPUT_WHITELIST
                    .iter()
                    .map(|s| format!(r#"{:?}"#, s))
                    .collect::<Vec<_>>()
                    .join(", "),
                &tmpdir.path(),
                oauth_token_path,
            ))?,
        )?;

        debug!("Settings: {:#?}", &settings);

        let plugin = settings.build_plugin(None)?;

        for _ in 0..2 {
            let _ = runtime.block_on(plugin.run(cincinnati::plugins::PluginIO::InternalIO(
                InternalIO {
                    graph: Default::default(),
                    parameters: Default::default(),
                },
            )))?;

            let regexes = DEFAULT_OUTPUT_WHITELIST
                .iter()
                .map(|s| regex::Regex::new(s).unwrap())
                .collect::<Vec<regex::Regex>>();
            assert!(!regexes.is_empty(), "no regexes compiled");

            let extracted_paths: HashSet<String> = walkdir::WalkDir::new(tmpdir.path())
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
        }

        Ok(())
    }
}
