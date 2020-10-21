//! This module implements the internal plugins

pub mod arch_filter;
pub mod channel_filter;
pub mod cincinnati_graph_fetch;
pub mod edge_add_remove;
pub mod metadata_fetch_quay;
pub mod node_remove;

mod graph_builder;

pub use graph_builder::{
    dkrv2_openshift_secondary_metadata_scraper, github_openshift_secondary_metadata_scraper,
    openshift_secondary_metadata_parser, release_scrape_dockerv2,
};
