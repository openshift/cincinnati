//! Plugins catalog.
//!
//! This catalog relies on a static list of all available plugins,
//! referenced by name. It is used for configuration purposes.

use super::internal::channel_filter::ChannelFilterPlugin;
use super::internal::edge_add_remove::EdgeAddRemovePlugin;
use super::internal::metadata_fetch_quay::QuayMetadataFetchPlugin;
use super::internal::node_remove::NodeRemovePlugin;
use failure::Fallible;
use plugins::BoxedPlugin;
use std::fmt::Debug;

/// Key used to look up plugin-type in a configuration entry.
static CONFIG_PLUGIN_NAME_KEY: &str = "name";

/// Settings for a plugin.
pub trait PluginSettings: Debug + Send {
    /// Build the corresponding plugin for this configuration.
    fn build_plugin(&self) -> Fallible<BoxedPlugin>;
}

/// Validate configuration for a plugin and fill in defaults.
pub fn deserialize_config(cfg: toml::Value) -> Fallible<Box<PluginSettings>> {
    let name = cfg
        .get(CONFIG_PLUGIN_NAME_KEY)
        .ok_or_else(|| format_err!("missing plugin name"))?
        .as_str()
        .ok_or_else(|| format_err!("invalid plugin name value"))?
        .to_string();

    match name.as_str() {
        ChannelFilterPlugin::PLUGIN_NAME => ChannelFilterPlugin::deserialize_config(cfg),
        EdgeAddRemovePlugin::PLUGIN_NAME => EdgeAddRemovePlugin::deserialize_config(cfg),
        NodeRemovePlugin::PLUGIN_NAME => NodeRemovePlugin::deserialize_config(cfg),
        QuayMetadataFetchPlugin::PLUGIN_NAME => QuayMetadataFetchPlugin::deserialize_config(cfg),
        x => bail!("unknown plugin '{}'", x),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deserialize_basic() {
        let empty: toml::Value = toml::from_str("").unwrap();
        deserialize_config(empty).unwrap_err();

        let no_name: toml::Value = toml::from_str("foo = 'bar'").unwrap();
        deserialize_config(no_name).unwrap_err();

        let node_remove_default: toml::Value = toml::from_str("name = 'node-remove'").unwrap();
        let nr_settings = deserialize_config(node_remove_default).unwrap();
        nr_settings.build_plugin().unwrap();

        let cfg = r#"
            name = "quay-metadata"
            repository = "mytest"
        "#;
        let quay_metadata_repo: toml::Value = toml::from_str(cfg).unwrap();
        let qm_settings = deserialize_config(quay_metadata_repo).unwrap();
        qm_settings.build_plugin().unwrap();
    }
}
