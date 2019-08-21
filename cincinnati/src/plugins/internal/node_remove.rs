//! This plugin removes releases according to its metadata

use crate::plugins::{
    AsyncIO, InternalIO, InternalPlugin, InternalPluginWrapper, BoxedPlugin, PluginSettings,
};
use failure::Fallible;

static DEFAULT_KEY_FILTER: &str = "io.openshift.upgrades.graph";

#[derive(Clone, Debug, Deserialize, SmartDefault)]
#[serde(default)]
pub struct NodeRemovePlugin {
    #[default(DEFAULT_KEY_FILTER.to_string())]
    pub key_prefix: String,
}

impl PluginSettings for NodeRemovePlugin {
    fn build_plugin(&self) -> Fallible<BoxedPlugin> {
        Ok(new_plugin!(InternalPluginWrapper(self.clone())))
    }
}

impl NodeRemovePlugin {
    /// Plugin name, for configuration.
    pub(crate) const PLUGIN_NAME: &'static str = "node-remove";

    /// Validate plugin configuration and fill in defaults.
    pub fn deserialize_config(cfg: toml::Value) -> Fallible<Box<PluginSettings>> {
        let plugin: Self = cfg.try_into()?;

        ensure!(!plugin.key_prefix.is_empty(), "empty prefix");

        Ok(Box::new(plugin))
    }
}

impl InternalPlugin for NodeRemovePlugin {
    fn run_internal(self: &Self, io: InternalIO) -> AsyncIO<InternalIO> {
        let closure = || -> Fallible<InternalIO> {
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
        };

        Box::new(futures::future::result(closure()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate as cincinnati;
    use commons::testing::init_runtime;
    use failure::ResultExt;
    use std::collections::HashMap;

    #[test]
    fn ensure_release_remove() -> Fallible<()> {
        let mut runtime = init_runtime()?;

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

        let future_processed_graph =
            Box::new(NodeRemovePlugin { key_prefix }).run_internal(InternalIO {
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
}
