//! Test helpers.

use failure::Fallible;
use tokio::runtime::Runtime;

/// Initialize logging.
pub fn init_logger() -> Fallible<()> {
    env_logger::try_init_from_env(env_logger::Env::default())?;
    Ok(())
}

/// Initialize a tokio runtime for tests, with logging.
pub fn init_runtime() -> Fallible<Runtime> {
    let _ = init_logger();
    Runtime::new().map_err(failure::Error::from)
}

/// Register a dummy gauge, with given value.
pub fn dummy_gauge(registry: &prometheus::Registry, value: f64) -> Fallible<()> {
    let test_gauge = prometheus::Gauge::new("dummy_gauge", "dummy help")?;
    test_gauge.set(value);
    registry.register(Box::new(test_gauge))?;
    Ok(())
}
