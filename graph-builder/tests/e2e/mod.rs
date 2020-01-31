#[cfg(feature = "test-e2e")]
use assert_json_diff::assert_json_include;
use reqwest::header::{HeaderValue, ACCEPT};
use serde_json::Value;
use std::env;
use test_case::test_case;
use url::Url;

#[test_case("a",    "amd64", include_str!("./testdata/a-amd64.json");    "channel a amd64")]
#[test_case("b",    "amd64", include_str!("./testdata/b-amd64.json");    "channel b amd64")]
#[test_case("test", "amd64", include_str!("./testdata/test-amd64.json"); "channel test amd64")]
fn e2e_channel_success(channel: &'static str, arch: &'static str, testdata: &str) {
    let graph_base_url = match env::var("GRAPH_URL") {
        Ok(env) => env,
        _ => panic!("GRAPH_URL unset"),
    };

    let expected: Value = serde_json::from_str(testdata).unwrap();

    let mut graph_url = Url::parse(&graph_base_url).unwrap();
    graph_url
        .query_pairs_mut()
        .append_pair("channel", channel)
        .append_pair("arch", arch);

    println!("Querying {}", graph_url.to_string());

    let mut res = reqwest::ClientBuilder::new()
        .danger_accept_invalid_certs(true)
        .build()
        .unwrap()
        .get(&graph_url.to_string())
        .header(ACCEPT, HeaderValue::from_static("application/json"))
        .send()
        .unwrap();
    assert_eq!(res.status().is_success(), true);
    let actual = serde_json::from_str(&res.text().unwrap()).unwrap();
    assert_json_include!(actual: actual, expected: expected)
}
