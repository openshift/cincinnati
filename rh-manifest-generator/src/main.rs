//! This program reads a given Cargo.lock file and emits a manifest as desribed at
//! https://mojo.redhat.com/docs/DOC-1195158#jive_content_id_NonGolang_dependencies

use anyhow::{bail, Result};
use cargo_lock::{Lockfile, Package};
use std::io::Write;
use std::sync::Arc;

fn main() -> Result<()> {
    let lockfile = Lockfile::load("Cargo.lock")?;
    let mut rh_manifest = std::fs::File::create("rh-manifest.txt")?;

    for package in lockfile.packages {
        let url = match &package.source {
            Some(_) => get_url(&package)?,
            None => {
                println!("{} doesn't have a source, skipping...", package.name);
                continue;
            }
        };

        let line = format!("{} {} {}\n", package.name, package.version, url);
        rh_manifest.write_all(line.as_bytes())?;
    }

    Ok(())
}

const CRATES_IO_INDEX_CONFIG_URL: &str =
    "https://raw.githubusercontent.com/rust-lang/crates.io-index/master/config.json";

#[allow(dead_code)]
#[derive(serde::Deserialize)]
struct RegistryConfig {
    dl: String,
    api: String,
}

fn get_dl_url() -> Result<String> {
    println!("Getting config from '{}'", CRATES_IO_INDEX_CONFIG_URL);
    let config_json = reqwest::blocking::get(CRATES_IO_INDEX_CONFIG_URL)?;
    let config: RegistryConfig = serde_json::from_str(&config_json.text()?)?;
    Ok(config.dl)
}

lazy_static::lazy_static! {
    static ref DEFAULT_REGISTRY_DL_URL: Arc<Result<String>> = {
        Arc::new(get_dl_url())
    };
}

fn get_url(package: &Package) -> Result<String> {
    let source = match &package.source {
        Some(source) => source,
        None => bail!("{} doesn't have a source", package.name),
    };

    let url = if source.is_default_registry() {
        format!(
            "{}/{}/{}/download",
            match &*DEFAULT_REGISTRY_DL_URL.clone() {
                Ok(dl_url) => dl_url,
                Err(e) => bail!("{}", e),
            },
            &package.name,
            &package.version
        )
    } else if source.is_git() {
        source.to_string()
    } else {
        bail!("unhandled source for package: {:?}", &package);
    };

    Ok(url)
}
