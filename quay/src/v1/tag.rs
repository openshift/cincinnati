//! Tag API.

use super::Client;
use anyhow::Result as Fallible;
use async_stream::stream;
use futures::Stream;
use reqwest::Method;

/// API result with paginated repository tags.
#[derive(Debug, Deserialize)]
pub(crate) struct PaginatedTags {
    #[allow(dead_code)]
    /// Pagination flag.
    pub(crate) has_additional: bool,
    #[allow(dead_code)]
    /// Pagination index.
    pub(crate) page: u32,
    /// List of tags in current page.
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
    pub async fn stream_tags<'a, 'b: 'a, S>(
        &'b self,
        repository: S,
        only_active_tags: bool,
    ) -> impl Stream<Item = Fallible<Tag>> + 'a
    where
        S: AsRef<str>,
    {
        // TODO(lucab): implement pagination, filtering, and other advanced options.
        let endpoint = format!("repository/{}/tag", repository.as_ref());
        let actives_only = format!("{}", only_active_tags);

        stream! {
            let req = self
                .new_request(Method::GET, endpoint)?
                .query(&[("onlyActiveTags", actives_only)]);

            let resp = req.send().await?;

            // Check if the response was successful
            if !resp.status().is_success() {
                let status = resp.status();
                yield Err(anyhow::anyhow!("Request failed with status {}", status));
                return;
            }

            let paginated_tags = resp.json::<PaginatedTags>().await?.tags;
            for tag in paginated_tags {
                yield Ok(tag);
            }
        }
    }
}
