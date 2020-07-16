use crate::errors::{Result, ResultExt};
use crate::v2;
use async_stream::try_stream;
use futures::stream::Stream;
use futures::{self};
use reqwest::{Method, RequestBuilder, StatusCode};

#[derive(Debug, Default, Deserialize, Serialize)]
struct Catalog {
    pub repositories: Vec<String>,
}

impl v2::Client {
    pub fn get_catalog<'a, 'b: 'a>(
        &'b self,
        paginate: Option<u32>,
    ) -> impl Stream<Item = Result<String>> + 'a {
        let url = {
            let suffix = if let Some(n) = paginate {
                format!("?n={}", n)
            } else {
                "".to_string()
            };
            let ep = format!("{}/v2/_catalog{}", self.base_url.clone(), suffix);

            reqwest::Url::parse(&ep)
                .chain_err(|| format!("failed to parse url from string '{}'", ep))
        };

        try_stream! {
            let req = self.build_reqwest(Method::GET, url?);

            let catalog = fetch_catalog(req).await?;

            for repo in catalog.repositories {
                yield repo;
            }
        }
    }
}

async fn fetch_catalog(req: RequestBuilder) -> Result<Catalog> {
    let r = req.send().await?;
    let status = r.status();
    trace!("Got status: {:?}", status);
    match status {
        StatusCode::OK => r
            .json::<Catalog>()
            .await
            .chain_err(|| "get_catalog: failed to fetch the whole body"),
        _ => bail!("get_catalog: wrong HTTP status '{}'", status),
    }
}
