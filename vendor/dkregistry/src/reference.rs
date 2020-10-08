//! Parser for `docker://` URLs.
//!
//! This module provides support for parsing image references.
//!
//! ## Example
//!
//! ```rust
//! # extern crate dkregistry;
//! # fn main() {
//! # fn run() -> dkregistry::errors::Result<()> {
//! #
//! use std::str::FromStr;
//! use dkregistry::reference::Reference;
//!
//! // Parse an image reference
//! let dkref = Reference::from_str("docker://busybox")?;
//! assert_eq!(dkref.registry(), "registry-1.docker.io");
//! assert_eq!(dkref.repository(), "library/busybox");
//! assert_eq!(dkref.version(), "latest");
//! #
//! # Ok(())
//! # };
//! # run().unwrap();
//! # }
//! ```
//!
//!

// The `docker://` schema is not officially documented, but has a reference implementation:
// https://github.com/docker/distribution/blob/v2.6.1/reference/reference.go

use regex;
use std::collections::VecDeque;
use std::str::FromStr;
use std::{fmt, str};

pub static DEFAULT_REGISTRY: &str = "registry-1.docker.io";
static DEFAULT_TAG: &str = "latest";
static DEFAULT_SCHEME: &str = "docker";

/// Image version, either a tag or a digest.
#[derive(Clone)]
pub enum Version {
    Tag(String),
    Digest(String, String),
}

#[derive(thiserror::Error, Debug)]
pub enum VersionParseError {
    #[error("wrong digest format: checksum missing")]
    WrongDigestFormat,
    #[error("unknown prefix: digest must start from : or @")]
    UnknownPrefix,
    #[error("empty string is invalid digest")]
    Empty,
}

impl str::FromStr for Version {
    type Err = VersionParseError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let v = match s.chars().nth(0) {
            Some(':') => Version::Tag(s.trim_start_matches(':').to_string()),
            Some('@') => {
                let r: Vec<&str> = s.trim_start_matches('@').splitn(2, ':').collect();
                if r.len() != 2 {
                    return Err(VersionParseError::WrongDigestFormat);
                };
                Version::Digest(r[0].to_string(), r[1].to_string())
            }
            Some(_) => return Err(VersionParseError::UnknownPrefix),
            None => return Err(VersionParseError::Empty),
        };
        Ok(v)
    }
}

impl Default for Version {
    fn default() -> Self {
        Version::Tag("latest".to_string())
    }
}

impl fmt::Debug for Version {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        let v = match *self {
            Version::Tag(ref s) => ":".to_string() + s,
            Version::Digest(ref t, ref d) => "@".to_string() + t + ":" + d,
        };
        write!(f, "{}", v)
    }
}

impl fmt::Display for Version {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        let v = match *self {
            Version::Tag(ref s) => s.to_string(),
            Version::Digest(ref t, ref d) => t.to_string() + ":" + d,
        };
        write!(f, "{}", v)
    }
}

/// A registry image reference.
#[derive(Clone, Debug, Default)]
pub struct Reference {
    has_schema: bool,
    raw_input: String,
    registry: String,
    repository: String,
    version: Version,
}

impl Reference {
    pub fn new(registry: Option<String>, repository: String, version: Option<Version>) -> Self {
        let reg = registry.unwrap_or_else(|| DEFAULT_REGISTRY.to_string());
        let ver = version.unwrap_or_else(|| Version::Tag(DEFAULT_TAG.to_string()));
        Self {
            has_schema: false,
            raw_input: "".into(),
            registry: reg,
            repository,
            version: ver,
        }
    }

    pub fn registry(&self) -> String {
        self.registry.clone()
    }

    pub fn repository(&self) -> String {
        self.repository.clone()
    }

    pub fn version(&self) -> String {
        self.version.to_string()
    }

    pub fn to_raw_string(&self) -> String {
        self.raw_input.clone()
    }

    //TODO(lucab): move this to a real URL type
    pub fn to_url(&self) -> String {
        format!(
            "{}://{}/{}{:?}",
            DEFAULT_SCHEME, self.registry, self.repository, self.version
        )
    }
}

impl fmt::Display for Reference {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        write!(f, "{}/{}{:?}", self.registry, self.repository, self.version)
    }
}

impl str::FromStr for Reference {
    type Err = ReferenceParseError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        parse_url(s)
    }
}

#[derive(thiserror::Error, Debug)]
pub enum ReferenceParseError {
    #[error("missing image name")]
    MissingImageName,
    #[error("version parse error")]
    VersionParse(#[from] VersionParseError),
    #[error("empty image name")]
    EmptyImageName,
    #[error("component '{component}' does not conform to regex '{regex}'")]
    RegexViolation {
        regex: &'static str,
        component: String,
    },
    #[error("empty repository name")]
    EmptyRepositoryName,
    #[error("repository name too long")]
    RepositoryNameTooLong,
}

fn parse_url(input: &str) -> Result<Reference, ReferenceParseError> {
    // TODO(lucab): investigate using a grammar-based parser.
    let mut rest = input;

    // Detect and remove schema.
    let has_schema = rest.starts_with("docker://");
    if has_schema {
        rest = input.trim_start_matches("docker://");
    };

    // Split path components apart and retain non-empty ones.
    let mut components: VecDeque<String> = rest
        .split('/')
        .filter(|s| !s.is_empty())
        .map(String::from)
        .collect();

    // Figure out if the first component is a registry String, and assume the
    // default registry if it's not.
    let first = components
        .pop_front()
        .ok_or(ReferenceParseError::MissingImageName)?;

    let registry = if regex::Regex::new(r"(?x)
        ^
        # hostname
        (([a-zA-Z0-9]|[a-zA-Z0-9][a-zA-Z0-9\-]*[a-zA-Z0-9])\.)+([A-Za-z0-9]|[A-Za-z0-9][A-Za-z0-9\-]*[A-Za-z0-9])

        # optional port
        ([:][0-9]{1,6})?
        $
    ").expect("hardcoded regex is invalid").is_match(&first) {
        first
    } else {
        components.push_front(first);
        DEFAULT_REGISTRY.to_string()
    };

    // Take image name and extract tag or digest-ref, if any.
    let last = components
        .pop_back()
        .ok_or(ReferenceParseError::MissingImageName)?;
    let (image_name, version) = match (last.rfind('@'), last.rfind(':')) {
        (Some(i), _) | (None, Some(i)) => {
            let s = last.split_at(i);
            (String::from(s.0), Version::from_str(s.1)?)
        }
        (None, None) => (last, Version::default()),
    };
    if image_name.is_empty() {
        return Err(ReferenceParseError::EmptyImageName);
    }

    // Handle images in default library namespace, that is:
    // `ubuntu` -> `library/ubuntu`
    if components.is_empty() && &registry == DEFAULT_REGISTRY {
        components.push_back("library".to_string());
    }
    components.push_back(image_name);

    // Check if all path components conform to the regex at
    // https://docs.docker.com/registry/spec/api/#overview.
    const REGEX: &'static str = "^[a-z0-9]+(?:[._-][a-z0-9]+)*$";
    let path_re = regex::Regex::new(REGEX).expect("hardcoded regex is invalid");
    components.iter().try_for_each(|component| {
        if !path_re.is_match(component) {
            return Err(ReferenceParseError::RegexViolation {
                component: component.clone(),
                regex: REGEX,
            });
        };

        Ok(())
    })?;

    // Re-assemble repository name.
    let repository = components.into_iter().collect::<Vec<_>>().join("/");
    if repository.is_empty() {
        return Err(ReferenceParseError::EmptyRepositoryName);
    }
    if repository.len() > 127 {
        return Err(ReferenceParseError::RepositoryNameTooLong);
    }

    Ok(Reference {
        has_schema,
        raw_input: input.to_string(),
        registry,
        repository,
        version,
    })
}
