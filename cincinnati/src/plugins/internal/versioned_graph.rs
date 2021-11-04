use self::cincinnati::plugins::prelude_plugin_impl::*;
use crate as cincinnati;
use commons::{CINCINNATI_VERSION, MIN_CINCINNATI_VERSION};

#[derive(Debug, Serialize, Deserialize, SmartDefault)]
#[serde(default)]
pub struct VersionedGraph {
    version: i32,
    #[serde(flatten)]
    graph: cincinnati::Graph,
}

impl VersionedGraph {
    pub const PLUGIN_NAME: &'static str = "versioned-graph";

    pub fn versioned_graph(io: &InternalIO) -> Fallible<VersionedGraph> {
        let min_version = match CINCINNATI_VERSION.get(*MIN_CINCINNATI_VERSION) {
            Some(version) => *version,
            None => bail!("error parsing minimum cincinnati version"),
        };
        Ok(VersionedGraph {
            version: match io.parameters.get("content_type") {
                None => min_version,
                Some(v) => match CINCINNATI_VERSION.get(v.as_str()) {
                    Some(version) => *version,
                    None => min_version,
                },
            },
            graph: io.graph.clone(),
        })
    }
}
