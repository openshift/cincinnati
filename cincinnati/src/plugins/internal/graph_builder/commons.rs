#[cfg(test)]
pub mod tests {
    //! Common functionality for graph-builder tests

    use crate as cincinnati;

    use cincinnati::plugins::internal::graph_builder::release;
    use cincinnati::plugins::internal::graph_builder::release_scrape_dockerv2::registry;

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
}
