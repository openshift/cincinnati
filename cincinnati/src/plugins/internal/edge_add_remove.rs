//! This plugin adds and removes Edges from Nodes based on metadata labels.

use crate as cincinnati;

use self::cincinnati::plugins::prelude::*;
use self::cincinnati::plugins::prelude_plugin_impl::*;

pub static DEFAULT_KEY_FILTER: &str = "io.openshift.upgrades.graph";
pub static DEFAULT_REMOVE_ALL_EDGES_VALUE: &str = "*";

#[derive(Clone, Debug, Deserialize, SmartDefault)]
#[serde(default)]
pub struct EdgeAddRemovePlugin {
    #[default(DEFAULT_KEY_FILTER.to_string())]
    pub key_prefix: String,

    #[default(DEFAULT_REMOVE_ALL_EDGES_VALUE.to_string())]
    pub remove_all_edges_value: String,

    /// If true causes the removal of all processed metadata from the releases.
    #[default(false)]
    pub remove_consumed_metadata: bool,
}

#[async_trait]
impl InternalPlugin for EdgeAddRemovePlugin {
    const PLUGIN_NAME: &'static str = Self::PLUGIN_NAME;

    async fn run_internal(self: &Self, io: InternalIO) -> Fallible<InternalIO> {
        let mut graph = io.graph;
        self.add_edges(&mut graph)?;
        self.remove_edges(&mut graph)?;

        Ok(InternalIO {
            graph,
            parameters: io.parameters,
        })
    }
}

impl PluginSettings for EdgeAddRemovePlugin {
    fn build_plugin(&self, _: Option<&prometheus::Registry>) -> Fallible<BoxedPlugin> {
        Ok(new_plugin!(InternalPluginWrapper(self.clone())))
    }
}

/// Adds and removes next and previous releases specified by metadata.
///
/// The labels are assumed to have the syntax `<prefix>.(previous|next).(remove|add)=(<Version>,)*<Version>`
///
/// # Label processing order
/// The labels are grouped and processed in two separate passes in the following order:
///
/// 1. *.add
///     1. previous
///     2. next
/// 2. *.remove
///     1. previous
///     2. previous_regex
///     3. next
///
/// This ordering has implications on the result of semantical contradictions, so that the `*.remove` labels take precedence over `*.add`.
///
/// # Strictness
/// The plugin aims to gracefully handle any inconsistencies to make the operation as robust as possible.
/// This includes cases where add or remove instructions refer to edges or releases which don't exist in the graph.
impl EdgeAddRemovePlugin {
    /// Plugin name, for configuration.
    pub const PLUGIN_NAME: &'static str = "edge-add-remove";

    /// Validate plugin configuration and fill in defaults.
    pub fn deserialize_config(cfg: toml::Value) -> Fallible<Box<dyn PluginSettings>> {
        let plugin: Self = cfg.try_into()?;

        ensure!(!plugin.key_prefix.is_empty(), "empty prefix");
        ensure!(
            !plugin.remove_all_edges_value.is_empty(),
            "empty value for removing all edges"
        );

        Ok(Box::new(plugin))
    }

    /// Remove next and previous releases specified by metadata.
    ///
    /// The labels are assumed to have the syntax `<prefix>.(previous|next).remove=(<Version>,)*<Version>`
    /// If the value equals a single `REMOVE_ALL_EDGES_VALUE` all edges at the given direction are removed.
    fn remove_edges(&self, mut graph: &mut cincinnati::Graph) -> Fallible<()> {
        macro_rules! handle_remove_edge {
            ($from:ident, $to:ident) => {
                if let Err(e) = graph.remove_edge(&$from, &$to) {
                    if let Some(eae) = e.downcast_ref::<cincinnati::errors::EdgeDoesntExist>() {
                        warn!("{}", eae);
                        continue;
                    };
                    bail!(e)
                };
            };
        }

        let previous_remove_key = format!("{}.{}", self.key_prefix, "previous.remove");
        graph
            .find_by_metadata_key(&previous_remove_key)
            .into_iter()
            .try_for_each(
                |(to, to_version, from_csv): (ReleaseId, String, String)| -> Fallible<()> {
                    if self.remove_consumed_metadata {
                        graph
                            .get_metadata_as_ref_mut(&to)
                            .map(|metadata| metadata.remove(&previous_remove_key))?;
                    }

                    if from_csv.trim() == self.remove_all_edges_value {
                        let parents: Vec<cincinnati::daggy::EdgeIndex> = graph
                            .previous_releases(&to)
                            .map(|(edge_index, _, _)| edge_index)
                            .collect();

                        trace!("removing parents for '{}': {:?}", to_version, parents);
                        return graph.remove_edges_by_index(&parents);
                    }

                    for from_version in from_csv.split(',').map(str::trim) {
                        let from_version =
                            try_annotate_semver_build(&mut graph, from_version, &to)?;

                        if let Some(from) = graph.find_by_version(&from_version) {
                            info!("[{}]: removing previous {}", to_version, from_version);
                            handle_remove_edge!(from, to)
                        } else {
                            warn!(
                                "couldn't find version given by 'previous.remove={}' in graph",
                                from_version
                            )
                        }
                    }
                    Ok(())
                },
            )?;

        // Remove edges instructed by "previous.remove_regex"
        let previous_remove_regex_key = format!("{}.{}", self.key_prefix, "previous.remove_regex");
        graph
            .find_by_metadata_key(&previous_remove_regex_key)
            .into_iter()
            .try_for_each(
                |(to, to_version, from_regex_string): (ReleaseId, String, String)| -> Fallible<()> {
                    if self.remove_consumed_metadata {
                        graph
                            .get_metadata_as_ref_mut(&to)
                            .map(|metadata| metadata.remove(&previous_remove_regex_key))?;
                    }

                    let from_regex = regex::Regex::new(&from_regex_string)
                        .context(format!("Parsing {} as Regex", &from_regex_string))?;

                    if from_regex_string == ".*" {
                        let parents: Vec<daggy::EdgeIndex> = graph
                            .previous_releases(&to)
                            .map(|(edge_index, _, _)| edge_index)
                            .collect();

                        trace!(
                            "removing parents by regex for '{}': {:?}",
                            to_version,
                            parents
                        );
                        return graph.remove_edges_by_index(&parents);
                    };

                    let froms = graph.find_by_fn_mut(|release| {
                        if from_regex.is_match(release.version()) {
                            debug!(
                                "Regex '{}' matches version '{}'",
                                &from_regex,
                                release.version(),
                            );
                            true
                        } else {
                            false
                        }
                    });

                    for (from, from_version) in froms {
                        info!(
                            "[{}]: removing previous {} by regex",
                            to_version, from_version
                        );
                        handle_remove_edge!(from, to);
                    }

                    Ok(())
                },
            )?;

        let next_remove_key = format!("{}.{}", self.key_prefix, "next.remove");
        graph
            .find_by_metadata_key(&next_remove_key)
            .into_iter()
            .try_for_each(
                |(from, from_version, to_csv): (ReleaseId, String, String)| -> Fallible<()> {
                    if self.remove_consumed_metadata {
                        graph
                            .get_metadata_as_ref_mut(&from)
                            .map(|metadata| metadata.remove(&next_remove_key))?;
                    }

                    for to_version in to_csv.split(',').map(str::trim) {
                        let to_version = try_annotate_semver_build(&mut graph, to_version, &from)?;
                        if let Some(to) = graph.find_by_version(&to_version) {
                            info!("[{}]: removing next {}", from_version, to_version);
                            handle_remove_edge!(from, to)
                        } else {
                            warn!(
                                "couldn't find version given by 'next.remove={}' in graph",
                                to_version
                            )
                        }
                    }
                    Ok(())
                },
            )?;

        Ok(())
    }

    /// Add next and previous releases specified by metadata.
    ///
    /// The labels are assumed to have the syntax `<prefix>.(previous|next).add=(<Version>,)*<Version>`
    fn add_edges(&self, mut graph: &mut cincinnati::Graph) -> Fallible<()> {
        macro_rules! handle_add_edge {
            ($direction:expr, $from:ident, $to:ident, $from_string:ident, $to_string:ident) => {
                if let Err(e) = graph.add_edge(&$from, &$to) {
                    if let Some(eae) = e.downcast_ref::<cincinnati::errors::EdgeAlreadyExists>() {
                        warn!("{}", eae);
                        continue;
                    };
                    bail!(e);
                };
            };
        }

        let previous_add_key = format!("{}.{}", self.key_prefix, "previous.add");
        graph
            .find_by_metadata_key(&previous_add_key)
            .into_iter()
            .try_for_each(|(to, to_version, from_csv)| -> Fallible<()> {
                if self.remove_consumed_metadata {
                    graph
                        .get_metadata_as_ref_mut(&to)
                        .map(|metadata| metadata.remove(&previous_add_key))?;
                }

                for from_version in from_csv.split(',').map(str::trim) {
                    let from_version_annotated =
                        try_annotate_semver_build(&mut graph, from_version, &to)?;

                    if let Some(from) = graph.find_by_version(&from_version_annotated) {
                        info!(
                            "[{}]: adding {} {}",
                            &to_version, "previous", &from_version_annotated
                        );
                        handle_add_edge!("previous", from, to, from_string, to_string);
                    } else {
                        warn!(
                            "couldn't find version given by 'previous.add={}' in graph",
                            from_version_annotated
                        )
                    }
                }
                Ok(())
            })?;

        let next_add_key = format!("{}.{}", self.key_prefix, "next.add");
        graph
            .find_by_metadata_key(&next_add_key)
            .into_iter()
            .try_for_each(|(from, from_version, to_csv)| -> Fallible<()> {
                if self.remove_consumed_metadata {
                    graph
                        .get_metadata_as_ref_mut(&from)
                        .map(|metadata| metadata.remove(&next_add_key))?;
                }

                for to_version in to_csv.split(',').map(str::trim) {
                    let to_version_annotated =
                        try_annotate_semver_build(&mut graph, to_version, &from)?;

                    if let Some(to) = graph.find_by_version(&to_version_annotated) {
                        info!(
                            "[{}]: adding {} {}",
                            &from_version, "next", &to_version_annotated
                        );
                        handle_add_edge!("next", from, to, from_version, to_version_annotated);
                    } else {
                        warn!(
                            "couldn't find version given by 'next.add={}' in graph",
                            to_version_annotated
                        )
                    }
                }
                Ok(())
            })?;

        Ok(())
    }
}

/// Try to find the architecture metadata and add it to the version String assuming SemVer.
///
/// If the referenced ReleaseId doesn't have the arch metadata, the version
/// string will be passed through unchanged.
fn try_annotate_semver_build(
    graph: &mut cincinnati::Graph,
    version: &str,
    arch_reference: &ReleaseId,
) -> Fallible<String> {
    let version = if let Some(arch) = graph
        .get_metadata_as_ref_mut(arch_reference)?
        .get("io.openshift.upgrades.graph.release.arch")
    {
        let mut version = semver::Version::parse(version)?;
        version.build = vec![semver::Identifier::AlphaNumeric(arch.to_string())];
        version.to_string()
    } else {
        version.to_string()
    };

    Ok(version)
}

#[cfg(test)]
mod tests {
    use super::*;
    use cincinnati::testing::generate_custom_graph;
    use cincinnati::MapImpl;
    use commons::testing::init_runtime;

    static KEY_PREFIX: &str = "test_key";

    #[test]
    fn ensure_previous_remove() -> Fallible<()> {
        let runtime = init_runtime()?;

        let key_prefix = "test_prefix".to_string();
        let key_suffix = "previous.remove".to_string();

        let metadata: Vec<(usize, MapImpl<String, String>)> = [
            (0, [].iter().cloned().collect()),
            (1, [].iter().cloned().collect()),
            (
                2,
                [(
                    format!("{}.{}", key_prefix, key_suffix),
                    String::from("0.0.0, 1.0.0"),
                )]
                .iter()
                .cloned()
                .collect(),
            ),
        ]
        .iter()
        .cloned()
        .collect();

        let input_graph: cincinnati::Graph = generate_custom_graph(
            "image",
            metadata.clone(),
            Some(vec![(0, 1), (0, 2), (1, 2)]),
        );

        let expected_graph: cincinnati::Graph =
            generate_custom_graph("image", metadata, Some([(0, 1)].to_vec()));

        let plugin = Box::new(EdgeAddRemovePlugin {
            key_prefix,
            remove_all_edges_value: DEFAULT_REMOVE_ALL_EDGES_VALUE.to_string(),

            ..Default::default()
        });
        let future_processed_graph = plugin.run_internal(InternalIO {
            graph: input_graph.clone(),
            parameters: Default::default(),
        });

        let processed_graph = runtime
            .block_on(future_processed_graph)
            .context("plugin run failed")?
            .graph;

        assert_eq!(expected_graph, processed_graph);
        Ok(())
    }

    #[test]
    fn ensure_previous_remove_all() -> Fallible<()> {
        let runtime = init_runtime()?;

        let key_prefix = "test_prefix".to_string();
        let key_suffix = "previous.remove".to_string();

        let metadata: Vec<(usize, MapImpl<String, String>)> = [
            (0, [].iter().cloned().collect()),
            (1, [].iter().cloned().collect()),
            (
                2,
                [(
                    format!("{}.{}", key_prefix, key_suffix),
                    DEFAULT_REMOVE_ALL_EDGES_VALUE.to_string(),
                )]
                .iter()
                .cloned()
                .collect(),
            ),
            (3, [].iter().cloned().collect()),
        ]
        .iter()
        .cloned()
        .collect();

        let input_graph: cincinnati::Graph = generate_custom_graph(
            "image",
            metadata.clone(),
            Some(vec![(0, 1), (0, 2), (1, 2), (2, 3)]),
        );

        let expected_graph: cincinnati::Graph =
            generate_custom_graph("image", metadata, Some([(0, 1), (2, 3)].to_vec()));

        let plugin = Box::new(EdgeAddRemovePlugin {
            key_prefix,
            remove_all_edges_value: DEFAULT_REMOVE_ALL_EDGES_VALUE.to_string(),

            ..Default::default()
        });
        let future_processed_graph = plugin.run_internal(InternalIO {
            graph: input_graph,
            parameters: Default::default(),
        });

        let processed_graph = runtime
            .block_on(future_processed_graph)
            .context("plugin run failed")?
            .graph;

        assert_eq!(expected_graph, processed_graph);

        Ok(())
    }

    #[test]
    fn ensure_next_remove() -> Fallible<()> {
        let runtime = init_runtime()?;

        let key_prefix = "test_prefix".to_string();
        let key_suffix = "next.remove".to_string();

        let metadata: Vec<(usize, MapImpl<String, String>)> = [
            (
                0,
                [(
                    format!("{}.{}", key_prefix, key_suffix),
                    String::from("1.0.0, 2.0.0"),
                )]
                .iter()
                .cloned()
                .collect(),
            ),
            (
                1,
                [(
                    format!("{}.{}", key_prefix, key_suffix),
                    String::from("2.0.0"),
                )]
                .iter()
                .cloned()
                .collect(),
            ),
            (2, [].iter().cloned().collect()),
        ]
        .iter()
        .cloned()
        .collect();

        let input_graph: cincinnati::Graph = generate_custom_graph(
            "image",
            metadata.clone(),
            Some(vec![(0, 1), (0, 2), (1, 2)]),
        );

        let expected_graph: cincinnati::Graph =
            generate_custom_graph("image", metadata, Some(vec![]));

        let plugin = Box::new(EdgeAddRemovePlugin {
            key_prefix,
            remove_all_edges_value: DEFAULT_REMOVE_ALL_EDGES_VALUE.to_string(),

            ..Default::default()
        });
        let future_processed_graph = plugin.run_internal(InternalIO {
            graph: input_graph,
            parameters: Default::default(),
        });

        let processed_graph = runtime
            .block_on(future_processed_graph)
            .context("plugin run failed")?
            .graph;

        assert_eq!(expected_graph, processed_graph);

        Ok(())
    }

    #[test]
    fn ensure_previous_add() -> Fallible<()> {
        let runtime = init_runtime()?;

        let key_prefix = "test_prefix".to_string();
        let key_suffix = "previous.add".to_string();

        let metadata: Vec<(usize, MapImpl<String, String>)> = [
            (0, [].iter().cloned().collect()),
            (1, [].iter().cloned().collect()),
            (
                2,
                [(
                    format!("{}.{}", key_prefix, key_suffix),
                    String::from("0.0.0, 1.0.0"),
                )]
                .iter()
                .cloned()
                .collect(),
            ),
        ]
        .iter()
        .cloned()
        .collect();

        let input_graph: cincinnati::Graph =
            generate_custom_graph("image", metadata.clone(), Some(vec![(0, 1)]));

        let expected_graph: cincinnati::Graph =
            generate_custom_graph("image", metadata, Some(vec![(0, 1), (0, 2), (1, 2)]));

        let plugin = Box::new(EdgeAddRemovePlugin {
            key_prefix,
            remove_all_edges_value: DEFAULT_REMOVE_ALL_EDGES_VALUE.to_string(),

            ..Default::default()
        });
        let future_processed_graph = plugin.run_internal(InternalIO {
            graph: input_graph,
            parameters: Default::default(),
        });

        let processed_graph = runtime
            .block_on(future_processed_graph)
            .context("plugin run failed")?
            .graph;

        assert_eq!(expected_graph, processed_graph);

        Ok(())
    }

    #[test]
    fn ensure_next_add() -> Fallible<()> {
        let runtime = init_runtime()?;

        let key_prefix = "test_prefix".to_string();
        let key_suffix = "next.add".to_string();

        let metadata: Vec<(usize, MapImpl<String, String>)> = [
            (
                0,
                [(
                    format!("{}.{}", key_prefix, key_suffix),
                    String::from("3.0.0 , 2.0.0"),
                )]
                .iter()
                .cloned()
                .collect(),
            ),
            (1, [].iter().cloned().collect()),
            (2, [].iter().cloned().collect()),
            (3, [].iter().cloned().collect()),
        ]
        .iter()
        .cloned()
        .collect();

        let input_graph: cincinnati::Graph = generate_custom_graph("image", metadata.clone(), None);

        let expected_graph: cincinnati::Graph = generate_custom_graph(
            "image",
            metadata,
            Some(vec![(0, 1), (0, 2), (0, 3), (1, 2), (2, 3)]),
        );

        let plugin = Box::new(EdgeAddRemovePlugin {
            key_prefix,
            remove_all_edges_value: DEFAULT_REMOVE_ALL_EDGES_VALUE.to_string(),

            ..Default::default()
        });

        let future_processed_graph = plugin.run_internal(InternalIO {
            graph: input_graph,
            parameters: Default::default(),
        });

        let processed_graph = runtime
            .block_on(future_processed_graph)
            .context("plugin run failed")?
            .graph;

        assert_eq!(expected_graph, processed_graph);

        Ok(())
    }

    macro_rules! label_processing_order_test {
        (
            name: $name:ident,
            input_metadata: $input_metadata:expr,
            input_edges: $input_edges:expr,
            expected_edges: $expected_edges:expr,
        ) => {
            #[test]
            fn $name() -> Fallible<()> {
                let runtime = init_runtime()?;

                let input_metadata: Vec<(usize, MapImpl<String, String>)> = $input_metadata
                    .iter()
                    .map(|(n, metadata)| {
                        (
                            *n,
                            metadata
                                .iter()
                                .map(|(k, v)| (format!("{}.{}", KEY_PREFIX, k), v.to_string()))
                                .collect(),
                        )
                    })
                    .collect();

                let input_graph: cincinnati::Graph =
                    generate_custom_graph("image", input_metadata.clone(), $input_edges.to_owned());

                let expected_graph: cincinnati::Graph =
                    generate_custom_graph("image", input_metadata, $expected_edges.to_owned());

                let plugin = Box::new(EdgeAddRemovePlugin {
                    key_prefix: KEY_PREFIX.to_string(),
                    remove_all_edges_value: DEFAULT_REMOVE_ALL_EDGES_VALUE.to_string(),

                    ..Default::default()
                });
                let future_processed_graph = plugin.run_internal(InternalIO {
                    graph: input_graph.clone(),
                    parameters: Default::default(),
                });

                let processed_graph = runtime.block_on(future_processed_graph)?.graph;

                assert_eq!(expected_graph, processed_graph);

                Ok(())
            }
        };
    }

    label_processing_order_test!(
        name: contradicting_inter_node_labels,
        input_metadata:
            vec![
                (
                    0,
                    vec![
                        // (a)
                        ("next.add", "1.0.0"),
                    ],
                ),
                (
                    1,
                    vec![
                        // (a)
                        ("previous.remove", "0.0.0"),
                        // (b)
                        ("next.remove", "2.0.0"),
                    ],
                ),
                (
                    2,
                    vec![
                        // (b)
                        ("previous.add", "1.0.0"),
                    ],
                ),
            ],
        input_edges: Some(vec![(0, 1), (1, 2)]),
        expected_edges: Some(vec![]),
    );

    label_processing_order_test!(
        name: contradicting_intra_node_labels,
        input_metadata:
            vec![
                (0, vec![("next.add", "1.0.0"), ("next.remove", "1.0.0")]),
                (1, vec![]),
                (
                    2,
                    vec![
                        // (b)
                        ("previous.remove", "1.0.0"),
                        ("previous.add", "1.0.0"),
                    ],
                ),
            ],
        input_edges: Some(vec![]),
        expected_edges: Some(vec![]),
    );

    label_processing_order_test!(
        name: contradicting_inter_and_intra_node_labels,
        input_metadata:
            vec![
                (
                    0,
                    vec![
                        // (a)
                        ("next.add", "1.0.0"),
                        ("next.remove", "1.0.0"),
                    ],
                ),
                (
                    1,
                    vec![
                        // (a)
                        ("previous.add", "0.0.0"),
                        // (b)
                        ("next.add", "2.0.0"),
                        ("next.remove", "2.0.0"),
                    ],
                ),
                (
                    2,
                    vec![
                        // (b)
                        ("previous.add", "1.0.0"),
                    ],
                ),
                (
                    3,
                    vec![
                        // (b)
                        ("previous.remove", "2.0.0"),
                        ("previous.add", "2.0.0"),
                    ],
                ),
            ],
        input_edges: Some(vec![]),
        expected_edges: Some(vec![]),
    );

    label_processing_order_test!(
        name: dont_add_duplicate_edges,
        input_metadata:
            vec![
                (0, vec![("next.add", "1.0.0"),],),
                (1, vec![("previous.add", "0.0.0"),],),
            ],
        input_edges: Some(vec![(0, 1)]),
        expected_edges: Some(vec![(0, 1)]),
    );

    label_processing_order_test!(
        name: gracefully_handle_nonexistent_edge_removal,
        input_metadata:
            vec![
                (0, vec![("next.remove", "1.0.0")]),
                (1, vec![("previous.remove", "1.0.0"),])
            ],
        input_edges: Some(vec![]),
        expected_edges: Some(vec![]),
    );

    label_processing_order_test!(
        name: gracefully_handle_nonexistent_release_references,
        input_metadata:
            vec![(
                0,
                vec![
                    ("next.add", "1.0.0"),
                    ("previous.add", "1.0.0"),
                    ("next.remove", "1.0.0"),
                    ("previous.remove", "1.0.0"),
                ]
            )],
        input_edges: Some(vec![]),
        expected_edges: Some(vec![]),
    );

    #[test]
    fn edge_remove_bug() -> Fallible<()> {
        let runtime = init_runtime()?;

        lazy_static::lazy_static! {
            static ref TEST_KEY_PREFIX: &'static str = "io.openshift.upgrades.graph";
            static ref PLUGINS: Vec<BoxedPlugin> = vec![
                Box::new(InternalPluginWrapper(EdgeAddRemovePlugin {
                    key_prefix: TEST_KEY_PREFIX.to_string(),
                    remove_all_edges_value: DEFAULT_REMOVE_ALL_EDGES_VALUE.to_string(),

                    ..Default::default()
                })),
            ];

        };

        let input_graph: cincinnati::Graph = serde_json::from_reader(
            std::fs::File::open(
                "src/plugins/test_fixtures/edge_add_remove_trigger_edge_index_error.json",
            )
            .unwrap(),
        )
        .unwrap();

        let process_result = cincinnati::plugins::process(
            PLUGINS.iter(),
            cincinnati::plugins::PluginIO::InternalIO(cincinnati::plugins::InternalIO {
                graph: input_graph.clone(),
                parameters: Default::default(),
            }),
        );

        let graph = runtime.block_on(process_result)?.graph;
        assert_ne!(graph, input_graph);

        Ok(())
    }

    // TODO(steveeJ): add multiarch tests once design is settled
}
