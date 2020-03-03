use crate as cincinnati;

use self::cincinnati::plugins::prelude::*;
use self::cincinnati::plugins::prelude_plugin_impl::*;

mod graph_data_model {
    //! This module contains the data types corresponding to the graph data files.

    use serde::de::Visitor;
    use serde::Deserialize;
    use serde::Deserializer;
    use std::collections::HashMap;
    /// Represents the blocked edges files in the data repository.
    #[derive(Debug, Deserialize)]
    pub struct BlockedEdge {
        pub to: semver::Version,
        pub from: RegexWrapper,
    }

    /// New type used to implement Deserialize for regex::Regex so we can use it in the `BlockedEdge` struct
    #[derive(Debug)]
    pub struct RegexWrapper(regex::Regex);

    impl std::ops::Deref for RegexWrapper {
        type Target = regex::Regex;

        fn deref(&self) -> &Self::Target {
            &self.0
        }
    }

    impl<'de> Deserialize<'de> for RegexWrapper {
        fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
        where
            D: Deserializer<'de>,
        {
            struct RegexVisitor;

            impl<'de> Visitor<'de> for RegexVisitor {
                type Value = RegexWrapper;

                fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
                where
                    E: serde::de::Error,
                {
                    let re = regex::Regex::new(value).map_err(|e| {
                        serde::de::Error::custom(format!("error parsing {} as Regex: {}", value, e))
                    })?;

                    Ok(RegexWrapper(re))
                }

                fn expecting(
                    &self,
                    _: &mut std::fmt::Formatter<'_>,
                ) -> std::result::Result<(), std::fmt::Error> {
                    panic!("impl of Visitor::expecting for RegexVisitor should not be required for deserialization.")
                }
            }

            deserializer.deserialize_str(RegexVisitor)
        }
    }

    /// Represents the channel files in the data repository.
    #[derive(Debug, Deserialize)]
    pub struct Channel {
        pub name: String,
        pub versions: Vec<semver::Version>,
    }

    /// Represents the raw metadata file in the data repository.
    pub type RawMetadata = HashMap<String, HashMap<String, String>>;
}

mod state {
    //! This module contains types to manage the plugin state.

    use std::collections::HashMap;
    use std::sync::Arc;
    use tokio::sync::RwLock as FuturesRwLock;

    #[derive(Debug, Default)]
    pub struct StateData {
        sha: Option<String>,
        metadata: HashMap<String, String>,
    }

    pub type State = Arc<FuturesRwLock<StateData>>;

    pub fn new() -> State {
        Arc::new(FuturesRwLock::new(Default::default()))
    }
}

pub static DEFAULT_ARCH: &str = "amd64";

/// Plugin settings.
#[derive(Clone, Debug, Deserialize, SmartDefault)]
#[serde(default)]
pub struct OpenshiftSecondaryMetadataParserSettings {
    data_directory: PathBuf,
    key_prefix: String,

    #[default(DEFAULT_ARCH.to_string())]
    default_arch: String,
}

impl OpenshiftSecondaryMetadataParserSettings {
    /// Validate plugin configuration and fill in defaults.
    pub fn deserialize_config(cfg: toml::Value) -> Fallible<Box<dyn PluginSettings>> {
        let settings: Self = cfg.try_into()?;

        ensure!(!settings.key_prefix.is_empty(), "empty key_prefix");
        ensure!(!settings.default_arch.is_empty(), "empty default_arch");

        Ok(Box::new(settings))
    }
}

/// Plugin.
#[derive(Debug)]
pub struct OpenshiftSecondaryMetadataParserPlugin {
    settings: OpenshiftSecondaryMetadataParserSettings,

    // Stores the result of the last run
    state: state::State,
}

impl OpenshiftSecondaryMetadataParserPlugin {
    pub fn new(settings: OpenshiftSecondaryMetadataParserSettings) -> Self {
        Self {
            settings,
            state: state::new(),
        }
    }
}

impl PluginSettings for OpenshiftSecondaryMetadataParserSettings {
    fn build_plugin(&self, _: Option<&prometheus::Registry>) -> Fallible<BoxedPlugin> {
        let plugin = OpenshiftSecondaryMetadataParserPlugin::new(self.clone());
        Ok(new_plugin!(InternalPluginWrapper(plugin)))
    }
}

async fn deserialize_directory_files<T>(
    path: &PathBuf,
    extension_re: regex::Regex,
) -> Fallible<Vec<T>>
where
    T: DeserializeOwned,
{
    use tokio::stream::Stream;
    use tokio::stream::StreamExt;

    let mut paths = tokio::fs::read_dir(&path)
        .await
        .context(format!("Reading directory {:?}", &path))?
        .filter_map(|tried_direntry| match tried_direntry {
            Ok(direntry) => Some(direntry),
            Err(e) => {
                warn!("{}", e);
                None
            }
        })
        .filter_map(|direntry| {
            let path = direntry.path();
            if let Some(extension) = &path.extension() {
                if extension_re.is_match(extension.to_str().unwrap_or_default()) {
                    Some(path)
                } else {
                    None
                }
            } else {
                debug!("{:?} does not have an extension", &path);
                None
            }
        });

    let mut t_vec: Vec<T> = Vec::with_capacity(match paths.size_hint() {
        (_, Some(upper)) => upper,
        (lower, None) => lower,
    });

    while let Some(path) = paths.next().await {
        match tokio::fs::read(&path).await {
            Ok(yaml) => match serde_yaml::from_slice(&yaml) {
                Ok(value) => t_vec.push(value),
                Err(e) => {
                    warn!("Failed to deserialize file at {:?}: {}", &path, e);
                }
            },
            Err(e) => {
                warn!("Couldn't read file {:?}: {}", &path, e);
            }
        }
    }

    Ok(t_vec)
}

impl OpenshiftSecondaryMetadataParserPlugin {
    pub(crate) const PLUGIN_NAME: &'static str = "openshift-secondary-metadata-parse";

    async fn process_raw_metadata(&self, io: &mut InternalIO) -> Fallible<()> {
        let path = self.settings.data_directory.join("raw/metadata.json");

        let json = tokio::fs::read(&path)
            .await
            .context(format!("Reading {:?}", &path))?;
        let raw_metadata = serde_json::from_slice::<graph_data_model::RawMetadata>(&json).context(
            format!("Deserializing the content of {:?} to RawMetadata", &path),
        )?;
        debug!("Found {} raw metadata entries", raw_metadata.len());

        raw_metadata.iter().for_each(|(version, metadata)| {
            metadata.iter().for_each(|(key, value)| {
                io.graph.find_by_fn_mut(|release| {
                    let release_semver = semver::Version::from_str(release.version())
                        .context(format!("Parsing {} as SemVer", release.version()));
                    if let Err(e) = &release_semver {
                        warn!("{}", e);
                    }

                    let version_semver = semver::Version::from_str(version)
                        .context(format!("Parsing {} as SemVer", &version));
                    if let Err(e) = &version_semver {
                        warn!("{}", e);
                    }

                    match (release_semver, version_semver) {
                        (Ok(release_semver), Ok(version_semver))
                            if release_semver == version_semver =>
                        {
                            release.get_metadata_mut().map(|metadata| {
                                metadata
                                    .entry((*key).to_string())
                                    .and_modify(|previous_add| {
                                        *previous_add += &format!(",{}", &value)
                                    })
                                    .or_insert_with(|| (*value).to_string())
                            });
                            true
                        }
                        _ => false,
                    }
                });
            })
        });

        Ok(())
    }

    async fn process_blocked_edges(&self, io: &mut InternalIO) -> Fallible<()> {
        let blocked_edges_dir = self.settings.data_directory.join("blocked-edges");
        let blocked_edges: Vec<graph_data_model::BlockedEdge> =
            deserialize_directory_files(&blocked_edges_dir, regex::Regex::new("ya+ml")?)
                .await
                .context(format!(
                    "Reading blocked edges from {:?}",
                    blocked_edges_dir
                ))?;
        debug!(
            "Found {} valid blocked edges declarations.",
            blocked_edges.len()
        );

        blocked_edges
            .into_iter()
            .try_for_each(|blocked_edge| -> Fallible<()> {
                let mut to = blocked_edge.to;

                // add build information to match architecture
                if to.build.is_empty() {
                    let arch =
                        // Special case for a few cases where the "s390x" arch was encoded with '-' instead of '+'
                        if to.pre == vec![semver::Identifier::AlphaNumeric("s390x".to_string())] {
                            "s390x"
                        } else {
                            &self.settings.default_arch
                        };

                    warn!("Adding architecture {} to {:?}", &arch, &to);
                    to.build
                        .push(semver::Identifier::AlphaNumeric(arch.to_string()));
                };

                // find version in the graph
                let release_id = match io.graph.find_by_version(&to.to_string()) {
                    Some(release_id) => release_id,
                    None => {
                        warn!("Release with version {} not found in graph", to);
                        return Ok(());
                    }
                };

                // add metadata to block edge using the `previous.remove_regex` metadata
                io.graph
                    .get_metadata_as_ref_mut(&release_id)
                    .context(format!(
                        "[blocked_edges] Getting mutable metadata for {}",
                        &to.to_string()
                    ))?
                    .insert(
                        format!("{}.{}", self.settings.key_prefix, "previous.remove_regex"),
                        blocked_edge.from.to_string(),
                    );

                Ok(())
            })?;

        Ok(())
    }

    async fn process_channels(&self, io: &mut InternalIO) -> Fallible<()> {
        let channels_dir = self.settings.data_directory.join("channels");
        let channels: Vec<graph_data_model::Channel> =
            deserialize_directory_files(&channels_dir, regex::Regex::new("ya+ml")?)
                .await
                .context(format!("Reading channels from {:?}", channels_dir))?;
        debug!("Found {} valid channel declarations.", channels.len());

        let channels_key = format!("{}.release.channels", self.settings.key_prefix);
        channels.into_iter().for_each(|channel|
        // Find out for each channel
        {
            let versions_in_channel = channel
                .versions
                .iter()
                .collect::<Vec<&semver::Version>>();

            let releases_in_channel = io.graph.find_by_fn_mut(|release| {
                let release_semver = match semver::Version::from_str(release.version())
                    .context(format!("Parsing {} as SemVer", release.version()))
                {
                    Ok(semver) => semver,
                    Err(e) => {
                        warn!("{}", e);
                        return false;
                    }
                };

                versions_in_channel.iter().any(|release_in_channel| {
                    let release_eq = *release_in_channel == &release_semver;

                    // Comparing semver::Version is not enough because it disregards the build information.
                    let build_eq = release_in_channel.build.is_empty() ||
                    release_in_channel.build == release_semver.build;

                    release_eq && build_eq
                })
            });

            for (release_id, version) in releases_in_channel {
                let metadata = match io
                    .graph
                    .get_metadata_as_ref_mut(&release_id)
                    .context(format!(
                        "[channels] Getting mutable metadata for {}",
                        &version
                    )) {
                    Ok(metadata) => metadata,
                    Err(e) => {
                        warn!("{}", e);
                        continue;
                    }
                };

                metadata
                    .entry(channels_key.clone())
                    .and_modify(|channels_value| {
                        channels_value.extend(format!(",{}", &channel.name).chars());
                    })
                    .or_insert_with(|| channel.name.clone());
            }
        });

        // Sort the channels as some tests and consumers might already depend on
        // the sorted output which existed in the hack util which is replaced by this plugin.
        let sorted_releases = io.graph.find_by_fn_mut(|release| {
            release
                .get_metadata_mut()
                .map(|metadata| {
                    metadata.entry(channels_key.clone()).and_modify(|channels| {
                        let mut channels_split = channels.split(',').collect::<Vec<_>>();
                        // this has to match the sorting at
                        // https://github.com/openshift/cincinnati-graph-data/blob/5fc8dd0825b42369de8070ecba2ae0c49d0a99d9/hack/graph-util.py#L187
                        channels_split.sort_by(|a, b| a.cmp(b));
                        channels_split.sort_by(|a, b| {
                            let a_split: Vec<&str> = a.splitn(2, '-').collect();
                            let b_split: Vec<&str> = b.splitn(2, '-').collect();
                            a_split[1].cmp(b_split[1])
                        });
                        *channels = channels_split.join(",")
                    })
                })
                .is_some()
        });
        debug!(
            "Sorted channels metadata of {} releases.",
            sorted_releases.len()
        );

        Ok(())
    }
}

#[async_trait]
impl InternalPlugin for OpenshiftSecondaryMetadataParserPlugin {
    async fn run_internal(self: &Self, mut io: InternalIO) -> Fallible<InternalIO> {
        self.process_raw_metadata(&mut io).await?;
        self.process_blocked_edges(&mut io).await?;
        self.process_channels(&mut io).await?;

        Ok(io)
    }
}

#[cfg(test)]
mod tests {
    use crate as cincinnati;

    use self::cincinnati::plugins::InternalIO;
    use self::cincinnati::plugins::InternalPlugin;

    use failure::{Fallible, ResultExt};
    use std::path::PathBuf;
    use std::str::FromStr;

    lazy_static::lazy_static! {
        static ref TEST_FIXTURE_DIR: PathBuf = {
            PathBuf::from_str("src/plugins/internal/graph_builder/openshift_secondary_metadata_parser/test_fixtures").unwrap()
        };
    }

    #[test]
    fn compare_quay_result_fixture_20200220104838() -> Fallible<()> {
        let mut runtime = commons::testing::init_runtime()?;

        let fixture_directory = TEST_FIXTURE_DIR.join("20200220.104838");

        let read_file_to_graph = |filename: &str| -> Fallible<cincinnati::Graph> {
            let path = fixture_directory.join(filename);
            let string =
                std::fs::read_to_string(&path).context(format!("Reading {:?} to string", &path))?;
            serde_json::from_str(&string)
                .context(format!("Deserializing {:?} to Graph", &path))
                .map_err(Into::into)
        };

        // Get the fixture data
        let graph_raw = read_file_to_graph("graph-gb-raw.json")?;
        let graph_with_quay_metadata: cincinnati::Graph =
            read_file_to_graph("graph-gb-with-quay-metadata.json")?;

        // Configure the plugin
        let plugin = Box::new(super::OpenshiftSecondaryMetadataParserPlugin::new(
            toml::from_str(&format!(
                r#"
                    data_directory = {:?}
                    key_prefix = "{}"
                "#,
                &fixture_directory.join("cincinnati-graph-data"),
                cincinnati::plugins::internal::edge_add_remove::DEFAULT_KEY_FILTER,
            ))
            .context("Parsing config string to settings")?,
        ));
        let edge_add_remove_plugin = Box::new(
            cincinnati::plugins::internal::edge_add_remove::EdgeAddRemovePlugin {
                remove_consumed_metadata: true,

                ..Default::default()
            },
        );

        let graph_result = {
            // Run the plugin
            let io = runtime
                .block_on(plugin.run_internal(InternalIO {
                    graph: graph_raw,
                    parameters: Default::default(),
                }))
                .context("Running plugin")?;

            // Run through the EdgeAddRemovePlugin to compare it with the control data
            runtime
                .block_on(edge_add_remove_plugin.run_internal(io))
                .context(
                    "Running plugin result with quay metadata through the EdgeEAddRemovePlugin",
                )?
                .graph
        };

        // Run the graph with quay metadata through the EdgeAddRemovePlugin
        // which will serve as the expected graph
        let graph_expected = {
            runtime
                .block_on(edge_add_remove_plugin.run_internal(InternalIO {
                    graph: graph_with_quay_metadata,
                    parameters: Default::default(),
                }))
                .context(
                    "Running fixture graph with quay metadata through the EdgeEAddRemovePlugin",
                )?
                .graph
        };

        // Sort the graphs for easier readable diffs
        let graph_expected_sorted = {
            let mut graph =
                serde_json::to_value(&graph_expected).context("Sorting expected graph")?;
            commons::testing::sort_json_graph_by_version(&mut graph);
            graph
        };

        let graph_result_sorted = {
            let mut graph = serde_json::to_value(&graph_result).context("Sorting result graph")?;
            commons::testing::sort_json_graph_by_version(&mut graph);
            graph
        };

        pretty_assertions::assert_eq!(graph_result_sorted, graph_expected_sorted);

        Ok(())
    }
}
