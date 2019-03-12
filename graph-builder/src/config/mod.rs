//! Configuration lookup, parsing and validation.
//!
//! This module takes care of sourcing configuration options from
//! multiple inputs (CLI and files), merging, and validating them.
//! It contains the following entities:
//!  * "options": configuration fragments (CLI flags, file snippets), optional and stringly-typed.
//!  * "unified config": configuration document, result of merging all options and defaults.
//!  * "app settings": runtime settings, result of config validation.

mod cli;
mod file;
mod settings;
mod unified;

pub use self::settings::AppSettings;
