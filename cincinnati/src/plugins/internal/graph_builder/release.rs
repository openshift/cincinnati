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

use commons::prelude_errors::*;
use itertools::Itertools;
use log::{trace, warn};
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
pub fn create_graph(releases: Vec<Release>) -> Result<cincinnati::Graph, Error> {
    let mut graph = cincinnati::Graph::default();

    releases
        .into_iter()
        .inspect(|release| trace!("Adding a release to the graph '{:?}'", release))
        .map(|release| {
            Ok((
                release.metadata.next.clone(),
                release.metadata.previous.clone(),
                release.metadata.version.build.clone(),
                graph.add_release(release)?,
            ))
        })
        .collect::<Vec<Fallible<_>>>()
        .into_iter()
        .try_for_each(|result| {
            let (next, previous, current_build, current) = result?;

            previous
                .into_iter()
                .map(|mut previous| {
                    previous.build = current_build.clone();
                    previous
                })
                .try_for_each(|version| -> Fallible<()> {
                    let previous = match graph.find_by_version(&version.to_string()) {
                        Some(id) => id,
                        None => {
                            warn!("Adding abstract release for {}", version.to_string());
                            graph.add_release(cincinnati::Release::Abstract(
                                cincinnati::AbstractRelease {
                                    version: version.to_string(),
                                },
                            ))?
                        }
                    };

                    if let Err(e) = graph.add_edge(&previous, &current) {
                        if let Some(eae) = e.downcast_ref::<cincinnati::errors::EdgeAlreadyExists>()
                        {
                            warn!("{}", eae);
                        } else {
                            return Err(e);
                        }
                    };

                    Ok(())
                })?;

            next.into_iter()
                .map(|mut next| {
                    next.build = current_build.clone();
                    next
                })
                .try_for_each(|version| -> Fallible<()> {
                    let next = match graph.find_by_version(&version.to_string()) {
                        Some(id) => id,
                        None => {
                            warn!("Adding abstract release for {}", version.to_string());
                            graph.add_release(cincinnati::Release::Abstract(
                                cincinnati::AbstractRelease {
                                    version: version.to_string(),
                                },
                            ))?
                        }
                    };

                    if let Err(e) = graph.add_edge(&&current, &next) {
                        if let Some(eae) = e.downcast_ref::<cincinnati::errors::EdgeAlreadyExists>()
                        {
                            warn!("{:?}", eae);
                        } else {
                            return Err(e);
                        }
                    };

                    Ok(())
                })
        })?;

    Ok(graph)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    #[test]
    fn create_graph_tolerates_nonexistent_edges() -> Fallible<()> {
        let releases = vec![Release {
            source: "test-0.0.1".to_string(),
            metadata: Metadata {
                kind: MetadataKind::V0,
                version: semver::Version::from((0, 0, 1)),
                next: Default::default(),
                previous: vec![semver::Version::from((0, 0, 0))],
                metadata: Default::default(),
            },
        }];

        let mut graph = create_graph(releases).unwrap();

        assert_eq!(graph.prune_abstract(), 1);

        Ok(())
    }

    #[test]
    fn create_graph_tolerates_duplicate_edges() -> Fallible<()> {
        let releases = vec![Release {
            source: "test-0.0.1".to_string(),
            metadata: Metadata {
                kind: MetadataKind::V0,
                version: semver::Version::from((0, 0, 1)),
                next: vec![
                    semver::Version::from((0, 0, 2)),
                    semver::Version::from((0, 0, 2)),
                ],
                previous: vec![
                    semver::Version::from((0, 0, 0)),
                    semver::Version::from((0, 0, 0)),
                ],
                metadata: Default::default(),
            },
        }];

        create_graph(releases).unwrap();

        Ok(())
    }

    #[test]
    fn create_graph_duplicate_releases() -> Fallible<()> {
        let mut valid = BTreeMap::new();
        valid.insert(
            "io.openshift.upgrades.graph.release.manifestref".to_string(),
            "sha256:872227b971ddfe537yb847cb7hed7caa464b81b565e5aadd".to_string(),
        );
        let mut invalid = BTreeMap::new();
        invalid.insert(
            "io.openshift.upgrades.graph.release.manifestref".to_string(),
            "sha256:872227b974324x2x47cb7hed7caa464b81b562r33x2323x23".to_string(),
        );

        let valid_release = Release {
            source: "test-0.0.1".to_string(),
            metadata: Metadata {
                kind: MetadataKind::V0,
                version: semver::Version::from((0, 0, 1)),
                next: Default::default(),
                previous: Default::default(),
                metadata: valid,
            },
        };

        let invalid_release = Release {
            source: "test-0.0.1".to_string(),
            metadata: Metadata {
                kind: MetadataKind::V0,
                version: semver::Version::from((0, 0, 1)),
                next: Default::default(),
                previous: Default::default(),
                metadata: invalid,
            },
        };

        let releases_no_error = vec![valid_release.clone(), valid_release.clone()];
        let releases_error = vec![valid_release, invalid_release];

        assert!(
            create_graph(releases_no_error).is_ok(),
            "create graph should not throw on encountering duplicate releases"
        );
        assert!(
            create_graph(releases_error).is_err(),
            "create graph should throw error on SHA sum mismatch for duplicate releases"
        );
        Ok(())
    }
}
