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

#[test_case("stable-4.7", "amd64", "application/json")]
#[test_case("stable-4.7", "amd64", "*/*")]
#[test_case("stable-4.7", "s390x", "application/json")]
#[test_case("stable-4.7", "s390x", "application/*")]
#[test_case("stable-4.8", "amd64", "application/json")]
#[test_case("stable-4.8", "s390x", "application/vnd.redhat.cincinnati.v1+json")]
#[test_case("stable-4.9", "amd64", "application/json")]
#[test_case("stable-4.9", "amd64", "application/vnd.redhat.cincinnati.v1+json")]
#[test_case("stable-4.9", "s390x", "application/json")]
#[test_case("candidate-4.12", "multi", "application/json")]
fn e2e_channel_success(channel: &'static str, arch: &'static str, header: &'static str) {
    let version = "v1";
    let testdata_path = format!(
        "{}/{}_{}_{}_{}.json",
        *TESTDATA_DIR, *METADATA_REVISION, channel, arch, version
    );
    let testdata = &std::fs::read_to_string(&testdata_path)
        .context(format!("reading {}", &testdata_path))
        .unwrap();
    let runtime = commons::testing::init_runtime().unwrap();

    let expected: cincinnati::plugins::internal::versioned_graph::VersionedGraph =
        serde_json::from_str(testdata).unwrap();

    let res = run_graph_query(channel, arch, header, &runtime);

    assert!(res.status().is_success(), "{}", res.status());
    let text = runtime.block_on(res.text()).unwrap();
    let actual: cincinnati::plugins::internal::versioned_graph::VersionedGraph =
        serde_json::from_str(&text)
            .context(format!("Failed to parse '{}' as json", text))
            .unwrap();

    if let Err(e) = cincinnati::testing::compare_versioned_graphs_verbose(
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

#[test_case("stable-4.9", "amd64", "application/vnd.redhat.cincinnati.v1+json")]
fn e2e_cors_headers(channel: &'static str, arch: &'static str, header: &'static str) {
    let runtime = commons::testing::init_runtime().unwrap();
    let res = run_graph_query(channel, arch, header, &runtime);
    let origin_value = res.headers().get("access-control-allow-origin").unwrap();
    assert_eq!(
        origin_value
            .to_str()
            .unwrap_or("error unwrapping origin_value header value"),
        ORIGIN_HEADER_VALUE.to_str().unwrap()
    );
}

// #ignore attribute as we dont need these tests working on cincinnati repo by default,
// its already being tested when we check for correctness of the graph output in `e2e_channel_success`
// this test is mainly for `cincinnati-graph-data` to check if the graph output is valid.
/// test the graph response for a correctly formatted graph
#[ignore]
#[test_case("stable-4.4", "amd64", "application/json")]
#[test_case("stable-4.5", "amd64", "*/*")]
#[test_case("stable-4.6", "s390x", "application/json")]
#[test_case("stable-4.7", "s390x", "application/*")]
#[test_case("stable-4.8", "amd64", "application/json")]
#[test_case("stable-4.9", "s390x", "application/vnd.redhat.cincinnati.v1+json")]
#[test_case("candidate-4.12", "multi", "application/json")]
fn e2e_graph_format(channel: &'static str, arch: &'static str, header: &'static str) {
    let runtime = commons::testing::init_runtime().unwrap();

    let res = run_graph_query(channel, arch, header, &runtime);

    assert!(res.status().is_success(), "{}", res.status());
    let text = runtime.block_on(res.text()).unwrap();
    let _actual: cincinnati::plugins::internal::versioned_graph::VersionedGraph =
        serde_json::from_str(&text)
            .context(format!("Failed to parse '{}' as json", text))
            .unwrap();
}

/// runs the graph query and returns the response
fn run_graph_query(
    channel: &'static str,
    arch: &'static str,
    header: &'static str,
    runtime: &Runtime,
) -> Response {
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
                .header(ACCEPT, HeaderValue::from_static(header))
                .header(ORIGIN, ORIGIN_HEADER_VALUE.clone())
                .send(),
        )
        .context("Failed to execute Prometheus query")
        .unwrap()
}
