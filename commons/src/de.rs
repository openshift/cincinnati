//! Deserializers.

/// Deserialize a log-level from a numerical value.
pub fn de_loglevel<'de, D>(deserializer: D) -> Result<Option<log::LevelFilter>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    use serde::Deserialize;
    let occurrences = String::deserialize(deserializer)?;

    let verbosity = match occurrences.as_str() {
        "" => log::LevelFilter::Warn,
        "v" => log::LevelFilter::Info,
        "vv" => log::LevelFilter::Debug,
        _ => log::LevelFilter::Trace,
    };
    Ok(Some(verbosity))
}
