//! This plugin removes releases according to its metadata

use failure::Fallible;
use plugins::{InternalIO, InternalPlugin, InternalPluginWrapper, Plugin, PluginIO};
use std::collections::HashMap;

static DEFAULT_KEY_FILTER: &str = "com.openshift.upgrades.graph";

pub struct NodeRemovePlugin {
    pub key_prefix: String,
}

impl NodeRemovePlugin {
    /// Plugin name, for configuration.
    pub(crate) const PLUGIN_NAME: &'static str = "node-remove";

    /// Validate plugin configuration and fill in defaults.
    pub fn sanitize_config(cfg: &mut HashMap<String, String>) -> Fallible<()> {
        let name = cfg.get("name").cloned().unwrap_or_default();
        ensure!(name == Self::PLUGIN_NAME, "unexpected plugin name");

        cfg.entry("key_prefix".to_string())
            .or_insert_with(|| DEFAULT_KEY_FILTER.to_string());
        // TODO(lucab): perform semantic validation.

        Ok(())
    }

    /// Try to build a plugin from settings.
    pub fn from_settings(cfg: &HashMap<String, String>) -> Fallible<Box<Plugin<PluginIO>>> {
        let key_prefix = cfg
            .get("key_prefix")
            .ok_or_else(|| format_err!("empty key_prefix"))?
            .to_string();
        let plugin = Self { key_prefix };

        Ok(Box::new(InternalPluginWrapper(plugin)))
    }
}

impl InternalPlugin for NodeRemovePlugin {
    fn run_internal(&self, io: InternalIO) -> Fallible<InternalIO> {
        let mut graph = io.graph;
        let key_suffix = "release.remove";

        let to_remove = {
            graph
                .find_by_metadata_pair(&format!("{}.{}", self.key_prefix, key_suffix), "true")
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

        Ok(InternalIO {
            graph,
            parameters: io.parameters,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate as cincinnati;
    use std::collections::HashMap;

    #[test]
    fn ensure_release_remove() {
        let key_prefix = "test_prefix".to_string();
        let key_suffix = "release.remove".to_string();

        let input_graph: cincinnati::Graph = {
            let metadata: HashMap<usize, HashMap<String, String>> = [
                (
                    0,
                    [(
                        format!("{}.{}", key_prefix, key_suffix),
                        String::from("true"),
                    )]
                    .iter()
                    .cloned()
                    .collect(),
                ),
                (1, HashMap::new()),
                (
                    2,
                    [(
                        format!("{}.{}", key_prefix, key_suffix),
                        String::from("true"),
                    )]
                    .iter()
                    .cloned()
                    .collect(),
                ),
            ]
            .iter()
            .cloned()
            .collect();

            crate::tests::generate_custom_graph(0, metadata.len(), metadata, None)
        };

        let expected_graph: cincinnati::Graph = {
            let metadata: HashMap<usize, HashMap<String, String>> =
                [(1, HashMap::new())].iter().cloned().collect();

            crate::tests::generate_custom_graph(1, metadata.len(), metadata, None)
        };

        let processed_graph = NodeRemovePlugin { key_prefix }
            .run_internal(InternalIO {
                graph: input_graph.clone(),
                parameters: Default::default(),
            })
            .expect("plugin run failed")
            .graph;

        assert_eq!(expected_graph, processed_graph);
    }
}
