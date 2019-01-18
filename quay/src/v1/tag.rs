//! Manifest API.

use super::Client;
use failure::Error;
use futures::prelude::*;
use futures::{future, stream};
use reqwest::Method;

/// API result with paginated repository tags.
#[derive(Debug, Deserialize)]
pub(crate) struct PaginatedTags {
    /// Pagination flag.
    pub(crate) has_additional: bool,
    /// Pagination index.
    pub(crate) page: u32,
    /// Set of tags in current page.
    pub(crate) tags: Vec<Tag>,
}

/// Repository tag.
#[derive(Clone, Debug, Deserialize, PartialEq)]
pub struct Tag {
    /// Manifest digest, in `type:digest` format.
    pub manifest_digest: Option<String>,
    /// Tag name.
    pub name: String,
    /// Whether this tag version is an history revert.
    pub reversion: bool,
}

impl Client {
    /// Fetch tags in a repository, in a streaming way.
    pub fn stream_tags<S: AsRef<str>>(
        &self,
        repository: S,
        only_active_tags: bool,
    ) -> impl Stream<Item = Tag, Error = Error> {
        // TODO(lucab): implement pagination, filtering, and other advanced options.
        let endpoint = format!("repository/{}/tag", repository.as_ref());
        let actives_only = format!("{}", only_active_tags);

        let req = self.new_request(Method::GET, endpoint);
        future::result(req)
            .map(|req| req.query(&[("onlyActiveTags", actives_only)]))
            .and_then(|req| req.send().from_err())
            .and_then(|resp| resp.error_for_status().map_err(Error::from))
            .and_then(|mut resp| resp.json::<PaginatedTags>().from_err())
            .map(|page| stream::iter_ok(page.tags))
            .flatten_stream()
    }
}
