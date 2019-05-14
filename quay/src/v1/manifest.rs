//! Manifest API.

use super::Client;
use failure::Error;
use futures::future;
use futures::prelude::*;
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

impl Into<(String, String)> for Label {
    fn into(self) -> (String, String) {
        (self.key, self.value)
    }
}

impl Client {
    /// Fetch manifestref labels
    pub fn get_labels<S: AsRef<str>>(
        &self,
        repository: S,
        manifest_ref: S,
        filter: Option<S>,
    ) -> impl Future<Item = Vec<Label>, Error = Error> {
        let endpoint = format!(
            "repository/{}/manifest/{}/labels",
            repository.as_ref(),
            manifest_ref.as_ref()
        );
        let req = self.new_request(Method::GET, &endpoint);
        future::result(req)
            .map(|req| {
                if let Some(filter) = filter {
                    req.query(&[("filter", filter.as_ref())])
                } else {
                    req
                }
            })
            .and_then(|req| req.send().from_err())
            .and_then(|resp| resp.error_for_status().map_err(Error::from))
            .and_then(|mut resp| resp.json::<Labels>().from_err())
            .map(|json| json.labels)
    }
}
