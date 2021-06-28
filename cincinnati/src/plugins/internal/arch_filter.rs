//! This plugin can be used to filter a graph by architecture.
//! It reads the requested architecture from the parameters value at key "arch",
//! and the value must match the regex specified at ARCH_VALIDATION_REGEX_STR
//!
//! The filtering also removes any architecture suffixes from the version strings
//! if they are present. The assumption for this is that the architecture would
//! be encoded as part of the _build_ information according to the SemVer specification.

use crate as cincinnati;

use self::cincinnati::plugins::prelude::*;
use self::cincinnati::plugins::prelude_plugin_impl::*;

use commons::GraphError;
use lazy_static::lazy_static;

pub static DEFAULT_KEY_FILTER: &str = "io.openshift.upgrades.graph";
pub static DEFAULT_ARCH_KEY: &str = "release.arch";
pub static DEFAULT_DEFAULT_ARCH: &str = "amd64";
pub static DEFAULT_DEFAULT_ARCH_THRESHOLD_VERSION: &str = "4.2.0-rc.0";

#[derive(Clone, Debug, Deserialize, SmartDefault)]
#[serde(default)]
pub struct ArchFilterPlugin {
    #[default(DEFAULT_KEY_FILTER.to_string())]
    pub key_prefix: String,

    #[default(DEFAULT_ARCH_KEY.to_string())]
    pub key_suffix: String,

    #[default(DEFAULT_DEFAULT_ARCH.to_string())]
    pub default_arch: String,
}

impl PluginSettings for ArchFilterPlugin {
    fn build_plugin(&self, _: Option<&prometheus::Registry>) -> Fallible<BoxedPlugin> {
        Ok(new_plugin!(InternalPluginWrapper(self.clone())))
    }
}

impl ArchFilterPlugin {
    /// Plugin name, for configuration.
    pub const PLUGIN_NAME: &'static str = "arch-filter";

    /// Validate plugin configuration and fill in defaults.
    pub fn deserialize_config(cfg: toml::Value) -> Fallible<Box<dyn PluginSettings>> {
        let plugin: Self = cfg.try_into()?;

        ensure!(!plugin.key_prefix.is_empty(), "empty arch-key prefix");
        ensure!(!plugin.key_suffix.is_empty(), "empty arch-key suffix");

        Ok(Box::new(plugin))
    }
}

/// Evaluate an architecture from the given "arch" parameters.
fn infer_arch(arch: Option<String>, default_arch: String) -> Result<String, GraphError> {
    match arch {
        Some(arch) => {
            if !ARCH_VALIDATION_REGEX_RE.is_match(&arch) {
                return Err(GraphError::InvalidParams(format!(
                    "arch '{}' does not match regex '{}'",
                    arch, ARCH_VALIDATION_REGEX_STR
                )));
            };
            Ok(arch.to_string())
        }
        None => {
            debug!(
                "no architecture given. assuming the default {}",
                default_arch
            );

            Ok(default_arch)
        }
    }
}

/// Regex for arch label validation.
static ARCH_VALIDATION_REGEX_STR: &str = r"^[0-9a-z]+$";

lazy_static! {
    static ref ARCH_VALIDATION_REGEX_RE: regex::Regex =
        regex::Regex::new(&ARCH_VALIDATION_REGEX_STR).expect("could not create regex");
}

#[async_trait]
impl InternalPlugin for ArchFilterPlugin {
    const PLUGIN_NAME: &'static str = Self::PLUGIN_NAME;

    async fn run_internal(self: &Self, internal_io: InternalIO) -> Fallible<InternalIO> {
        let arch = infer_arch(
            internal_io.parameters.get("arch").map(|s| s.to_string()),
            self.default_arch.clone(),
        )?;

        let mut graph = internal_io.graph;

        // iterate over all releases attempt to remove the arch metadata key
        // 1. if it exists, keep every release which matches the given `arch`
        // 2. collect all other releases to be removed
        let to_remove = {
            graph
                .find_by_fn_mut(|release| {
                    match release {
                        cincinnati::Release::Concrete(concrete_release) => concrete_release
                            .metadata
                            .remove(&format!("{}.{}", self.key_prefix, self.key_suffix))
                            .map_or(true, |values| {
                                !values.split(',').any(|value| value.trim() == arch)
                            }),
                        // remove if it's not a ConcreteRelease
                        _ => true,
                    }
                })
                .into_iter()
                .map(|(release_id, version)| {
                    trace!("queuing '{}' for removal", version);
                    release_id
                })
                .collect()
        };

        // remove all matches from the Graph
        let removed = graph.remove_releases(to_remove);

        trace!("removed {} releases", removed);

        // remove the build suffix from the version
        graph
            .iter_releases_mut(|mut release| {
                let version = {
                    let release_version = release.version().to_owned();

                    semver::Version::parse(&release_version)
                        .context(release_version.clone())
                        .map(|mut version| {
                            version.build.retain(|elem| elem.to_string() != arch);
                            trace!("rewriting version {} ->  {}", release_version, version);
                            version.to_string()
                        })?
                };

                match &mut release {
                    cincinnati::Release::Abstract(release) => release.version = version,
                    cincinnati::Release::Concrete(release) => release.version = version,
                };

                Ok(())
            })
            .map_err(|e| GraphError::ArchVersionError(e.to_string()))?;

        Ok(InternalIO {
            graph,
            parameters: internal_io.parameters,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cincinnati::testing::generate_custom_graph;
    use cincinnati::testing::TestMetadata;
    use commons::testing::init_runtime;

    #[test]
    fn plugin_filters_by_arch_and_strips_suffixes() -> Fallible<()> {
        let runtime = init_runtime()?;

        let input_metadata: TestMetadata = vec![
            (
                0,
                [
                    (String::from("version_suffix"), String::from("+amd64")),
                    (String::from("release.arch"), String::from("amd64")),
                ]
                .iter()
                .cloned()
                .collect(),
            ),
            (
                1,
                [
                    (String::from("version_suffix"), String::from("+amd64")),
                    (String::from("release.arch"), String::from("amd64")),
                ]
                .iter()
                .cloned()
                .collect(),
            ),
            (
                2,
                [
                    (String::from("version_suffix"), String::from("+arm64")),
                    (String::from("release.arch"), String::from("arm64")),
                ]
                .iter()
                .cloned()
                .collect(),
            ),
            (
                3,
                [
                    (String::from("version_suffix"), String::from("+arm64")),
                    (String::from("release.arch"), String::from("arm64")),
                ]
                .iter()
                .cloned()
                .collect(),
            ),
        ];
        let input_edges = Some(vec![(0, 1), (2, 3)]);
        let input_graph: cincinnati::Graph =
            generate_custom_graph("image", input_metadata.clone(), input_edges.to_owned());

        // filter by arm64
        let expected_metadata: TestMetadata = vec![
            (2, [].iter().cloned().collect()),
            (3, [].iter().cloned().collect()),
        ];
        let expected_edges = Some(vec![(0, 1)]);

        let expected_graph: cincinnati::Graph =
            generate_custom_graph("image", expected_metadata, expected_edges.to_owned());

        let plugin = Box::new(ArchFilterPlugin {
            key_prefix: "release".to_string(),
            key_suffix: "arch".to_string(),
            default_arch: "amd64".to_string(),
        });
        let future_processed_graph = plugin.run_internal(InternalIO {
            graph: input_graph.clone(),
            parameters: [("arch", "arm64")]
                .iter()
                .map(|(k, v)| (k.to_string(), v.to_string()))
                .collect(),
        });

        let processed_graph = runtime.block_on(future_processed_graph)?.graph;

        assert_eq!(expected_graph, processed_graph);

        Ok(())
    }

    #[test]
    fn ensure_infer_arch() -> Fallible<()> {
        // (arch, default_arch), expecteded_arch
        [
            ((None, "amd64"), "amd64"),
            ((Some("arm64"), "amd64"), "arm64"),
            ((None, "amd64"), "amd64"),
            ((Some("arm64"), "amd64"), "arm64"),
        ]
        .iter()
        .try_for_each(|(params, expected_arch)| -> Fallible<()> {
            assert_eq!(
                infer_arch(params.0.map(str::to_string), params.1.to_string(),)?,
                expected_arch.to_string()
            );

            Ok(())
        })?;

        Ok(())
    }
}
