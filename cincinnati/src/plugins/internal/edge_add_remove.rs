//! This plugin adds and removes Edges from Nodes accordingly

use crate as cincinnati;
use failure::Fallible;
use plugins::{InternalIO, InternalPlugin};
use ReleaseId;

pub struct EdgeAddRemovePlugin {
    pub key_prefix: String,
}

impl InternalPlugin for EdgeAddRemovePlugin {
    fn run_internal(&self, io: InternalIO) -> Fallible<InternalIO> {
        let mut graph = io.graph;
        self.remove_edges(&mut graph)?;
        self.add_edges(&mut graph)?;
        Ok(InternalIO {
            graph,
            parameters: io.parameters,
        })
    }
}

/// Adds and removes next and previous releases to/from quay labels
///
/// The labels are assumed to have the syntax `<prefix>.(previous|next).(remove|add)=(<Version>,)*<Version>`
impl EdgeAddRemovePlugin {
    /// Remove next and previous releases from quay labels
    ///
    /// The labels are assumed to have the syntax `<prefix>.(previous|next).remove=(<Version>,)*<Version>`
    fn remove_edges(&self, graph: &mut cincinnati::Graph) -> Fallible<()> {
        graph
            .find_by_metadata_key(&format!("{}.{}", self.key_prefix, "previous.remove"))
            .into_iter()
            .try_for_each(
                |(to, to_version, from_csv): (ReleaseId, String, String)| -> Fallible<()> {
                    for from_version in from_csv.split(',').map(|s| s.trim()) {
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
                    for to_version in to_csv.split(',').map(|s| s.trim()) {
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

    /// Add next and previous releases from quay labels
    ///
    /// The labels are assumed to have the syntax `<prefix>.(previous|next).add=(<Version>,)*<Version>`
    fn add_edges(&self, graph: &mut cincinnati::Graph) -> Fallible<()> {
        graph
            .find_by_metadata_key(&format!("{}.{}", self.key_prefix, "previous.add"))
            .into_iter()
            .try_for_each(|(to, to_version, from_csv)| -> Fallible<()> {
                for from_version in from_csv.split(',').map(|s| s.trim()) {
                    if let Some(from) = graph.find_by_version(&from_version) {
                        info!("[{}]: adding previous {}", &from_version, &to_version);
                        graph.add_edge(&from, &to)?
                    } else {
                        bail!(
                            "couldn't find version given by 'previous.add={}' in graph",
                            from_version
                        )
                    }
                }
                Ok(())
            })?;

        graph
            .find_by_metadata_key(&format!("{}.{}", self.key_prefix, "next.add"))
            .into_iter()
            .try_for_each(|(from, from_string, to_csv)| -> Fallible<()> {
                for to_string in to_csv.split(',').map(|s| s.trim()) {
                    if let Some(to) = graph.find_by_version(&to_string) {
                        info!("[{}]: adding next {}", &from_string, &to_string);
                        graph.add_edge(&from, &to)?;
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
    use std::collections::HashMap;

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
            Some([(0, 1)].iter().cloned().collect()),
        );

        let processed_graph = EdgeAddRemovePlugin { key_prefix }
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

        let processed_graph = EdgeAddRemovePlugin { key_prefix }
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

        let processed_graph = EdgeAddRemovePlugin { key_prefix }
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

        let processed_graph = EdgeAddRemovePlugin { key_prefix }
            .run_internal(InternalIO {
                graph: input_graph,
                parameters: Default::default(),
            })
            .expect("plugin run failed")
            .graph;

        assert_eq!(expected_graph, processed_graph);
    }
}
