use commons::prelude_errors::*;
use reqwest::header::{HeaderValue, ACCEPT, ORIGIN};
use reqwest::Response;
use std::env;
use test_case::test_case;
use tokio::runtime::Runtime;
use url::Url;

lazy_static::lazy_static! {
    static ref METADATA_REVISION: String = std::env::var("E2E_METADATA_REVISION").unwrap();
    static ref TESTDATA_DIR: String = std::env::var("E2E_TESTDATA_DIR").unwrap();
    static ref ORIGIN_HEADER_VALUE: HeaderValue = HeaderValue::from_str("example.com").unwrap();
}

#[test_case("stable-4.2", "amd64")]
#[test_case("stable-4.2", "s390x")]
#[test_case("stable-4.3", "amd64")]
#[test_case("stable-4.3", "s390x")]
fn e2e_channel_success(channel: &'static str, arch: &'static str) {
    let testdata_path = format!(
        "{}/{}_{}_{}.json",
        *TESTDATA_DIR, *METADATA_REVISION, channel, arch,
    );
    let testdata = &std::fs::read_to_string(&testdata_path)
        .context(format!("reading {}", &testdata_path))
        .unwrap();
    let runtime = commons::testing::init_runtime().unwrap();

    let expected: cincinnati::Graph = serde_json::from_str(testdata).unwrap();

    let res = run_graph_query(channel, arch, &runtime);

    assert_eq!(res.status().is_success(), true, "{}", res.status());
    let text = runtime.block_on(res.text()).unwrap();
    let actual: cincinnati::Graph = serde_json::from_str(&text)
        .context(format!("Failed to parse '{}' as json", text))
        .unwrap();

    if let Err(e) = cincinnati::testing::compare_graphs_verbose(
        expected,
        actual,
        cincinnati::testing::CompareGraphsVerboseSettings {
            unwanted_metadata_keys: &[
                "io.openshift.upgrades.graph.previous.remove_regex",
                "io.openshift.upgrades.graph.previous.remove",
            ],

            ..Default::default()
        },
    ) {
        panic!("{}", e);
    }
}

#[test_case("stable-4.3", "amd64")]
fn e2e_cors_headers(channel: &'static str, arch: &'static str) {
    let runtime = commons::testing::init_runtime().unwrap();
    let res = run_graph_query(channel, arch, &runtime);
    let origin_value = res.headers().get("access-control-allow-origin").unwrap();
    assert_eq!(
        origin_value
            .to_str()
            .unwrap_or("error unwrapping origin_value header value"),
        ORIGIN_HEADER_VALUE.to_str().unwrap()
    );
}

/// runs the graph query and returns the response
fn run_graph_query(channel: &'static str, arch: &'static str, runtime: &Runtime) -> Response {
    let graph_base_url = match env::var("GRAPH_URL") {
        Ok(env) => env,
        _ => panic!("GRAPH_URL unset"),
    };

    let mut graph_url = Url::parse(&graph_base_url).unwrap();
    graph_url
        .query_pairs_mut()
        .append_pair("channel", channel)
        .append_pair("arch", arch);

    println!("Querying {}", graph_url.to_string());

    runtime
        .block_on(
            reqwest::ClientBuilder::new()
                .danger_accept_invalid_certs(true)
                .build()
                .unwrap()
                .get(&graph_url.to_string())
                .header(ACCEPT, HeaderValue::from_static("application/json"))
                .header(ORIGIN, ORIGIN_HEADER_VALUE.clone())
                .send(),
        )
        .context("Failed to execute Prometheus query")
        .unwrap()
}
