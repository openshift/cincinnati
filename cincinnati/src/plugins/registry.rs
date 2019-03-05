//! Plugin registry.
//!
//! This registry relies on a static list of all available plugin,
//! referenced by name. It is used for configuration purposes.

#![allow(clippy::implicit_hasher)]

use super::internal::edge_add_remove::EdgeAddRemovePlugin;
use super::internal::metadata_fetch_quay::QuayMetadataFetchPlugin;
use super::internal::node_remove::NodeRemovePlugin;
use super::{Plugin, PluginIO};
use failure::Fallible;
use std::collections::HashMap;

/// Validate configuration for a plugin and fill in defaults.
pub fn sanitize_config(cfg: &mut HashMap<String, String>) -> Fallible<()> {
    let kind = cfg.get("name").cloned().unwrap_or_default();
    match kind.as_str() {
        EdgeAddRemovePlugin::PLUGIN_NAME => EdgeAddRemovePlugin::sanitize_config(cfg),
        NodeRemovePlugin::PLUGIN_NAME => NodeRemovePlugin::sanitize_config(cfg),
        QuayMetadataFetchPlugin::PLUGIN_NAME => QuayMetadataFetchPlugin::sanitize_config(cfg),
        x => bail!("unknown plugin '{}'", x),
    }
}

/// Try to build a plugin from runtime settings.
pub fn try_from_settings(settings: &HashMap<String, String>) -> Fallible<Box<Plugin<PluginIO>>> {
    let kind = settings.get("name").cloned().unwrap_or_default();
    match kind.as_str() {
        EdgeAddRemovePlugin::PLUGIN_NAME => EdgeAddRemovePlugin::from_settings(settings),
        NodeRemovePlugin::PLUGIN_NAME => NodeRemovePlugin::from_settings(settings),
        QuayMetadataFetchPlugin::PLUGIN_NAME => QuayMetadataFetchPlugin::from_settings(settings),
        x => bail!("unknown plugin '{}'", x),
    }
}
