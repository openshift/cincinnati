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

use std::net::IpAddr;
use std::num::ParseIntError;
use std::str::FromStr;
use std::time::Duration;

#[derive(Debug, StructOpt)]
pub struct Options {
    #[structopt(short = "v", parse(from_occurrences))]
    /// Verbosity level
    pub verbosity: u64,

    #[structopt(long = "registry", default_value = "http://localhost:5000")]
    /// URL for the container image registry
    pub registry: String,

    #[structopt(long = "repository", default_value = "openshift")]
    /// Name of the container image repository
    pub repository: String,

    #[structopt(long = "period", default_value = "30", parse(try_from_str = "parse_duration"))]
    /// Duration of the pause (in seconds) between scans of the registry
    pub period: Duration,

    #[structopt(long = "address", default_value = "127.0.0.1")]
    /// Address on which the server will listen
    pub address: IpAddr,

    #[structopt(long = "port", default_value = "8080")]
    /// Port to which the server will bind
    pub port: u16,
}

fn parse_duration(src: &str) -> Result<Duration, ParseIntError> {
    Ok(Duration::from_secs(u64::from_str(src)?))
}
