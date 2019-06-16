//! Asynchronous client for quay.io v1 API.

// adding commit 1
#[macro_use]
extern crate failure;
extern crate futures;
extern crate reqwest;
extern crate serde;
#[macro_use]
extern crate serde_derive;
extern crate serde_json;

use failure::Fallible;
use failure::ResultExt;
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
