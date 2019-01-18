//! Manifest API.

use super::Client;
use failure::Error;
use futures::future;
use futures::prelude::*;
use reqwest::Method;

/// API result with all labels.
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

impl Client {
    pub fn get_labels<S: AsRef<str>>(
        &self,
        repository: S,
        manifest_ref: S,
    ) -> impl Future<Item = Vec<Label>, Error = Error> {
        let endpoint = format!(
            "repository/{}/manifest/{}/labels",
            repository.as_ref(),
            manifest_ref.as_ref()
        );
        let req = self.new_request(Method::GET, &endpoint);
        future::result(req)
            .and_then(|req| req.send().from_err())
            .and_then(|resp| resp.error_for_status().map_err(Error::from))
            .and_then(|mut resp| resp.json::<Labels>().from_err())
            .map(|json| json.labels)
    }
}
