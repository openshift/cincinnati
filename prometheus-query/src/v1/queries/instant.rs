//! Impelement instant queries

use super::*;
use anyhow::Result as Fallible;
use reqwest;
use std::time::Duration;

pub static INSTANT_QUERY_PATH_SUFFIX: &str = "/api/v1/query";

impl Client {
    /// Sends the given query to the remote API, given an optional `time` and timeout.SystemTime
    ///
    /// The `time` is measured since the UNIX_EPOCH
    pub fn query(
        &self,
        query: String,
        time: Option<chrono::DateTime<chrono::Utc>>,
        timeout: Option<Duration>,
    ) -> Fallible<QueryResult> {
        self.new_request(reqwest::Method::GET, INSTANT_QUERY_PATH_SUFFIX)
            .and_then(move |request_builder| {
                let mut query = vec![("query", query)];

                if let Some(time) = time {
                    query.push(("time", time.to_rfc3339()));
                }

                if let Some(timeout) = timeout {
                    query.push(("timeout", format!("{}s", timeout.as_secs())));
                };

                trace!("sending query '{:?}'", &query);
                request_builder.query(&query).send().map_err(Into::into)
            })
            .and_then(|response| response.error_for_status().map_err(Into::into))
            .and_then(|response| response.json().map_err(Into::into))
    }
}
