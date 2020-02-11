//! Common functionality for graph-builder tests

use crate as graph_builder;
use graph_builder::plugins::release_scrape_dockerv2::registry::{self};

fn init_logger() {
    let _ = env_logger::try_init_from_env(env_logger::Env::default());
}

pub fn common_init() -> (tokio::runtime::Runtime, registry::cache::Cache) {
    init_logger();
    (
        tokio::runtime::Runtime::new().unwrap(),
        registry::cache::new(),
    )
}

pub fn remove_metadata_by_key(releases: &mut Vec<graph_builder::release::Release>, key: &str) {
    for release in releases.iter_mut() {
        release.metadata.metadata.remove(key);
    }
}
