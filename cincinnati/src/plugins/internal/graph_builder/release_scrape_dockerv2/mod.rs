//! This plugin scrapes a Docker V2 compatible registry repository for release images.

pub mod plugin;
pub mod registry;

pub use plugin::{
    ReleaseScrapeDockerv2Plugin, ReleaseScrapeDockerv2Settings, DEFAULT_FETCH_CONCURRENCY,
    DEFAULT_MANIFESTREF_KEY, DEFAULT_SCRAPE_REGISTRY, DEFAULT_SCRAPE_REPOSITORY,
};
