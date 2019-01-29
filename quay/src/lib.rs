//! Asynchronous client for quay.io v1 API.

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
use std::path::PathBuf;

pub mod v1;

pub fn read_credentials(credentials_path: Option<&PathBuf>) -> Fallible<Option<String>> {
    match &credentials_path {
        Some(pathbuf) => {
            let file =
                File::open(pathbuf).context(format!("could not open '{}'", &pathbuf.display()))?;

            let first_line = BufReader::new(file)
                .lines()
                .nth(0)
                .ok_or_else(|| format_err!("empty credentials."))?;

            let token = first_line?.trim_end().to_string();

            if token.is_empty() {
                bail!("found an empty first line in '{}'", &pathbuf.display())
            }

            Ok(Some(token))
        }
        None => Ok(None),
    }
}
