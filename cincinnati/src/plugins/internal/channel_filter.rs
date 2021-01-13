//! This plugin can be used to filter a graph by a specific channel.
//! It reads the requested channel from the parameters value at key "channel",
//! and the value must match the regex specified at CHANNEL_VALIDATION_REGEX_STR

use crate as cincinnati;

use self::cincinnati::plugins::prelude::*;
use self::cincinnati::plugins::prelude_plugin_impl::*;

use commons::GraphError;
use lazy_static::lazy_static;

static DEFAULT_KEY_FILTER: &str = "io.openshift.upgrades.graph";
static DEFAULT_CHANNEL_KEY: &str = "release.channels";

#[derive(Clone, Debug, Deserialize, SmartDefault)]
#[serde(default)]
pub struct ChannelFilterPlugin {
    #[default(DEFAULT_KEY_FILTER.to_string())]
    pub key_prefix: String,

    #[default(DEFAULT_CHANNEL_KEY.to_string())]
    pub key_suffix: String,
}

impl PluginSettings for ChannelFilterPlugin {
    fn build_plugin(&self, _: Option<&prometheus::Registry>) -> Fallible<BoxedPlugin> {
        Ok(new_plugin!(InternalPluginWrapper(self.clone())))
    }
}

impl ChannelFilterPlugin {
    pub const PLUGIN_NAME: &'static str = "channel-filter";

    /// Validate plugin configuration and fill in defaults.
    pub fn deserialize_config(cfg: toml::Value) -> Fallible<Box<dyn PluginSettings>> {
        let plugin: Self = cfg.try_into()?;

        ensure!(!plugin.key_prefix.is_empty(), "empty channel-key prefix");
        ensure!(!plugin.key_suffix.is_empty(), "empty channel-key suffix");

        Ok(Box::new(plugin))
    }
}

/// Regex for channel label validation.
static CHANNEL_VALIDATION_REGEX_STR: &str = r"^[0-9a-z\-\.]+$";

lazy_static! {
    static ref CHANNEL_VALIDATION_REGEX_RE: regex::Regex =
        regex::Regex::new(&CHANNEL_VALIDATION_REGEX_STR).expect("could not create regex");
}

#[async_trait]
impl InternalPlugin for ChannelFilterPlugin {
    const PLUGIN_NAME: &'static str = Self::PLUGIN_NAME;

    async fn run_internal(self: &Self, internal_io: InternalIO) -> Fallible<InternalIO> {
        let channel = get_multiple_values!(internal_io.parameters, "channel")
            .map_err(|e| GraphError::MissingParams(vec![e.to_string()]))?
            .clone();

        if !CHANNEL_VALIDATION_REGEX_RE.is_match(&channel) {
            Err(GraphError::InvalidParams(format!(
                "channel '{}' does not match regex '{}'",
                channel, CHANNEL_VALIDATION_REGEX_STR
            )))?;
        };

        let mut graph = internal_io.graph;

        let to_remove = {
            graph
                .find_by_fn_mut(|release| {
                    match release {
                        cincinnati::Release::Concrete(concrete_release) => concrete_release
                            .metadata
                            .get_mut(&format!("{}.{}", self.key_prefix, self.key_suffix))
                            .map_or(true, |values| {
                                !values.split(',').any(|value| value.trim() == channel)
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
    use cincinnati::MapImpl;
    use commons::testing::init_runtime;
    use std::collections::HashMap;

    #[test]
    fn ensure_channel_param_validation() {
        let runtime = init_runtime().unwrap();

        let plugin = Box::new(ChannelFilterPlugin {
            key_prefix: "".to_string(),
            key_suffix: "".to_string(),
        });

        struct Datum {
            channels: std::vec::Vec<&'static str>,
            assert_fn: Box<dyn Fn(&Fallible<InternalIO>)>,
        }

        for datum in &mut [
            Datum {
                channels: vec!["validchannel", "validchannel-0", "validchannel-0.0"],
                assert_fn: Box::new(|result| {
                    assert!(result.is_ok(), "result '{:?}' is not an error", result);
                }),
            },
            Datum {
                channels: vec!["", "invalid_channel", "invalid:channel"],
                assert_fn: Box::new(|result| {
                    assert!(result.is_err(), "result '{:?}' is not an error", result);
                }),
            },
        ] {
            for channel in &mut datum.channels {
                let plugin = plugin.clone();
                let future_result = plugin.run_internal(InternalIO {
                    graph: Default::default(),
                    parameters: [("channel", channel)]
                        .iter()
                        .map(|(a, b)| (a.to_string(), b.to_string()))
                        .collect(),
                });
                let result = runtime.block_on(future_result);
                (datum.assert_fn)(&result);
            }
        }
    }

    #[test]
    fn ensure_channel_filter() {
        let runtime = init_runtime().unwrap();

        let key_prefix = "test_prefix".to_string();
        let key_suffix = "channels".to_string();

        let plugin = Box::new(ChannelFilterPlugin {
            key_prefix: key_prefix.clone(),
            key_suffix: key_suffix.clone(),
        });

        fn generate_test_metadata(
            key_prefix: &str,
            key_suffix: &str,
        ) -> Vec<(usize, MapImpl<String, String>)> {
            [
                (
                    0,
                    [(
                        format!("{}.{}", &key_prefix, &key_suffix),
                        String::from("a, c"),
                    )]
                    .iter()
                    .cloned()
                    .collect(),
                ),
                (
                    1,
                    [(
                        format!("{}.{}", &key_prefix, &key_suffix),
                        String::from("a, c"),
                    )]
                    .iter()
                    .cloned()
                    .collect(),
                ),
                (
                    2,
                    [(
                        format!("{}.{}", &key_prefix, &key_suffix),
                        String::from("b, c"),
                    )]
                    .iter()
                    .cloned()
                    .collect(),
                ),
                (
                    3,
                    [(
                        format!("{}.{}", &key_prefix, &key_suffix),
                        String::from("b, c"),
                    )]
                    .iter()
                    .cloned()
                    .collect(),
                ),
                (4, [].iter().cloned().collect()),
            ]
            .iter()
            .cloned()
            .collect()
        }

        struct Datum {
            pub description: String,
            pub parameters: HashMap<String, String>,
            pub input_graph: cincinnati::Graph,
            pub expected_graph: cincinnati::Graph,
        }

        let data = vec![
            Datum {
                description: String::from("filter graph by channel=A"),
                parameters: [("channel", "a")]
                    .iter()
                    .cloned()
                    .map(|(a, b)| (a.to_string(), b.to_string()))
                    .collect(),
                input_graph: {
                    let metadata = generate_test_metadata(&key_prefix, &key_suffix);
                    generate_custom_graph("image", metadata, Some(vec![(0, 1), (1, 2), (2, 3)]))
                },
                expected_graph: {
                    let metadata: Vec<(usize, MapImpl<String, String>)> = [
                        (
                            0,
                            [(
                                format!("{}.{}", &key_prefix, &key_suffix),
                                String::from("a, c"),
                            )]
                            .iter()
                            .cloned()
                            .collect(),
                        ),
                        (
                            1,
                            [(
                                format!("{}.{}", &key_prefix, &key_suffix),
                                String::from("a, c"),
                            )]
                            .iter()
                            .cloned()
                            .collect(),
                        ),
                    ]
                    .iter()
                    .cloned()
                    .collect();

                    generate_custom_graph("image", metadata, None)
                },
            },
            Datum {
                description: String::from("filter graph by channel=B"),
                parameters: [("channel", "b")]
                    .iter()
                    .cloned()
                    .map(|(a, b)| (a.to_string(), b.to_string()))
                    .collect(),
                input_graph: {
                    let metadata = generate_test_metadata(&key_prefix, &key_suffix);
                    generate_custom_graph("image", metadata, Some(vec![(0, 1), (1, 2), (2, 3)]))
                },
                expected_graph: {
                    let metadata: Vec<(usize, MapImpl<String, String>)> = [
                        (
                            2,
                            [(
                                format!("{}.{}", &key_prefix, &key_suffix),
                                String::from("b, c"),
                            )]
                            .iter()
                            .cloned()
                            .collect(),
                        ),
                        (
                            3,
                            [(
                                format!("{}.{}", &key_prefix, &key_suffix),
                                String::from("b, c"),
                            )]
                            .iter()
                            .cloned()
                            .collect(),
                        ),
                    ]
                    .iter()
                    .cloned()
                    .collect();

                    generate_custom_graph("image", metadata, None)
                },
            },
            Datum {
                description: String::from("filter graph by channel=C"),
                parameters: [("channel", "c")]
                    .iter()
                    .cloned()
                    .map(|(a, b)| (a.to_string(), b.to_string()))
                    .collect(),
                input_graph: {
                    let metadata = generate_test_metadata(&key_prefix, &key_suffix);
                    generate_custom_graph("image", metadata, Some(vec![(0, 1), (1, 2), (2, 3)]))
                },
                expected_graph: {
                    let metadata: Vec<(usize, MapImpl<String, String>)> = [
                        (
                            0,
                            [(
                                format!("{}.{}", &key_prefix, &key_suffix),
                                String::from("a, c"),
                            )]
                            .iter()
                            .cloned()
                            .collect(),
                        ),
                        (
                            1,
                            [(
                                format!("{}.{}", &key_prefix, &key_suffix),
                                String::from("a, c"),
                            )]
                            .iter()
                            .cloned()
                            .collect(),
                        ),
                        (
                            2,
                            [(
                                format!("{}.{}", &key_prefix, &key_suffix),
                                String::from("b, c"),
                            )]
                            .iter()
                            .cloned()
                            .collect(),
                        ),
                        (
                            3,
                            [(
                                format!("{}.{}", &key_prefix, &key_suffix),
                                String::from("b, c"),
                            )]
                            .iter()
                            .cloned()
                            .collect(),
                        ),
                    ]
                    .iter()
                    .cloned()
                    .collect();

                    generate_custom_graph("image", metadata, None)
                },
            },
        ];

        for (i, datum) in data.into_iter().enumerate() {
            println!("processing data set #{}: '{}'", i, datum.description);
            let plugin = plugin.clone();
            let future_processed_graph = plugin.run_internal(InternalIO {
                graph: datum.input_graph,
                parameters: datum.parameters,
            });

            let processed_graph = runtime
                .block_on(future_processed_graph)
                .expect("plugin run failed")
                .graph;

            assert_eq!(datum.expected_graph, processed_graph);
        }
    }
}
