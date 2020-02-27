use super::github_v3;

use crate as cincinnati;

use self::cincinnati::plugins::prelude::*;
use self::cincinnati::plugins::prelude_plugin_impl::*;

use tokio::sync::Mutex as FuturesMutex;

pub static DEFAULT_OUTPUT_WHITELIST: &[&str] = &[
    "channels/.+\\.ya+ml",
    "blocked-edges/.+\\.ya+ml",
    "raw/metadata.json",
];

/// Environment variable name for the Oauth token path
pub static GITHUB_SCRAPER_TOKEN_PATH_ENV: &str = "CINCINNATI_GITHUB_SCRAPER_OAUTH_TOKEN_PATH";

static USER_AGENT: &str = "openshift/cincinnati";

/// Plugin settings.
#[derive(Debug, SmartDefault, Clone, Deserialize)]
#[serde(default)]
pub struct GithubOpenshiftSecondaryMetadataScraperSettings {
    github_org: String,
    github_repo: String,
    branch: String,
    output_directory: PathBuf,

    /// Vector of regular expressions used as a positive output filter.
    /// An empty vector is regarded as a configuration error.
    #[default(DEFAULT_OUTPUT_WHITELIST.iter().map(|s| (*s).to_string()).collect())]
    output_whitelist: Vec<String>,
    oauth_token_path: Option<PathBuf>,
}

impl GithubOpenshiftSecondaryMetadataScraperSettings {
    /// Validate plugin configuration and fill in defaults.
    pub fn deserialize_config(cfg: toml::Value) -> Fallible<Box<dyn PluginSettings>> {
        let settings: Self = cfg.try_into()?;

        ensure!(!settings.github_org.is_empty(), "empty github_org");
        ensure!(!settings.github_repo.is_empty(), "empty github_repo");
        ensure!(!settings.branch.is_empty(), "empty branch");
        ensure!(
            !settings.output_whitelist.is_empty(),
            "empty output_whitelist"
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
#[derive(Debug, SmartDefault)]
pub struct GithubOpenshiftSecondaryMetadataScraperPlugin {
    settings: GithubOpenshiftSecondaryMetadataScraperSettings,
    output_whitelist: Vec<regex::Regex>,

    #[default(FuturesMutex::new(Default::default()))]
    state: FuturesMutex<State>,
    oauth_token: Option<String>,
}

impl GithubOpenshiftSecondaryMetadataScraperPlugin {
    pub(crate) const PLUGIN_NAME: &'static str = "github-secondary-metadata-scrape";

    /// Instantiate a new instance of `Self`.
    pub fn try_new(settings: GithubOpenshiftSecondaryMetadataScraperSettings) -> Fallible<Self> {
        let output_whitelist: Vec<regex::Regex> = settings
            .output_whitelist
            .iter()
            .try_fold(
                Vec::with_capacity(settings.output_whitelist.len()),
                |mut acc, cur| -> Fallible<_> {
                    let re = regex::Regex::new(cur)?;
                    acc.push(re);
                    Ok(acc)
                },
            )
            .context("Parsing output whitelist strings as regex")?;

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

        Ok(Self {
            settings,
            output_whitelist,
            oauth_token,

            ..Default::default()
        })
    }

    /// Lookup the latest commit on the given branch and update `self.state.commit_wanted`.
    async fn refresh_commit_wanted(&self) -> Fallible<bool> {
        let url = github_v3::branches_url(&self.settings.github_org, &self.settings.github_repo);

        trace!("Getting branches from {}", &url);

        let request = {
            let request = reqwest::Client::new()
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
                if branch.name == self.settings.branch {
                    Some(branch.commit.clone())
                } else {
                    None
                }
            })
            .nth(0)
            .ok_or_else(|| {
                failure::err_msg(format!(
                    "{}/{} does not have branch {}: {:#?}",
                    &self.settings.github_org,
                    &self.settings.github_repo,
                    &self.settings.branch,
                    &branches
                ))
            })?;

        trace!(
            "Latest commit on branch {}: {:?}",
            &self.settings.branch,
            &latest_commit
        );

        let mut state = self.state.lock().await;

        (*state).commit_wanted = Some(latest_commit.clone());

        let should_update = if let Some(commit_completed) = &state.commit_completed {
            commit_completed != &latest_commit
        } else {
            true
        };

        Ok(should_update)
    }

    /// Fetch the tarball for the latest wanted commit and extract it to the output directory.
    async fn download_wanted(&self) -> Fallible<(github_v3::Commit, Box<[u8]>)> {
        let commit_wanted = {
            let state = &self.state.lock().await;
            state
                .commit_wanted
                .clone()
                .ok_or_else(|| failure::err_msg("commit_wanted unset"))?
        };

        let url = github_v3::tarball_url(
            &self.settings.github_org,
            &self.settings.github_repo,
            &commit_wanted,
        );

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

    /// Extract a given blob to the output directory, adhering to the output whitelist, and finally update the completed commit state.
    async fn extract(&self, commit: github_v3::Commit, bytes: Box<[u8]>) -> Fallible<()> {
        // Use a tempdir as intermediary extraction target, and later rename to the destination
        let tmpdir = tempfile::tempdir()?;

        {
            let settings = self.settings.clone();
            let commit = commit.clone();
            let output_whitelist = self.output_whitelist.clone();
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
                            .ok_or_else(|| failure::err_msg("Could not get string from entry"))?
                            .to_owned();
                        trace!("Processing entry with path {:?}", &path);

                        if output_whitelist
                            .iter()
                            .any(|whitelist_regex| whitelist_regex.is_match(&path))
                        {
                            debug!("Unpacking {:?} to {:?}", &path, &settings.output_directory);
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
            let rename_to = &self.settings.output_directory;
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
    async fn run_internal(self: &Self, io: InternalIO) -> Fallible<InternalIO> {
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
        let mut runtime = commons::testing::init_runtime()?;

        let tmpdir = tempfile::tempdir()?;

        let oauth_token_path = std::env::var(GITHUB_SCRAPER_TOKEN_PATH_ENV)?;

        let settings =
            toml::from_str::<GithubOpenshiftSecondaryMetadataScraperSettings>(&format!(
                r#"
                    github_org = "openshift"
                    github_repo = "cincinnati-graph-data"
                    branch = "master"
                    output_whitelist = [ {} ]
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
            ))?;

        debug!("Settings: {:#?}", &settings);

        let plugin = Box::new(GithubOpenshiftSecondaryMetadataScraperPlugin::try_new(
            settings,
        )?);

        let _ = runtime.block_on(plugin.run_internal(InternalIO {
            graph: Default::default(),
            parameters: Default::default(),
        }))?;

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

        Ok(())
    }
}
