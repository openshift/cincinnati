//! Asynchronous client for quay.io v1 API.

#[macro_use]
extern crate serde_derive;

use anyhow::{bail, format_err, Context, Result as Fallible};
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;

pub mod v1;

pub fn read_credentials<P>(path: P) -> Fallible<String>
where
    P: AsRef<Path>,
{
    let filepath = path.as_ref();
    let file = File::open(filepath).context(format!("could not open '{}'", filepath.display()))?;

    let first_line = BufReader::new(file)
        .lines()
        .nth(0)
        .ok_or_else(|| format_err!("empty credentials."))?;

    let token = first_line?.trim_end().to_string();

    if token.is_empty() {
        bail!("found an empty first line in '{}'", filepath.display())
    }

    Ok(token)
}
