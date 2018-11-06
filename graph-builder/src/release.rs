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

use itertools::Itertools;
use semver::Version;
use std::collections::HashMap;
use std::fmt;

#[derive(Debug, Deserialize, Clone, PartialEq)]
pub struct Metadata {
    pub kind: MetadataKind,
    pub version: Version,

    #[serde(default)]
    pub previous: Vec<Version>,
    #[serde(default)]
    pub next: Vec<Version>,
    #[serde(default)]
    pub metadata: HashMap<String, String>,
}

impl fmt::Display for Metadata {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "Metadata {{ version: {}, previous: [{}], next: [{}], metadata: {:?} }}",
            self.version,
            self.previous.iter().format(", "),
            self.next.iter().format(", "),
            self.metadata
        )
    }
}

#[derive(Debug, Deserialize, Clone, PartialEq)]
pub enum MetadataKind {
    #[serde(rename = "cincinnati-metadata-v0")]
    V0,
}
