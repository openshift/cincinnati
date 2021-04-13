//! This plugin removes releases according to its metadata

use crate as cincinnati;

use self::cincinnati::plugins::prelude::*;
use self::cincinnati::plugins::prelude_plugin_impl::*;

/// Prefix for the metadata key operations.
pub static DEFAULT_KEY_FILTER: &str = "io.openshift.upgrades.graph";

#[derive(Clone, Debug, Deserialize, SmartDefault)]
#[serde(default)]
pub struct NodeRemovePlugin {
    #[default(DEFAULT_KEY_FILTER.to_string())]
    pub key_prefix: String,
}

impl PluginSettings for NodeRemovePlugin {
    fn build_plugin(&self, _: Option<&prometheus::Registry>) -> Fallible<BoxedPlugin> {
        Ok(new_plugin!(InternalPluginWrapper(self.clone())))
    }
}

impl NodeRemovePlugin {
    /// Plugin name, for configuration.
    pub const PLUGIN_NAME: &'static str = "node-remove";

    /// Validate plugin configuration and fill in defaults.
    pub fn deserialize_config(cfg: toml::Value) -> Fallible<Box<dyn PluginSettings>> {
        let plugin: Self = cfg.try_into()?;

        ensure!(!plugin.key_prefix.is_empty(), "empty prefix");

        Ok(Box::new(plugin))
    }
}

#[async_trait]
impl InternalPlugin for NodeRemovePlugin {
    const PLUGIN_NAME: &'static str = Self::PLUGIN_NAME;

    async fn run_internal(self: &Self, io: InternalIO) -> Fallible<InternalIO> {
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
    use crate as cincinnati;

    use super::*;
    use cincinnati::testing::{generate_custom_graph, TestMetadata};
    use commons::testing::init_runtime;

    #[test]
    fn ensure_release_remove() -> Fallible<()> {
        let runtime = init_runtime()?;

        let key_prefix = "test_prefix".to_string();
        let key_suffix = "release.remove".to_string();

        let input_graph: cincinnati::Graph = {
            let metadata: TestMetadata = vec![
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
                (1, [].iter().cloned().collect()),
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
            ];
            generate_custom_graph("image", metadata, None)
        };

        let expected_graph: cincinnati::Graph = {
            let metadata: TestMetadata = vec![(1, [].iter().cloned().collect())];

            generate_custom_graph("image", metadata, None)
        };

        let plugin = Box::new(NodeRemovePlugin { key_prefix });
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
}
