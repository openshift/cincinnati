use crate::errors::{Error, Result};
use crate::mediatypes;
use crate::v2::*;
use mime;
use reqwest::{self, header, StatusCode, Url};
use std::iter::FromIterator;
use std::str::FromStr;

mod manifest_schema1;
pub use self::manifest_schema1::*;

mod manifest_schema2;
pub use self::manifest_schema2::*;

impl Client {
    /// Fetch an image manifest.
    ///
    /// The name and reference parameters identify the image.
    /// The reference may be either a tag or digest.
    pub async fn get_manifest(&self, name: &str, reference: &str) -> Result<Manifest> {
        self.get_manifest_and_ref(name, reference)
            .await
            .map(|(manifest, _)| manifest)
    }

    /// Fetch an image manifest and return it with its digest.
    ///
    /// The name and reference parameters identify the image.
    /// The reference may be either a tag or digest.
    pub async fn get_manifest_and_ref(
        &self,
        name: &str,
        reference: &str,
    ) -> Result<(Manifest, Option<String>)> {
        let url = self.build_url(name, reference)?;

        let accept_headers = build_accept_headers(&self.index);

        let client_spare0 = self.clone();

        let res = self
            .build_reqwest(Method::GET, url.clone())
            .headers(accept_headers)
            .send()
            .await?;

        let status = res.status();
        trace!("GET '{}' status: {:?}", res.url(), status);

        match status {
            StatusCode::OK => {}
            _ => return Err(Error::UnexpectedHttpStatus(status)),
        }

        let headers = res.headers();
        let content_digest = match headers.get("docker-content-digest") {
            Some(content_digest_value) => Some(content_digest_value.to_str()?.to_string()),
            None => {
                debug!("cannot find manifestref in headers");
                None
            }
        };

        let header_content_type = headers.get(header::CONTENT_TYPE);
        let media_type = evaluate_media_type(header_content_type, &url)?;

        trace!(
            "content-type: {:?}, media-type: {:?}",
            header_content_type,
            media_type
        );

        match media_type {
            mediatypes::MediaTypes::ManifestV2S1Signed => Ok((
                res.json::<ManifestSchema1Signed>()
                    .await
                    .map(Manifest::S1Signed)?,
                content_digest,
            )),
            mediatypes::MediaTypes::ManifestV2S2 => {
                let m = res.json::<ManifestSchema2Spec>().await?;
                Ok((
                    m.fetch_config_blob(client_spare0, name.to_string())
                        .await
                        .map(Manifest::S2)?,
                    content_digest,
                ))
            }
            mediatypes::MediaTypes::ManifestList => Ok((
                res.json::<ManifestList>().await.map(Manifest::ML)?,
                content_digest,
            )),
            unsupported => Err(Error::UnsupportedMediaType(unsupported)),
        }
    }

    fn build_url(&self, name: &str, reference: &str) -> Result<Url> {
        let ep = format!(
            "{}/v2/{}/manifests/{}",
            self.base_url.clone(),
            name,
            reference
        );
        reqwest::Url::parse(&ep).map_err(|e| Error::from(e))
    }

    /// Fetch content digest for a particular tag.
    pub async fn get_manifestref(&self, name: &str, reference: &str) -> Result<Option<String>> {
        let url = self.build_url(name, reference)?;

        let accept_headers = build_accept_headers(&self.index);

        let res = self
            .build_reqwest(Method::HEAD, url)
            .headers(accept_headers)
            .send()
            .await?;

        let status = res.status();
        trace!("HEAD '{}' status: {:?}", res.url(), status);

        match status {
            StatusCode::OK => {}
            _ => return Err(Error::UnexpectedHttpStatus(status)),
        }

        let headers = res.headers();
        let content_digest = match headers.get("docker-content-digest") {
            Some(content_digest_value) => Some(content_digest_value.to_str()?.to_string()),
            None => {
                debug!("cannot find manifestref in headers");
                None
            }
        };
        Ok(content_digest)
    }

    /// Check if an image manifest exists.
    ///
    /// The name and reference parameters identify the image.
    /// The reference may be either a tag or digest.
    pub async fn has_manifest(
        &self,
        name: &str,
        reference: &str,
        mediatypes: Option<&[&str]>,
    ) -> Result<Option<mediatypes::MediaTypes>> {
        let url = self.build_url(name, reference)?;
        let accept_types = match mediatypes {
            None => {
                let m = mediatypes::MediaTypes::ManifestV2S2.to_mime();
                vec![m]
            }
            Some(ref v) => to_mimes(v),
        };

        let mut accept_headers = header::HeaderMap::with_capacity(accept_types.len());
        for accept_type in accept_types {
            let header_value = header::HeaderValue::from_str(&accept_type.to_string())
                .expect("mime type is always valid header value");
            accept_headers.insert(header::ACCEPT, header_value);
        }

        trace!("HEAD {:?}", url);

        let r = self
            .build_reqwest(Method::HEAD, url.clone())
            .headers(accept_headers)
            .send()
            .await
            .map_err(Error::from)?;

        let status = r.status();

        trace!(
            "Manifest check status '{:?}', headers '{:?}",
            r.status(),
            r.headers(),
        );

        match status {
            StatusCode::MOVED_PERMANENTLY
            | StatusCode::TEMPORARY_REDIRECT
            | StatusCode::FOUND
            | StatusCode::OK => {
                let media_type =
                    evaluate_media_type(r.headers().get(header::CONTENT_TYPE), &r.url())?;
                trace!("Manifest media-type: {:?}", media_type);
                Ok(Some(media_type))
            }
            StatusCode::NOT_FOUND => Ok(None),
            _ => Err(Error::UnexpectedHttpStatus(status)),
        }
    }
}

fn to_mimes(v: &[&str]) -> Vec<mime::Mime> {
    let res = v
        .iter()
        .filter_map(|x| {
            let mtype = mediatypes::MediaTypes::from_str(x);
            match mtype {
                Ok(m) => Some(m.to_mime()),
                _ => None,
            }
        })
        .collect();
    res
}

// Evaluate the `MediaTypes` from the the request header.
fn evaluate_media_type(
    content_type: Option<&reqwest::header::HeaderValue>,
    url: &Url,
) -> Result<mediatypes::MediaTypes> {
    let header_content_type = content_type
        .map(|hv| hv.to_str())
        .map(std::result::Result::unwrap_or_default);

    let is_pulp_based = url.path().starts_with("/pulp/docker/v2");

    match (header_content_type, is_pulp_based) {
        (Some(header_value), false) => {
            mediatypes::MediaTypes::from_str(header_value).map_err(Into::into)
        }
        (None, false) => Err(Error::MediaTypeSniff),
        (Some(header_value), true) => {
            // TODO: remove this workaround once Satellite returns a proper content-type here
            match header_value {
                "application/x-troff-man" => {
                    trace!("Applying workaround for pulp-based registries, e.g. Satellite");
                    mediatypes::MediaTypes::from_str(
                        "application/vnd.docker.distribution.manifest.v1+prettyjws",
                    )
                    .map_err(Into::into)
                }
                _ => {
                    debug!("Received content-type '{}' from pulp-based registry. Feeling lucky and trying to parse it...", header_value);
                    mediatypes::MediaTypes::from_str(header_value).map_err(Into::into)
                }
            }
        }
        (None, true) => {
            trace!("Applying workaround for pulp-based registries, e.g. Satellite");
            mediatypes::MediaTypes::from_str(
                "application/vnd.docker.distribution.manifest.v1+prettyjws",
            )
            .map_err(Into::into)
        }
    }
}

fn build_accept_headers(registry: &str) -> header::HeaderMap {
    // GCR incorrectly parses `q` parameters, so we use special Accept for it.
    // Bug: https://issuetracker.google.com/issues/159827510.
    // TODO: when bug is fixed, this workaround should be removed.
    let no_q = registry == "gcr.io" || registry.ends_with(".gcr.io");

    let accepted_types = vec![
        // accept header types and their q value, as documented in
        // https://tools.ietf.org/html/rfc7231#section-5.3.2
        (mediatypes::MediaTypes::ManifestV2S2, 0.5),
        (mediatypes::MediaTypes::ManifestV2S1Signed, 0.4),
        // TODO(steveeJ): uncomment this when all the Manifest methods work for it
        // mediatypes::MediaTypes::ManifestList,
    ];

    let accepted_types_string = accepted_types
        .into_iter()
        .map(|(ty, q)| {
            format!(
                "{}{}",
                ty.to_string(),
                if no_q {
                    String::default()
                } else {
                    format!("; q={}", q)
                }
            )
        })
        .collect::<Vec<_>>()
        .join(",");

    header::HeaderMap::from_iter(vec![(
        header::ACCEPT,
        header::HeaderValue::from_str(&accepted_types_string).expect(
            "should be always valid because both float and mime type only use allowed ASCII chard",
        ),
    )])
}

/// Umbrella type for common actions on the different manifest schema types
#[derive(Debug)]
pub enum Manifest {
    S1Signed(manifest_schema1::ManifestSchema1Signed),
    S2(manifest_schema2::ManifestSchema2),
    ML(manifest_schema2::ManifestList),
}

#[derive(Debug, thiserror::Error)]
pub enum ManifestError {
    #[error("no architecture in manifest")]
    NoArchitecture,
    #[error("architecture mismatch")]
    ArchitectureMismatch,
    #[error("manifest {0} does not support the 'layer_digests' method")]
    LayerDigestsUnsupported(String),
    #[error("manifest {0} does not support the 'architecture' method")]
    ArchitectureNotSupported(String),
}

impl Manifest {
    /// List digests of all layers referenced by this manifest, if available.
    ///
    /// The returned layers list is ordered starting with the base image first.
    pub fn layers_digests(&self, architecture: Option<&str>) -> Result<Vec<String>> {
        match (self, self.architectures(), architecture) {
            (Manifest::S1Signed(m), _, None) => Ok(m.get_layers()),
            (Manifest::S2(m), _, None) => Ok(m.get_layers()),
            (Manifest::S1Signed(m), Ok(ref self_architectures), Some(ref a)) => {
                let self_a = self_architectures
                    .first()
                    .ok_or(ManifestError::NoArchitecture)?;
                if self_a != a {
                    return Err(ManifestError::ArchitectureMismatch.into());
                }
                Ok(m.get_layers())
            }
            (Manifest::S2(m), Ok(ref self_architectures), Some(ref a)) => {
                let self_a = self_architectures
                    .first()
                    .ok_or(ManifestError::NoArchitecture)?;
                if self_a != a {
                    return Err(ManifestError::ArchitectureMismatch.into());
                }
                Ok(m.get_layers())
            }
            // Manifest::ML(_) => TODO(steveeJ),
            _ => Err(ManifestError::LayerDigestsUnsupported(format!("{:?}", self)).into()),
        }
    }

    /// The architectures of the image the manifest points to, if available.
    pub fn architectures(&self) -> Result<Vec<String>> {
        match self {
            Manifest::S1Signed(m) => Ok([m.architecture.clone()].to_vec()),
            Manifest::S2(m) => Ok([m.architecture()].to_vec()),
            // Manifest::ML(_) => TODO(steveeJ),
            _ => Err(ManifestError::ArchitectureNotSupported(format!("{:?}", self)).into()),
        }
    }
}
