//! This plugin parses the humman readable OpenShift secondary metadata format,
//! as used in https://github.com/openshift/cincinnati-graph-data.
//! There is currently no formal specification for this format.

pub mod plugin;

pub use plugin::{
    OpenshiftSecondaryMetadataParserPlugin, OpenshiftSecondaryMetadataParserSettings,
};
