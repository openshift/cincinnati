//! This plugin adds and removes Edges from Nodes accordingly

use crate as cincinnati;
use crate::plugins::{
    InternalIO, InternalPlugin, InternalPluginWrapper, Plugin, PluginIO, PluginSettings,
};
use crate::ReleaseId;
use failure::Fallible;

static DEFAULT_KEY_FILTER: &str = "io.openshift.upgrades.graph";
pub static DEFAULT_REMOVE_ALL_EDGES_VALUE: &str = "*";

#[derive(Clone, Debug, Deserialize, SmartDefault)]
#[serde(default)]
pub struct EdgeAddRemovePlugin {
    #[default(DEFAULT_KEY_FILTER.to_string())]
    pub key_prefix: String,

    #[default(DEFAULT_REMOVE_ALL_EDGES_VALUE.to_string())]
    pub remove_all_edges_value: String,
}

impl InternalPlugin for EdgeAddRemovePlugin {
    fn run_internal(&self, io: InternalIO) -> Fallible<InternalIO> {
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
    fn build_plugin(&self) -> Fallible<Box<Plugin<PluginIO>>> {
        Ok(Box::new(InternalPluginWrapper(self.clone())))
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
///     2. next
///
/// This ordering has implications on the result of semantical contradictions, so that the `*.remove` labels take precedence over `*.add`.
impl EdgeAddRemovePlugin {
    /// Plugin name, for configuration.
    pub(crate) const PLUGIN_NAME: &'static str = "edge-add-remove";

    /// Validate plugin configuration and fill in defaults.
    pub fn deserialize_config(cfg: toml::Value) -> Fallible<Box<PluginSettings>> {
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
    fn remove_edges(&self, graph: &mut cincinnati::Graph) -> Fallible<()> {
        graph
            .find_by_metadata_key(&format!("{}.{}", self.key_prefix, "previous.remove"))
            .into_iter()
            .try_for_each(
                |(to, to_version, from_csv): (ReleaseId, String, String)| -> Fallible<()> {
                    if from_csv.trim() == self.remove_all_edges_value {
                        let parents: Vec<daggy::EdgeIndex> = graph
                            .previous_releases(&to)
                            .map(|(edge_index, _, _)| edge_index)
                            .collect();

                        trace!("removing parents for '{}': {:?}", to_version, parents);
                        return graph.remove_edges_by_index(&parents);
                    }

                    for from_version in from_csv.split(',').map(str::trim) {
                        if let Some(from) = graph.find_by_version(&from_version) {
                            info!("[{}]: removing previous {}", from_version, to_version,);
                            graph.remove_edge(&from, &to)?
                        } else {
                            bail!(
                                "couldn't find version given by 'previous.remove={}' in graph",
                                from_version
                            )
                        }
                    }
                    Ok(())
                },
            )?;

        graph
            .find_by_metadata_key(&format!("{}.{}", self.key_prefix, "next.remove"))
            .into_iter()
            .try_for_each(
                |(from, from_version, to_csv): (ReleaseId, String, String)| -> Fallible<()> {
                    for to_version in to_csv.split(',').map(str::trim) {
                        if let Some(to) = graph.find_by_version(&to_version) {
                            info!("[{}]: removing next {}", from_version, to_version);
                            graph.remove_edge(&from, &to)?
                        } else {
                            info!(
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
    fn add_edges(&self, graph: &mut cincinnati::Graph) -> Fallible<()> {
        macro_rules! handle_add_edge {
            ($direction:expr, $from:ident, $to:ident, $from_string:ident, $to_string:ident) => {
                if let Err(e) = graph.add_edge(&$from, &$to) {
                    if let Some(eae) = e.downcast_ref::<crate::errors::EdgeAlreadyExists>() {
                        warn!("{}", eae);
                        continue;
                    };
                    bail!(e);
                };
            };
        }

        graph
            .find_by_metadata_key(&format!("{}.{}", self.key_prefix, "previous.add"))
            .into_iter()
            .try_for_each(|(to, to_string, from_csv)| -> Fallible<()> {
                for from_string in from_csv.split(',').map(str::trim) {
                    if let Some(from) = graph.find_by_version(&from_string) {
                        info!("[{}]: adding {} {}", &to_string, "previous", &from_string);
                        handle_add_edge!("previous", from, to, from_string, to_string);
                    } else {
                        bail!(
                            "couldn't find version given by 'previous.add={}' in graph",
                            from_string
                        )
                    }
                }
                Ok(())
            })?;

        graph
            .find_by_metadata_key(&format!("{}.{}", self.key_prefix, "next.add"))
            .into_iter()
            .try_for_each(|(from, from_string, to_csv)| -> Fallible<()> {
                for to_string in to_csv.split(',').map(str::trim) {
                    if let Some(to) = graph.find_by_version(&to_string) {
                        info!("[{}]: adding {} {}", &from_string, "next", &to_string);
                        handle_add_edge!("next", from, to, from_string, to_string);
                    } else {
                        bail!(
                            "couldn't find version given by 'next.add={}' in graph",
                            to_string
                        )
                    }
                }
                Ok(())
            })?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate as cincinnati;
    use commons::testing::init_logger;
    use std::collections::HashMap;

    static KEY_PREFIX: &str = "test_key";

    #[test]
    fn ensure_previous_remove() {
        let key_prefix = "test_prefix".to_string();
        let key_suffix = "previous.remove".to_string();

        let metadata: HashMap<usize, HashMap<String, String>> = [
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

        let input_graph: cincinnati::Graph = crate::tests::generate_custom_graph(
            0,
            metadata.len(),
            metadata.clone(),
            Some(vec![(0, 1), (0, 2), (1, 2)]),
        );

        let expected_graph: cincinnati::Graph = crate::tests::generate_custom_graph(
            0,
            metadata.len(),
            metadata,
            Some([(0, 1)].to_vec()),
        );

        let processed_graph = EdgeAddRemovePlugin {
            key_prefix,
            remove_all_edges_value: DEFAULT_REMOVE_ALL_EDGES_VALUE.to_string(),
        }
        .run_internal(InternalIO {
            graph: input_graph.clone(),
            parameters: Default::default(),
        })
        .expect("plugin run failed")
        .graph;

        assert_eq!(expected_graph, processed_graph);
    }

    #[test]
    fn ensure_previous_remove_all() {
        let key_prefix = "test_prefix".to_string();
        let key_suffix = "previous.remove".to_string();

        let metadata: HashMap<usize, HashMap<String, String>> = [
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

        let input_graph: cincinnati::Graph = crate::tests::generate_custom_graph(
            0,
            metadata.len(),
            metadata.clone(),
            Some(vec![(0, 1), (0, 2), (1, 2), (2, 3)]),
        );

        let expected_graph: cincinnati::Graph = crate::tests::generate_custom_graph(
            0,
            metadata.len(),
            metadata,
            Some([(0, 1), (2, 3)].to_vec()),
        );

        let processed_graph = EdgeAddRemovePlugin {
            key_prefix,
            remove_all_edges_value: DEFAULT_REMOVE_ALL_EDGES_VALUE.to_string(),
        }
        .run_internal(InternalIO {
            graph: input_graph.clone(),
            parameters: Default::default(),
        })
        .expect("plugin run failed")
        .graph;

        assert_eq!(expected_graph, processed_graph);
    }

    #[test]
    fn ensure_next_remove() {
        let key_prefix = "test_prefix".to_string();
        let key_suffix = "next.remove".to_string();

        let metadata: HashMap<usize, HashMap<String, String>> = [
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

        let input_graph: cincinnati::Graph = crate::tests::generate_custom_graph(
            0,
            metadata.len(),
            metadata.clone(),
            Some(vec![(0, 1), (0, 2), (1, 2)]),
        );

        let expected_graph: cincinnati::Graph =
            crate::tests::generate_custom_graph(0, metadata.len(), metadata, Some(vec![]));

        let processed_graph = EdgeAddRemovePlugin {
            key_prefix,
            remove_all_edges_value: DEFAULT_REMOVE_ALL_EDGES_VALUE.to_string(),
        }
        .run_internal(InternalIO {
            graph: input_graph.clone(),
            parameters: Default::default(),
        })
        .expect("plugin run failed")
        .graph;

        assert_eq!(expected_graph, processed_graph);
    }

    #[test]
    fn ensure_previous_add() {
        let key_prefix = "test_prefix".to_string();
        let key_suffix = "previous.add".to_string();

        let metadata: HashMap<usize, HashMap<String, String>> = [
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

        let input_graph: cincinnati::Graph = crate::tests::generate_custom_graph(
            0,
            metadata.len(),
            metadata.clone(),
            Some(vec![(0, 1)]),
        );

        let expected_graph: cincinnati::Graph = crate::tests::generate_custom_graph(
            0,
            metadata.len(),
            metadata,
            Some(vec![(0, 1), (0, 2), (1, 2)]),
        );

        let processed_graph = EdgeAddRemovePlugin {
            key_prefix,
            remove_all_edges_value: DEFAULT_REMOVE_ALL_EDGES_VALUE.to_string(),
        }
        .run_internal(InternalIO {
            graph: input_graph.clone(),
            parameters: Default::default(),
        })
        .expect("plugin run failed")
        .graph;

        assert_eq!(expected_graph, processed_graph);
    }

    #[test]
    fn ensure_next_add() {
        let key_prefix = "test_prefix".to_string();
        let key_suffix = "next.add".to_string();

        let metadata: HashMap<usize, HashMap<String, String>> = [
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

        let input_graph: cincinnati::Graph =
            crate::tests::generate_custom_graph(0, metadata.len(), metadata.clone(), None);

        let expected_graph: cincinnati::Graph = crate::tests::generate_custom_graph(
            0,
            metadata.len(),
            metadata,
            Some(vec![(0, 1), (0, 2), (0, 3), (1, 2), (2, 3)]),
        );

        let processed_graph = EdgeAddRemovePlugin {
            key_prefix,
            remove_all_edges_value: DEFAULT_REMOVE_ALL_EDGES_VALUE.to_string(),
        }
        .run_internal(InternalIO {
            graph: input_graph,
            parameters: Default::default(),
        })
        .expect("plugin run failed")
        .graph;

        assert_eq!(expected_graph, processed_graph);
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
                let _ = init_logger();

                let input_metadata: HashMap<usize, HashMap<String, String>> = $input_metadata
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

                let input_graph: cincinnati::Graph = crate::tests::generate_custom_graph(
                    0,
                    input_metadata.len(),
                    input_metadata.clone(),
                    $input_edges.to_owned(),
                );

                let expected_graph: cincinnati::Graph = crate::tests::generate_custom_graph(
                    0,
                    input_metadata.len(),
                    input_metadata,
                    $expected_edges.to_owned(),
                );

                let processed_graph = EdgeAddRemovePlugin {
                    key_prefix: KEY_PREFIX.to_string(),
                    remove_all_edges_value: DEFAULT_REMOVE_ALL_EDGES_VALUE.to_string(),
                }
                .run_internal(InternalIO {
                    graph: input_graph.clone(),
                    parameters: Default::default(),
                })?
                .graph;

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
}
