//! Deserializers.

/// Deserialize a log-level from a numerical value.
pub fn de_loglevel<'de, D>(deserializer: D) -> Result<Option<log::LevelFilter>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    use serde::Deserialize;
    let numlevel = u8::deserialize(deserializer)?;

    let verbosity = match numlevel {
        0 => log::LevelFilter::Warn,
        1 => log::LevelFilter::Info,
        2 => log::LevelFilter::Debug,
        _ => log::LevelFilter::Trace,
    };
    Ok(Some(verbosity))
}
