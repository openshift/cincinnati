use anyhow::Context;
use hamcrest2::prelude::*;
use prometheus_query::v1::queries::{QueryData, QueryResult, QuerySuccess, VectorResult};
use prometheus_query::v1::Client;
use serde_json::{Map, Value};
use std::env;
use test_case::test_case;

fn get_query_result_string(query: &'static str) -> VectorResult {
    let prometheus_api_base = match env::var("PROM_ENDPOINT") {
        Ok(env) => format!("{}/api/v1/query", env),
        _ => panic!("PROM_ENDPOINT unset"),
    };

    let prometheus_token = match env::var("PROM_TOKEN") {
        Ok(env) => env,
        _ => panic!("PROM_TOKEN unset"),
    };

    let prometheus_client = Client::builder()
        .api_base(Some(prometheus_api_base))
        .access_token(Some(prometheus_token))
        .accept_invalid_certs(Some(true))
        .build()
        .context("Failed to establish Prometheus connection")
        .unwrap();

    let result: QuerySuccess = match prometheus_client
        .query(query.to_string(), None, None)
        .context("Error running query")
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
    return vector_data.last().unwrap().clone();
}

#[test_case("sum(kube_pod_container_status_restarts_total{container=~'(graph-builder|policy-engine)'}) or vector(0)" => "0"; "No GB/PE restarts")]
#[test_case("sum(kube_pod_container_status_last_terminated_reason{container=~'(graph-builder|policy-engine)', reason='Completed'}) or vector(0)" => "0"; "Crashes due to liveness checks")]
#[test_case("sum(kube_pod_container_status_last_terminated_reason{container=~'(graph-builder|policy-engine)', reason='OOMKilled'})  or vector(0)" => "0"; "Crashes due to OOM killer")]
#[test_case("sum(kube_pod_container_status_last_terminated_reason{container=~'(graph-builder|policy-engine)', reason='Error'}) or vector(0)" => "0"; "Crashes due to process exit code")]
#[test_case("sum(cincinnati_gb_graph_upstream_errors_total) or vector(0)" => "0"; "No scrape errors")]
#[test_case("sum(cincinnati_pe_http_upstream_errors_total) or vector(0)" => "0"; "No upstream errors")]
fn check_slo_exact(query: &'static str) -> String {
    get_query_result_string(query).sample().to_string()
}

#[test_case("sum_over_time(cincinnati_gb_graph_upstream_scrapes_total[1h])" => is greater_than_or_equal_to(1); "At least one scrape has been performed")]
#[test_case("sum(cincinnati_pe_graph_response_errors_total{code!~'4.+'}) or vector(0)" => is less_than(1); "Only HTTP 4xx errors returned")]
fn check_slo_numeric(query: &'static str) -> i32 {
    get_query_result_string(query)
        .sample()
        .to_string()
        .parse::<i32>()
        .unwrap()
}

// 90% of requests are served in less than 0.5 seconds
#[test_case(
    "max_over_time(histogram_quantile(0.90, sum(cincinnati_pe_graph_serve_duration_seconds_bucket) by (le))[1h:1m])"
     => is less_than(0.5); "90% of requests are served in 0.5 sec"
)]
#[test_case("min_over_time(sum(rate(cincinnati_pe_graph_incoming_requests_total[1m]))[1h:10m]) > 20" => is greater_than(100.0); "OSUS can handle at least 100rps")]
fn check_slo_float(query: &'static str) -> f32 {
    get_query_result_string(query)
        .sample()
        .to_string()
        .parse::<f32>()
        .unwrap()
}

// Graph builder reports valid git commit
#[test_case("cincinnati_gb_build_info", "git_commit" => is not(eq("unknown")); "graph-builder returns build information")]
// Policy engine reports valid git commit
#[test_case("cincinnati_pe_build_info", "git_commit" => is not(eq("unknown")); "policy-engine returns build information")]
fn check_slo_parameter(query: &'static str, parameter: &'static str) -> String {
    let result = get_query_result_string(query);
    let metric: &Map<String, Value> = match result.metric() {
        Value::Object(v) => v,
        _ => panic!("Non-object value received"),
    };
    let param_value: &Value = match metric.get(parameter) {
        None => panic!("{} not found in {:#?}", parameter, metric),
        Some(v) => v,
    };
    match param_value {
        Value::String(v) => v.to_string(),
        _ => panic!("Expected {} to be a string", param_value.to_string()),
    }
}
