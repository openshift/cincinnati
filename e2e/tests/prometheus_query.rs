use commons::prelude_errors::*;
use prometheus_query::v1::queries::{QueryData, QueryResult, VectorResult};
use prometheus_query::v1::Client;

// #[tokio::test]
#[test]
fn query_prometheus() -> Fallible<()> {
    let api_base = std::env::var("PROM_ENDPOINT").context("PROM_ENDPOINT not set")?;
    let token = std::env::var("PROM_TOKEN").context("PROM_TOKEN not set")?;

    let client = Client::builder()
        .api_base(Some(api_base))
        .access_token(Some(token))
        .accept_invalid_certs(Some(true))
        .build()?;

    let query = r#"up{job="apiserver"}"#;

    // TODO(vrutkovs): update expected_json
    let expected_json = r#"
        {"status":"success","data":{"resultType":"vector","result":[{"metric":{"version":"4.0.0-0.3"},"value":[1552056334,"0.04081632653061224"]},{"metric":{"version":"4.0.0-0.alpha-2019-03-05-054505"},"value":[1552056334,"0.056451612903225805"]},{"metric":{"version":"4.0.0-0.6"},"value":[1552056334,"0.07692307692307693"]},{"metric":{"version":"4.0.0-0.7"},"value":[1552056334,"0.04519774011299435"]},{"metric":{"version":"4.0.0-0.5"},"value":[1552056334,"0.0625"]}]}}
        "#;

    let mut expected_result: Vec<VectorResult> =
        match serde_json::from_str::<QueryResult>(expected_json)? {
            QueryResult::Success(query_success) => match query_success.data() {
                QueryData::Vector(vector) => vector.to_vec(),
                _ => panic!("expected vector"),
            },
            _ => panic!("expected result"),
        };

    let mut result: Vec<VectorResult> = match client.query(query.to_string(), None, None)? {
        QueryResult::Success(query_success) => match query_success.data() {
            QueryData::Vector(vector) => vector.to_vec(),
            _ => panic!("expected vector"),
        },
        _ => panic!("expected result"),
    };

    fn sort_by_version(a: &VectorResult, b: &VectorResult) -> std::cmp::Ordering {
        let a = a.sample().clone();
        let b = b.sample().clone();

        if a < b {
            std::cmp::Ordering::Less
        } else if a > b {
            std::cmp::Ordering::Greater
        } else {
            std::cmp::Ordering::Equal
        }
    }

    expected_result.sort_by(sort_by_version);
    result.sort_by(sort_by_version);

    // TODO(vrutkovs): remove timestamps from expected_result/result before uncommenting assert
    //assert_eq!(expected_result, result);

    Ok(())
}
