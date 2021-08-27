/// Tiny crate to verify message signature and format
use self::cincinnati::plugins::prelude::*;
use crate as cincinnati;
use bytes::Bytes;
use futures::TryFutureExt;
use reqwest::Client;
use serde::Deserialize;
use serde_json;
use std::fs::read_dir;
use std::io::copy;
use std::ops::Range;
use std::path::PathBuf;
use url::Url;

use sequoia_openpgp::parse::{stream::*, Parse};
use sequoia_openpgp::policy::StandardPolicy as P;
use sequoia_openpgp::{Cert, Error, KeyHandle, Result};

// Signature format
#[derive(Deserialize)]
struct SignatureImage {
    #[serde(rename = "docker-manifest-digest")]
    digest: String,
}

#[derive(Deserialize)]
struct SignatureCritical {
    image: SignatureImage,
}

#[derive(Deserialize)]
struct Signature {
    critical: SignatureCritical,
}

/// Keyring is a collection of public keys
pub type Keyring = Vec<Cert>;

// CVO has maxSignatureSearch = 10 in pkg/verify/verify.go
pub static MAX_SIGNATURES: u64 = 10;

/// Create a Keyring from a dir of public keys
pub fn load_public_keys(public_keys_dir: &PathBuf) -> Fallible<Keyring> {
    let mut result: Keyring = vec![];
    for entry in read_dir(public_keys_dir).context("Reading public keys dir")? {
        let path = &entry?.path();
        let path_str = match path.to_str() {
            None => continue,
            Some(p) => p,
        };
        match Cert::from_file(path) {
            Err(err) => return Err(format_err!("Error parsing {} {:?}", path_str, err)),
            Ok(c) => result.push(c),
        };
    }
    Ok(result)
}

/// Fetch signature contents by building a URL for signature store
pub async fn fetch_url(http_client: &Client, base_url: &Url, sha: &str, i: u64) -> Fallible<Bytes> {
    let url = base_url
        .join(format!("{}/", sha.replace(":", "=")).as_str())?
        .join(format!("signature-{}", i).as_str())?;
    let res = http_client
        .get(url.clone())
        .send()
        .map_err(|e| format_err!(e.to_string()))
        .await?;

    let url_s = url.to_string();
    let status = res.status();
    match status.is_success() {
        true => Ok(res.bytes().await?),
        false => Err(format_err!("Error fetching {} - {}", url_s, status)),
    }
}

// This fetches keys and computes the validity of the verification.
struct Helper {
    keyring: Keyring,
}
impl VerificationHelper for Helper {
    fn get_certs(&mut self, _: &[KeyHandle]) -> Result<Vec<Cert>> {
        Ok(self.keyring.clone())
    }

    fn check(&mut self, structure: MessageStructure) -> Result<()> {
        let mut good = false;
        for (i, layer) in structure.into_iter().enumerate() {
            match (i, layer) {
                // First, we are interested in signatures over the
                // data, i.e. level 0 signatures.
                (0, MessageLayer::SignatureGroup { results }) => {
                    // Finally, given a VerificationResult, which only says
                    // whether the signature checks out mathematically, we apply
                    // our policy.
                    match results.into_iter().next() {
                        Some(Ok(_)) => good = true,
                        Some(Err(e)) => return Err(Error::from(e).into()),
                        None => return Err(format_err!("No signature")),
                    }
                }
                _ => return Err(format_err!("Unexpected message structure")),
            }
        }

        if good {
            Ok(()) // Good signature.
        } else {
            Err(format_err!("Signature verification failed"))
        }
    }
}

/// Verify that signature is valid and contains expected digest
pub async fn verify_signature(
    public_keys: &Keyring,
    body: Bytes,
    expected_digest: &str,
) -> Fallible<()> {
    let p = &P::new();
    let mut plaintext = Vec::new();
    let helper = Helper {
        keyring: public_keys.to_vec(),
    };
    let mut verifier = VerifierBuilder::from_bytes(&body)?.with_policy(p, None, helper)?;

    // Verify the message signature
    copy(&mut verifier, &mut plaintext).context("Parsing message")?;

    // Check that message has valid contents
    let signature: Signature =
        serde_json::from_slice(&plaintext).context("Deserializing message")?;
    let message_digest = signature.critical.image.digest;
    if message_digest == expected_digest {
        Ok(())
    } else {
        Err(format_err!(
            "Valid signature, but digest mismatches: {}",
            message_digest
        ))
    }
}

/// Generate URLs for signature store and attempt to find a valid signature
pub async fn verify_signatures_for_digest(
    client: &Client,
    base_url: &Url,
    public_keys: &Keyring,
    digest: &str,
) -> Fallible<()> {
    let mut errors = vec![];

    let mut attempts = Range {
        start: 1,
        end: MAX_SIGNATURES,
    };
    loop {
        if let Some(i) = attempts.next() {
            match fetch_url(client, base_url, digest, i).await {
                Ok(body) => match verify_signature(public_keys, body, digest).await {
                    Ok(_) => return Ok(()),
                    Err(e) => errors.push(e),
                },
                Err(e) => errors.push(e),
            }
        } else {
            return Err(format_err!(
                "Failed to find signatures for digest {}: {:#?}",
                digest,
                errors
            ));
        }
    }
}
