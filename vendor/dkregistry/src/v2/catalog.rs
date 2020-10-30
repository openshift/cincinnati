use crate::errors::Result;
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

            reqwest::Url::parse(&ep).map_err(|err| crate::Error::from(err))
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
            .await.map_err(Into::into),
        _ => Err(crate::Error::UnexpectedHttpStatus(status)),
    }
}
