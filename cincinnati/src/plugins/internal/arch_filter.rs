//! This plugin can be used to filter a graph by architecture.
//! It reads the requested architecture from the parameters value at key "arch",
//! and the value must match the regex specified at ARCH_VALIDATION_REGEX_STR
//!
//! The filtering also removes any architecture suffixes from the version strings
//! if they are present. The assumption for this is that the architecture would
//! be encoded as part of the _build_ information according to the SemVer specification.
use crate::plugins::{
    AsyncIO, BoxedPlugin, InternalIO, InternalPlugin, InternalPluginWrapper, PluginSettings,
};
use commons::GraphError;
use failure::{Fallible, ResultExt};
use futures::Future;
use prometheus::Registry;

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

    #[default(DEFAULT_DEFAULT_ARCH_THRESHOLD_VERSION.to_string())]
    pub default_arch_threshold_version: String,
}

impl PluginSettings for ArchFilterPlugin {
    fn build_plugin(&self, _: Option<&Registry>) -> Fallible<BoxedPlugin> {
        Ok(new_plugin!(InternalPluginWrapper(self.clone())))
    }
}

impl ArchFilterPlugin {
    pub const PLUGIN_NAME: &'static str = "arch-filter";

    /// Validate plugin configuration and fill in defaults.
    pub fn deserialize_config(cfg: toml::Value) -> Fallible<Box<dyn PluginSettings>> {
        let plugin: Self = cfg.try_into()?;

        ensure!(!plugin.key_prefix.is_empty(), "empty arch-key prefix");
        ensure!(!plugin.key_suffix.is_empty(), "empty arch-key suffix");

        Ok(Box::new(plugin))
    }
}

/// Evaluate an architecture from the combination of "arch" and "version" parameters.
///
/// Assuming that version has been sent since the beginning of all versions,
/// this function fails if `arch.is_none() && version > default_arch_threshold_version`.
fn infer_version(
    arch: Option<String>,
    version: Option<String>,
    default_arch: String,
    default_arch_threshold_version: String,
) -> Result<String, GraphError> {
    match (arch, version) {
        (Some(arch), _) => {
            if !ARCH_VALIDATION_REGEX_RE.is_match(&arch) {
                return Err(GraphError::InvalidParams(format!(
                    "arch '{}' does not match regex '{}'",
                    arch, ARCH_VALIDATION_REGEX_STR
                )));
            };
            Ok(arch.to_string())
        }
        (None, Some(version)) => {
            let (semantic_version, threshold_semantic_version) = {
                use std::str::FromStr;
                (
                    semver::Version::from_str(&version)
                        .map_err(|e| GraphError::ArchVersionError(e.to_string()))?,
                    semver::Version::from_str(&default_arch_threshold_version)
                        .map_err(|e| GraphError::ArchVersionError(e.to_string()))?,
                )
            };

            if semantic_version > threshold_semantic_version {
                return Err(GraphError::MissingParams(
                    vec!["arch"].into_iter().map(String::from).collect(),
                ));
            }

            debug!(
                "no architecture given. inferring {} by version {} <= {}",
                default_arch, version, default_arch_threshold_version
            );

            Ok(default_arch)
        }
        (None, None) => Err(GraphError::MissingParams(
            vec!["arch", "version"]
                .into_iter()
                .map(String::from)
                .collect(),
        )),
    }
}

/// Regex for arch label validation.
static ARCH_VALIDATION_REGEX_STR: &str = r"^[0-9a-z]+$";

lazy_static! {
    static ref ARCH_VALIDATION_REGEX_RE: regex::Regex =
        regex::Regex::new(&ARCH_VALIDATION_REGEX_STR).expect("could not create regex");
}

impl InternalPlugin for ArchFilterPlugin {
    fn run_internal(self: &Self, internal_io: InternalIO) -> AsyncIO<InternalIO> {
        let arch = {
            let interesting_params =
                try_get_multiple_values!(internal_io.parameters, "arch", "version");

            infer_version(
                interesting_params.0.map(std::string::ToString::to_string),
                interesting_params.1.map(std::string::ToString::to_string),
                self.default_arch.clone(),
                self.default_arch_threshold_version.clone(),
            )
        }
        .map_err(failure::Error::from);

        let future_result = futures::future::result(arch)
            .join(futures::future::ok::<_, failure::Error>((
                internal_io,
                self.key_prefix.to_owned(),
                self.key_suffix.to_owned(),
            )))
            .and_then(|(arch, (internal_io, key_prefix, key_suffix))| {
                let mut graph = internal_io.graph;

                // iterate over all releases which have the arch metadata set and
                // 1. for every release which matches the given `arch`, rewrite the
                //    arch metadata to only contain `arch`
                // 2. collect all other releases to be removed
                let to_remove = {
                    graph
                        .find_by_fn_mut(|release| {
                            match release {
                                crate::Release::Concrete(concrete_release) => concrete_release
                                    .metadata
                                    .get_mut(&format!("{}.{}", key_prefix, key_suffix))
                                    .map_or(true, |values| {
                                        if values.split(',').any(|value| value.trim() == arch) {
                                            *values = arch.clone();
                                            false
                                        } else {
                                            true
                                        }
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
                            crate::Release::Abstract(release) => release.version = version,
                            crate::Release::Concrete(release) => release.version = version,
                        };

                        Ok(())
                    })
                    .map_err(|e| GraphError::ArchVersionError(e.to_string()))?;

                Ok(InternalIO {
                    graph,
                    parameters: internal_io.parameters,
                })
            })
            .map_err(failure::Error::from);

        Box::new(future_result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate as cincinnati;
    use crate::testing::TestMetadata;
    use cincinnati::testing::generate_custom_graph;
    use commons::testing::init_runtime;

    #[test]
    fn plugin_filters_by_arch_and_strips_suffixes() -> Fallible<()> {
        let mut runtime = init_runtime()?;

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
            (
                2,
                [(String::from("release.arch"), String::from("arm64"))]
                    .iter()
                    .cloned()
                    .collect(),
            ),
            (
                3,
                [(String::from("release.arch"), String::from("arm64"))]
                    .iter()
                    .cloned()
                    .collect(),
            ),
        ];
        let expected_edges = Some(vec![(0, 1)]);

        let expected_graph: cincinnati::Graph =
            generate_custom_graph("image", expected_metadata, expected_edges.to_owned());

        let future_processed_graph = Box::new(ArchFilterPlugin {
            key_prefix: "release".to_string(),
            key_suffix: "arch".to_string(),
            default_arch: "amd64".to_string(),
            default_arch_threshold_version: "1.0.0".to_string(),
        })
        .run_internal(InternalIO {
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
    fn ensure_infer_version() -> Fallible<()> {
        // (arch, version, default_arch, default_arch_threshold), expecteded_arch
        [
            ((None, Some("1.0.0"), "amd64", "2.0.0"), "amd64"),
            ((Some("arm64"), Some("1.0.0"), "amd64", "2.0.0"), "arm64"),
            ((None, Some("2.0.0-rc.0"), "amd64", "2.0.0"), "amd64"),
            (
                (Some("arm64"), Some("2.0.0-rc.0"), "amd64", "2.0.0"),
                "arm64",
            ),
        ]
        .iter()
        .try_for_each(|(params, expected_arch)| -> Fallible<()> {
            assert_eq!(
                infer_version(
                    params.0.map(str::to_string),
                    params.1.map(str::to_string),
                    params.2.to_string(),
                    params.3.to_string()
                )?,
                expected_arch.to_string()
            );

            Ok(())
        })?;

        Ok(())
    }
}
