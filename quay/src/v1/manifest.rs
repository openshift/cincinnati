//! Manifest API.

use super::Client;
use anyhow::Error;
use reqwest::Method;

/// API result with all labels.
///
/// The quay.io documentation doesn't specify the result type.
/// It was inspected manually like so:
/// ```console
/// $ curl --get https://quay.io/api/v1/repository/redhat/openshift-cincinnati-test-labels-public-manual/manifest/sha256:b35233e5d354ca2c1cdf9f71f2cdfa807d030f2b307ce7c5e88d86f20e6b65a0/labels | jq .
/// {
///   "labels": [
///     {
///       "value": "0.0.1",
///       "media_type": "text/plain",
///       "id": "03e8f6db-4669-42d8-a4ec-2d2d2785b0b7",
///       "key": "io.openshift.upgrades.graph.edge-previous-add",
///       "source_type": "api"
///     },
///     {
///       "value": "0.0.0",
///       "media_type": "text/plain",
///       "id": "b5e17080-08ce-4a98-a397-6869a0e16dbe",
///       "key": "io.openshift.upgrades.graph.edge-previous-add",
///       "source_type": "api"
///     }
///   ]
/// }
/// ```
#[derive(Debug, Deserialize)]
pub(crate) struct Labels {
    pub(crate) labels: Vec<Label>,
}

/// Tag label.
#[derive(Clone, Debug, Deserialize, PartialEq)]
pub struct Label {
    /// Label key.
    pub key: String,
    /// Label value.
    pub value: String,
    /// Label media-type.
    pub media_type: String,
    /// Label identifier.
    pub id: String,
    /// Label Source.
    pub source_type: String,
}

impl From<Label> for (String, String) {
    fn from(l: Label) -> (String, String) {
        (l.key, l.value)
    }
}

impl Client {
    /// Fetch manifestref labels
    pub async fn get_labels<S: AsRef<str>>(
        &self,
        repository: S,
        manifest_ref: S,
        filter: Option<S>,
    ) -> Result<Vec<Label>, Error> {
        let endpoint = format!(
            "repository/{}/manifest/{}/labels",
            repository.as_ref(),
            manifest_ref.as_ref()
        );

        // IMPORTANT: We intentionally do NOT pass the filter parameter to the Quay API
        // because using the filter parameter requires authentication (via @require_repo_read
        // decorator in Quay's implementation), even for public repositories. Since we access
        // public repositories without authentication, we must fetch all labels and filter
        // client-side instead.
        //
        // Background: The Quay API endpoint has @require_repo_read which enforces read
        // permissions when the filter parameter is used, but allows unauthenticated access
        // when fetching all labels without filtering. This is intentional behavior in Quay.
        // Source: https://github.com/quay/quay/blob/d13921d53123e678a716000273ab49b8f46b9d1f/endpoints/api/manifest.py#L214-L223
        //
        // Example (requires authentication - returns 403 without auth):
        //   https://quay.io/api/v1/repository/redhat/openshift-cincinnati-test-labels-public-manual/manifest/sha256:0275e5e316373faaabea9f13dfc27541e3c6e301b08bd92f443e987195faa9d6/labels?filter=io.openshift.upgrades.graph
        // Example (public read - works without auth):
        //   https://quay.io/api/v1/repository/redhat/openshift-cincinnati-test-labels-public-manual/manifest/sha256:0275e5e316373faaabea9f13dfc27541e3c6e301b08bd92f443e987195faa9d6/labels
        // See: https://github.com/openshift/cincinnati/pull/1057
        let req = self.new_request(Method::GET, &endpoint)?;

        let resp = req.send().await?;

        // Check if the response was successful
        if !resp.status().is_success() {
            let status = resp.status();
            return Err(anyhow::anyhow!("Request failed with status {}", status));
        }

        let json = resp.json::<Labels>().await?;

        // Apply client-side filtering if a filter prefix was provided
        let labels = if let Some(filter) = filter {
            let filter_str = filter.as_ref();
            json.labels
                .into_iter()
                .filter(|label| label.key.starts_with(filter_str))
                .collect()
        } else {
            json.labels
        };

        Ok(labels)
    }
}
