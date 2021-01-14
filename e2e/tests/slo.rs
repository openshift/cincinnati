use prometheus_query::v1::queries::{QueryData, QueryResult, QuerySuccess, VectorResult};
use prometheus_query::v1::Client;
use std::env;
use test_case::test_case;

// Service reports at all times
#[test_case(r#"min_over_time(up{job="cincinnati-policy-engine"}[1h])"#, "1")]
#[test_case(r#"min_over_time(up{job="cincinnati-graph-builder"}[1h])"#, "1")]
// No upstream errors
#[test_case("max_over_time(cincinnati_pe_http_upstream_errors_total[1h])", "0")]
#[test_case("max_over_time(cincinnati_gb_graph_upstream_errors_total[1h])", "0")]
// Use clamp_min to bring up the minimal serve duration to 1s
// If the quantile would produce a bigger result this test would fail
#[test_case(
    "clamp_min(histogram_quantile(0.90, sum(cincinnati_pe_v1_graph_serve_duration_seconds_bucket) by (le)), 1)",
    "1"
)]
// At least one scrape has been performed
#[test_case("clamp_max(cincinnati_gb_graph_upstream_scrapes_total, 1)", "1")]
// No scrape errors
#[test_case("cincinnati_gb_graph_upstream_errors_total", "0")]
fn check_slo(query: &'static str, expected: &'static str) {
    let prometheus_api_base = match env::var("PROM_ENDPOINT") {
        Ok(env) => format!("{}/api/v1/query", env),
        _ => panic!("PROM_ENDPOINT unset"),
    };

    let prometheus_token = match env::var("PROM_TOKEN") {
        Ok(env) => env,
        _ => panic!("PROM_TOKEN unset"),
    };

    let mut runtime = commons::testing::init_runtime().unwrap();

    let prometheus_client = Client::builder()
        .api_base(Some(prometheus_api_base.clone()))
        .access_token(Some(prometheus_token))
        .accept_invalid_certs(Some(true))
        .build()
        .unwrap();

    let result: QuerySuccess = match runtime
        .block_on(prometheus_client.query(query.to_string(), None, None))
        .unwrap()
    {
        QueryResult::Success(query_success) => query_success,
        _ => panic!("expected success"),
    };
    let vector_data: &Vec<VectorResult> = match result.data() {
        QueryData::Vector(vector_data) => vector_data,
        _ => panic!("expected vector"),
    };
    assert_ne!(vector_data.len(), 0, "the vector contains 0 elements");
    let first_result: &VectorResult = vector_data.get(0).unwrap();
    assert_eq!(first_result.sample().to_string(), expected);
}
