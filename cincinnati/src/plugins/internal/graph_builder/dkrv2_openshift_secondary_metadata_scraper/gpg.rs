/// Tiny crate to verify message signature and format
use self::cincinnati::plugins::prelude::*;
use crate as cincinnati;
use bytes::buf::BufExt;
use bytes::Bytes;
use futures::TryFutureExt;
use reqwest::Client;
use serde::Deserialize;
use serde_json;
use std::fs::{read_dir, File};
use std::ops::Range;
use std::path::PathBuf;
use url::Url;

use pgp::composed::message::Message;
use pgp::composed::signed_key::SignedPublicKey;
use pgp::Deserializable;

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
pub type Keyring = Vec<SignedPublicKey>;

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
        let file = File::open(path).context(format!("Reading {}", path_str))?;
        let (pubkey, _) =
            SignedPublicKey::from_armor_single(file).context(format!("Parsing {}", path_str))?;
        match pubkey.verify() {
            Err(err) => return Err(format_err!("{:?}", err)),
            Ok(_) => result.push(pubkey),
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

/// Verify that signature is valid and contains expected digest
pub async fn verify_signature(
    public_keys: &Keyring,
    body: Bytes,
    expected_digest: &str,
) -> Fallible<()> {
    let msg = Message::from_bytes(body.reader()).context("Parsing message")?;

    // Verify signature using provided public keys
    if !public_keys.iter().any(|ref k| msg.verify(k).is_ok()) {
        return Err(format_err!("No matching key found to decrypt {:#?}", msg));
    }

    // Deserialize the message
    let contents = match msg.get_content().context("Reading contents")? {
        None => return Err(format_err!("Empty message received")),
        Some(m) => m,
    };
    let signature: Signature =
        serde_json::from_slice(&contents).context("Deserializing message")?;
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
