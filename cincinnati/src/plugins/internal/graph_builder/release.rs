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

use crate as cincinnati;

use self::cincinnati::MapImpl;

use itertools::Itertools;
use log::trace;
use semver::Version;
use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Release {
    pub source: String,
    pub metadata: Metadata,
}

impl Into<cincinnati::Release> for Release {
    fn into(self) -> cincinnati::Release {
        cincinnati::Release::Concrete(cincinnati::ConcreteRelease {
            version: self.metadata.version.to_string(),
            payload: self.source,
            metadata: self.metadata.metadata,
        })
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct Metadata {
    pub kind: MetadataKind,
    pub version: Version,

    #[serde(default)]
    pub previous: Vec<Version>,
    #[serde(default)]
    pub next: Vec<Version>,
    #[serde(default)]
    pub metadata: MapImpl<String, String>,
}

impl fmt::Display for Metadata {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "Metadata {{ version: {}, previous: [{}], next: [{}], metadata: {:?} }}",
            self.version,
            self.previous.iter().format(", "),
            self.next.iter().format(", "),
            self.metadata,
        )
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub enum MetadataKind {
    #[serde(rename = "cincinnati-metadata-v0")]
    V0,
}

/// Turns a collection of Releases into a Cincinnati Graph
///
/// When processing previous/next release metadata it is assumed that the edge
/// destination has the same build type as the origin.
pub fn create_graph(releases: Vec<Release>) -> Result<cincinnati::Graph, failure::Error> {
    let mut graph = cincinnati::Graph::default();

    releases
        .into_iter()
        .inspect(|release| trace!("Adding a release to the graph '{:?}'", release))
        .try_for_each(|release| {
            let previous = release
                .metadata
                .previous
                .iter()
                .cloned()
                .map(|mut previous| {
                    previous.build = release.metadata.version.build.clone();
                    previous
                })
                .collect::<Vec<_>>();

            let next = release
                .metadata
                .next
                .iter()
                .cloned()
                .map(|mut next| {
                    next.build = release.metadata.version.build.clone();
                    next
                })
                .collect::<Vec<_>>();
            let current = graph.add_release(release)?;

            previous.iter().try_for_each(|version| {
                let previous = match graph.find_by_version(&version.to_string()) {
                    Some(id) => id,
                    None => graph.add_release(cincinnati::Release::Abstract(
                        cincinnati::AbstractRelease {
                            version: version.to_string(),
                        },
                    ))?,
                };
                graph.add_edge(&previous, &current).and_then(|edge| {
                    trace!(
                        "Adding EdgeIndex({}): from {} to {:?}",
                        edge.index(),
                        &version.to_string(),
                        current
                    );
                    Ok(())
                })
            })?;

            next.iter().try_for_each(|version| {
                let next = match graph.find_by_version(&version.to_string()) {
                    Some(id) => id,
                    None => graph.add_release(cincinnati::Release::Abstract(
                        cincinnati::AbstractRelease {
                            version: version.to_string(),
                        },
                    ))?,
                };
                graph.add_edge(&current, &next).and_then(|edge| {
                    trace!(
                        "Adding EdgeIndex({}): from {:?} to {}",
                        edge.index(),
                        current,
                        &version.to_string(),
                    );
                    Ok(())
                })
            })
        })?;

    Ok(graph)
}
