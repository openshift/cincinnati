use crate::errors::{Error, Result};
use crate::v2::*;
use reqwest;
use reqwest::{Method, StatusCode};

impl Client {
    /// Check if a blob exists.
    pub async fn has_blob(&self, name: &str, digest: &str) -> Result<bool> {
        let url = {
            let ep = format!("{}/v2/{}/blobs/{}", self.base_url, name, digest);
            match reqwest::Url::parse(&ep) {
                Ok(url) => url,
                Err(e) => {
                    return Err(Error::from(format!(
                        "failed to parse url from string: {}",
                        e
                    )));
                }
            }
        };

        let res = self.build_reqwest(Method::HEAD, url.clone()).send().await?;

        trace!("Blob HEAD status: {:?}", res.status());

        match res.status() {
            StatusCode::OK => Ok(true),
            _ => Ok(false),
        }
    }

    /// Retrieve blob.
    pub async fn get_blob(&self, name: &str, digest: &str) -> Result<Vec<u8>> {
        let digest = ContentDigest::try_new(digest.to_string())?;

        let blob = {
            let ep = format!("{}/v2/{}/blobs/{}", self.base_url, name, digest);
            let url = reqwest::Url::parse(&ep)
                .map_err(|e| Error::from(format!("failed to parse url from string: {}", e)))?;

            let res = self.build_reqwest(Method::GET, url.clone()).send().await?;

            trace!("GET {} status: {}", res.url(), res.status());
            let status = res.status();

            if !(status.is_success()
                // Let client errors through to populate them with the body
                || status.is_client_error())
            {
                return Err(Error::from(format!(
                    "GET request failed with status '{}'",
                    status
                )));
            }

            let status = res.status();
            let body_vec = res.bytes().await?.to_vec();
            let len = body_vec.len();

            if status.is_success() {
                trace!("Successfully received blob with {} bytes ", len);
                Ok(body_vec)
            } else if status.is_client_error() {
                Err(Error::from(format!(
                    "GET request failed with status '{}' and body of size {}: {:#?}",
                    status,
                    len,
                    String::from_utf8_lossy(&body_vec)
                )))
            } else {
                // We only want to handle success and client errors here
                error!(
                    "Received unexpected HTTP status '{}' after fetching the body. Please submit a bug report.",
                    status
                );
                Err(Error::from(format!(
                    "GET request failed with status '{}'",
                    status
                )))
            }
        }?;

        digest.try_verify(&blob)?;
        Ok(blob.to_vec())
    }
}
