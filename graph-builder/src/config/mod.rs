//! Configuration lookup, parsing and validation.
//!
//! This module takes care of sourcing configuration options from
//! multiple inputs (CLI and files), merging, and validating them.
//! It contains the following entities:
//!  * "options": configuration fragments (CLI flags, file snippets).
//!  * "app settings": runtime settings, result of config validation.

macro_rules! assign_if_some {
    ( $dst:expr, $src:expr ) => {{
        if let Some(x) = $src {
            $dst = x.into();
        };
    }};
}

mod cli;
mod file;
mod options;
mod settings;

pub use self::settings::AppSettings;

/// Merge configuration options into runtime settings.
///
/// This consumes a generic configuration object, merging its options
/// into runtime settings. It only overlays populated values from config,
/// leaving unset ones preserved as-is from existing settings.
trait MergeOptions<T> {
    /// MergeOptions values from `options` into current settings.
    fn merge(&mut self, options: T);
}
