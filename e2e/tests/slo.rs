use failure::{format_err, Fallible};
use reqwest::header::{HeaderValue, AUTHORIZATION};
use reqwest::Response;
use serde::Deserialize;
use std::env;
use test_case::test_case;
use tokio::runtime::Runtime;
use url::Url;

#[derive(Deserialize)]
struct PromMetric {
    value: (f64, String),
}

#[derive(Deserialize)]
struct PromData {
    result: Vec<PromMetric>,
}

#[derive(Deserialize)]
struct PromResponse {
    status: String,
    data: PromData,
}

async fn prom_http_request(url: String, token: String) -> Fallible<Response> {
    let header_value = format!("Bearer {}", token);
    let authorization_header = HeaderValue::from_str(&header_value).unwrap();
    reqwest::ClientBuilder::new()
        .danger_accept_invalid_certs(true)
        .build()
        .unwrap()
        .get(&url)
        .header(AUTHORIZATION, authorization_header)
        .send()
        .await
        .map_err(Into::into)
}

fn query_prom(runtime: &mut Runtime, url: String, token: String) -> Fallible<String> {
    let res = runtime.block_on(prom_http_request(url, token)).unwrap();
    let text = runtime.block_on(res.text()).unwrap();
    println!("Prom response: {}", text);

    let r: PromResponse = match serde_json::from_str(&text) {
        Ok(r) => r,
        Err(e) => panic!("Failed to serialize json: {}", e),
    };
    match r.status.as_str() {
        "success" => match r.data.result.len() {
            1 => Ok(r.data.result[0].value.1.clone()),
            n => Err(format_err!("unexpected number of results: {}", n)),
        },
        status => Err(format_err!("incorrect query status: {}", status)),
    }
}

// Service reports at all times
#[test_case(r#"min_over_time(up{job="cincinnati-policy-engine"}[1h])"#, "1")]
#[test_case(r#"min_over_time(up{job="cincinnati-graph-builder"}[1h])"#, "1")]
// No upstream errors
#[test_case("max_over_time(cincinnati_pe_http_upstream_errors_total[1h])", "0")]
#[test_case("max_over_time(cincinnati_gb_graph_upstream_errors_total[1h])", "0")]
// Use clamp_min to bring up the minimal serve duration to 0.1 seconds
// If the quantile would produce a bigger result this test would fail
#[test_case(
    "clamp_min(histogram_quantile(0.90, sum(cincinnati_pe_v1_graph_serve_duration_seconds_bucket) by (le)), 0.1)",
    "0.1"
)]
// At least one scrape has been performed
#[test_case("clamp_max(cincinnati_gb_graph_upstream_scrapes_total, 1)", "1")]
// No scrape errors
#[test_case("cincinnati_gb_graph_upstream_errors_total", "0")]
fn check_slo(query: &'static str, expected: &'static str) {
    let prom_url = match env::var("PROM_ENDPOINT") {
        Ok(env) => format!("https://{}/api/v1/query", env),
        _ => panic!("PROM_ENDPOINT unset"),
    };

    let prom_token = match env::var("PROM_TOKEN") {
        Ok(env) => env,
        _ => panic!("PROM_TOKEN unset"),
    };

    let mut runtime = commons::testing::init_runtime().unwrap();

    let mut query_url = Url::parse(&prom_url).unwrap();
    query_url.query_pairs_mut().append_pair("query", query);

    println!("Querying {}", query_url.to_string());
    let actual = query_prom(&mut runtime, query_url.to_string(), prom_token).unwrap();
    pretty_assertions::assert_eq!(actual, expected)
}
