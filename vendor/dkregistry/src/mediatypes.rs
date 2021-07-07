//! Media-types for API objects.

use crate::errors::{Result};
use mime;
use strum::EnumProperty;

// For schema1 types, see https://docs.docker.com/registry/spec/manifest-v2-1/
// For schema2 types, see https://docs.docker.com/registry/spec/manifest-v2-2/

#[derive(EnumProperty, EnumString, ToString, Debug, Hash, PartialEq)]
pub enum MediaTypes {
    /// Manifest, version 2 schema 1.
    #[strum(serialize = "application/vnd.docker.distribution.manifest.v1+json")]
    #[strum(props(Sub = "vnd.docker.distribution.manifest.v1+json"))]
    ManifestV2S1,
    /// Signed manifest, version 2 schema 1.
    #[strum(
        serialize = "application/vnd.docker.distribution.manifest.v1+prettyjws",
        to_string = "application/vnd.docker.distribution.manifest.v1+prettyjws",

        // TODO(steveeJ) find a generic way to handle this form
        serialize = "application/vnd.docker.distribution.manifest.v1+prettyjws; charset=utf-8",
    )]
    #[strum(props(Sub = "vnd.docker.distribution.manifest.v1+prettyjws"))]
    ManifestV2S1Signed,
    /// Manifest, version 2 schema 2.
    #[strum(serialize = "application/vnd.docker.distribution.manifest.v2+json")]
    #[strum(props(Sub = "vnd.docker.distribution.manifest.v2+json"))]
    ManifestV2S2,
    /// Manifest List (aka "fat manifest").
    #[strum(serialize = "application/vnd.docker.distribution.manifest.list.v2+json")]
    #[strum(props(Sub = "vnd.docker.distribution.manifest.list.v2+json"))]
    ManifestList,
    /// Image layer, as a gzip-compressed tar.
    #[strum(serialize = "application/vnd.docker.image.rootfs.diff.tar.gzip")]
    #[strum(props(Sub = "vnd.docker.image.rootfs.diff.tar.gzip"))]
    ImageLayerTgz,
    /// Configuration object for a container.
    #[strum(serialize = "application/vnd.docker.container.image.v1+json")]
    #[strum(props(Sub = "vnd.docker.container.image.v1+json"))]
    ContainerConfigV1,
    /// Generic JSON
    #[strum(serialize = "application/json")]
    #[strum(props(Sub = "json"))]
    ApplicationJson,
}

impl MediaTypes {
    // TODO(lucab): proper error types
    pub fn from_mime(mtype: &mime::Mime) -> Result<Self> {
        match (mtype.type_(), mtype.subtype(), mtype.suffix()) {
            (mime::APPLICATION, mime::JSON, _) => Ok(MediaTypes::ApplicationJson),
            (mime::APPLICATION, subt, Some(suff)) => {
                match (subt.to_string().as_str(), suff.to_string().as_str()) {
                    ("vnd.docker.distribution.manifest.v1", "json") => Ok(MediaTypes::ManifestV2S1),
                    ("vnd.docker.distribution.manifest.v1", "prettyjws") => {
                        Ok(MediaTypes::ManifestV2S1Signed)
                    }
                    ("vnd.docker.distribution.manifest.v2", "json") => Ok(MediaTypes::ManifestV2S2),
                    ("vnd.docker.distribution.manifest.list.v2", "json") => {
                        Ok(MediaTypes::ManifestList)
                    }
                    ("vnd.docker.image.rootfs.diff.tar.gzip", _) => Ok(MediaTypes::ImageLayerTgz),
                    ("vnd.docker.container.image.v1", "json") => Ok(MediaTypes::ContainerConfigV1),
                    _ => return Err(crate::Error::UnknownMimeType(mtype.clone())),
                }
            }
            _ => return Err(crate::Error::UnknownMimeType(mtype.clone())),
        }
    }
    pub fn to_mime(&self) -> mime::Mime {
        match self {
            &MediaTypes::ApplicationJson => Ok(mime::APPLICATION_JSON),
            ref m => {
                if let Some(s) = m.get_str("Sub") {
                    ("application/".to_string() + s).parse()
                } else {
                    "application/star".parse()
                }
            }
        }
        .expect("to_mime should be always successful")
    }
}
