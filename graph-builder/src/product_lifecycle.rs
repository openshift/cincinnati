//! Product lifecycle API fetcher.
//!
//! This module fetches product lifecycle information from the Red Hat product API
//! and makes it available for inclusion in the graph-data tarball.

use crate::graph::State;
use commons::prelude_errors::*;
use log::{error, info};
use prometheus::{Counter, Gauge, Opts};
use std::io::Write;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use tokio::time::sleep;

lazy_static::lazy_static! {
    static ref PRODUCT_LIFECYCLE_FETCHES: Counter = Counter::with_opts(
        Opts::new(
            "product_lifecycle_fetches_total",
            "Total number of product lifecycle API fetch attempts"
        )
    )
    .unwrap();

    static ref PRODUCT_LIFECYCLE_ERRORS: Counter = Counter::with_opts(
        Opts::new(
            "product_lifecycle_errors_total",
            "Total number of product lifecycle API fetch errors"
        )
    )
    .unwrap();

    static ref PRODUCT_LIFECYCLE_NOT_MODIFIED: Counter = Counter::with_opts(
        Opts::new(
            "product_lifecycle_not_modified_total",
            "Total number of 304 Not Modified responses from product lifecycle API"
        )
    )
    .unwrap();

    static ref PRODUCT_LIFECYCLE_LAST_FETCH: Gauge = Gauge::with_opts(
        Opts::new(
            "product_lifecycle_last_successful_fetch_timestamp",
            "UTC timestamp of last successful product lifecycle fetch"
        )
    )
    .unwrap();
}

/// Register product lifecycle metrics
pub fn register_metrics(registry: &prometheus::Registry) -> Fallible<()> {
    registry.register(Box::new(PRODUCT_LIFECYCLE_FETCHES.clone()))?;
    registry.register(Box::new(PRODUCT_LIFECYCLE_ERRORS.clone()))?;
    registry.register(Box::new(PRODUCT_LIFECYCLE_NOT_MODIFIED.clone()))?;
    registry.register(Box::new(PRODUCT_LIFECYCLE_LAST_FETCH.clone()))?;
    Ok(())
}

/// Fetch product data from the API
async fn fetch_products(
    client: &reqwest::Client,
    api_url: &str,
    last_etag: Option<&str>,
) -> Fallible<(Option<String>, Option<String>)> {
    let mut request = client.get(api_url).header("Accept", "application/json");

    if let Some(etag) = last_etag {
        request = request.header("If-None-Match", etag);
    }

    let response = request.send().await?;
    let status = response.status();
    let new_etag = response
        .headers()
        .get("etag")
        .and_then(|v| v.to_str().ok())
        .map(String::from);

    if status == reqwest::StatusCode::NOT_MODIFIED {
        // 304 Not Modified - data hasn't changed
        PRODUCT_LIFECYCLE_NOT_MODIFIED.inc();
        info!("Product lifecycle data not modified (304)");
        return Ok((None, new_etag));
    }

    if !status.is_success() {
        bail!("Product lifecycle API returned status {}", status);
    }

    let body = response.text().await?;
    Ok((Some(body), new_etag))
}

/// Save product JSON to a temporary file
fn save_to_file(json: &str) -> Fallible<PathBuf> {
    let temp_dir = std::env::temp_dir();
    let final_path = temp_dir.join("products.json");

    let mut temp_file =
        tempfile::NamedTempFile::new_in(&temp_dir).context("Failed to create temporary file")?;

    temp_file
        .write_all(json.as_bytes())
        .context("Failed to write to temporary file")?;
    temp_file
        .as_file_mut()
        .sync_all()
        .context("Failed to sync temporary file")?;

    temp_file
        .persist(&final_path)
        .context("Failed to persist temporary file")?;

    Ok(final_path)
}

/// Main polling loop for product lifecycle data
pub async fn run(api_url: String, poll_interval: Duration, timeout: Duration, state: Arc<State>) {
    info!(
        "Starting product lifecycle fetcher: api_url={}, poll_interval={}s, timeout={}s",
        api_url,
        poll_interval.as_secs(),
        timeout.as_secs()
    );

    // Build HTTP client
    let client = reqwest::ClientBuilder::new()
        .gzip(true)
        .timeout(timeout)
        .build()
        .expect("Failed to build HTTP client for product lifecycle");

    let mut first_iteration = true;
    let mut last_etag: Option<String> = None;

    loop {
        if first_iteration {
            first_iteration = false;
        } else {
            sleep(poll_interval).await;
        }

        PRODUCT_LIFECYCLE_FETCHES.inc();

        // Fetch from API
        match fetch_products(&client, &api_url, last_etag.as_deref()).await {
            Ok((Some(json), new_etag)) => {
                // New data received
                info!(
                    "Fetched product lifecycle data ({} bytes), etag={:?}",
                    json.len(),
                    new_etag
                );

                // Validate it's valid JSON
                if let Err(e) = serde_json::from_str::<serde_json::Value>(&json) {
                    error!("Product lifecycle API returned invalid JSON: {}", e);
                    PRODUCT_LIFECYCLE_ERRORS.inc();
                    continue;
                }

                // Save to file
                match save_to_file(&json) {
                    Ok(path) => {
                        last_etag = new_etag;
                        *state.product_data_path.write() = Some(path.clone());

                        PRODUCT_LIFECYCLE_LAST_FETCH.set(
                            std::time::SystemTime::now()
                                .duration_since(std::time::UNIX_EPOCH)
                                .unwrap_or_default()
                                .as_secs() as f64,
                        );
                        info!("Saved product lifecycle data to {:?}", path);
                    }
                    Err(e) => {
                        error!("Failed to save product lifecycle data to file: {}", e);
                        PRODUCT_LIFECYCLE_ERRORS.inc();
                    }
                }
            }
            Ok((None, new_etag)) => {
                // 304 Not Modified - update ETag but keep existing data
                if let Some(etag) = new_etag {
                    last_etag = Some(etag);
                }
            }
            Err(e) => {
                error!("Failed to fetch product lifecycle data: {}", e);
                PRODUCT_LIFECYCLE_ERRORS.inc();
                // Continue loop - will retry on next interval
            }
        }
    }
}
