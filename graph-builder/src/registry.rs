// Copyright 2018 Alex Crawford
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use cincinnati;
use failure::{Error, ResultExt};
use flate2::read::GzDecoder;
use release;
use reqwest::{self, Url};
use serde_json;
use std::io::Read;
use std::path::Path;
use tar::Archive;

pub struct Release {
    pub source: String,
    pub metadata: release::Metadata,
}

impl Into<cincinnati::Release> for Release {
    fn into(self) -> cincinnati::Release {
        cincinnati::Release::Concrete(cincinnati::ConcreteRelease {
            version: self.metadata.version,
            payload: self.source,
            metadata: self.metadata.metadata,
        })
    }
}

/// Fetches a vector of all release metadata from the given repository, hosted on the given
/// registry.
pub fn fetch_releases(registry: &str, repo: &str) -> Result<Vec<Release>, Error> {
    let mut metadata = Vec::new();
    for tag in fetch_tags(registry, repo)? {
        metadata.push(Release {
            source: format!("{}/{}:{}", registry, repo, tag),
            metadata: fetch_metadata(registry, repo, &tag)?,
        })
    }
    Ok(metadata)
}

#[derive(Debug, Deserialize)]
struct Tags {
    name: String,
    tags: Vec<String>,
}

fn fetch_tags(registry: &str, repo: &str) -> Result<Vec<String>, Error> {
    let base = Url::parse(registry)?;
    let tags: Tags = {
        let mut response = reqwest::get(base.join(&format!("v2/{}/tags/list", repo))?)
            .context("failed to fetch image tags")?;
        ensure!(
            response.status().is_success(),
            "failed to fetch image tags: {}",
            response.status()
        );

        serde_json::from_str(&response.text()?)?
    };

    Ok(tags.tags)
}

#[derive(Debug, Deserialize)]
struct Manifest {
    #[serde(rename = "schemaVersion")]
    schema_version: usize,
    name: String,
    tag: String,
    architecture: String,
    #[serde(rename = "fsLayers")]
    fs_layers: Vec<Layer>,
}

#[derive(Debug, Deserialize)]
struct Layer {
    #[serde(rename = "blobSum")]
    blob_sum: String,
}

fn fetch_metadata(registry: &str, repo: &str, tag: &str) -> Result<release::Metadata, Error> {
    trace!("fetching metadata from {}/{}:{}", registry, repo, tag);

    let base = Url::parse(registry)?;
    let manifest: Manifest = {
        let mut response = reqwest::get(base.join(&format!("v2/{}/manifests/{}", repo, tag))?)
            .context("failed to fetch image manifest")?;
        ensure!(
            response.status().is_success(),
            "failed to fetch image manifest: {}",
            response.status()
        );

        serde_json::from_str(&response.text()?).context("failed to parse image manifest")?
    };

    for layer in manifest.fs_layers {
        match fetch_metadata_from_layer(&base, repo, &layer) {
            Ok(metadata) => return Ok(metadata),
            Err(err) => debug!("metadata document not found in layer: {}", err),
        }
    }

    bail!("metadata document not found in image")
}

fn fetch_metadata_from_layer(
    base: &Url,
    repo: &str,
    layer: &Layer,
) -> Result<release::Metadata, Error> {
    trace!("fetching metadata from {}", layer.blob_sum);

    let response = reqwest::get(base.join(&format!("v2/{}/blobs/{}", repo, layer.blob_sum))?)
        .context("failed to fetch image blob")?;

    ensure!(
        response.status().is_success(),
        "failed to fetch metadata document: {}",
        response.status()
    );

    let mut archive = Archive::new(GzDecoder::new(response));
    match archive
        .entries()?
        .filter_map(|entry| match entry {
            Ok(file) => Some(file),
            Err(err) => {
                debug!("failed to read archive entry: {}", err);
                None
            }
        })
        .find(|file| match file.header().path() {
            Ok(path) => path == Path::new("cincinnati.json"),
            Err(err) => {
                debug!("failed to read file header: {}", err);
                false
            }
        }) {
        Some(mut file) => {
            let mut contents = String::new();
            file.read_to_string(&mut contents)?;
            serde_json::from_str(&contents).context("failed to parse cincinnati.json")
        }
        None => bail!("cincinnati.json not found"),
    }.map_err(Into::into)
}
