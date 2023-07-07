//! Configuration lookup, parsing and validation.
//!
//! This module takes care of sourcing configuration options from
//! multiple inputs (CLI and files), merging, and validating them.
//! It contains the following entities:
//!  * "options": configuration fragments (CLI flags, file snippets).
//!  * "app settings": runtime settings, result of config validation.

mod cli;
mod file;
mod options;
mod settings;

#[cfg(test)]
pub(crate) use self::file::FileOptions;

pub use self::settings::AppSettings;
