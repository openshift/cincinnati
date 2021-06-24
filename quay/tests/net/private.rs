use anyhow::Error;
use futures::StreamExt;
use tokio::runtime::Runtime;

fn common_init() -> (Runtime, Option<String>) {
    let _ = env_logger::try_init_from_env(env_logger::Env::default());
    let runtime = Runtime::new().unwrap();
    let token = std::env::var("CINCINNATI_TEST_QUAY_API_TOKEN")
        .expect("CINCINNATI_TEST_QUAY_API_TOKEN missing");
    (runtime, Some(token))
}

#[test]
#[ignore = "currently broken on CI"]
fn test_wrong_auth() {
    let (rt, _) = common_init();
    let repo = "redhat/openshift-cincinnati-test-private-manual";

    let client = quay::v1::Client::builder()
        .access_token(Some("CLEARLY_WRONG".to_string()))
        .build()
        .unwrap();
    let fetch_tags = async {
        client
            .stream_tags(repo, true)
            .await
            .map(Result::unwrap_err)
            .collect::<Vec<Error>>()
            .await
    };
    rt.block_on(fetch_tags);
}

#[test]
#[ignore = "currently broken on CI"]
fn test_stream_active_tags() {
    let (rt, token) = common_init();
    let repo = "redhat/openshift-cincinnati-test-private-manual";
    let expected = vec!["0.0.1", "0.0.0"];

    let client = quay::v1::Client::builder()
        .access_token(token)
        .build()
        .unwrap();
    let fetch_tags = async {
        client
            .stream_tags(repo, true)
            .await
            .map(Result::unwrap)
            .collect::<Vec<quay::v1::Tag>>()
            .await
    };
    let tags = rt.block_on(fetch_tags);
    let tag_names: Vec<String> = tags.into_iter().map(|tag| tag.name).collect();
    assert_eq!(tag_names, expected);
}

#[test]
#[ignore = "currently broken on CI"]
fn test_get_labels() {
    let (rt, token) = common_init();
    let repo = "redhat/openshift-cincinnati-test-private-manual";
    let tag_name = "0.0.0";

    let client = quay::v1::Client::builder()
        .access_token(token)
        .build()
        .unwrap();
    let fetch_tags = async {
        client
            .stream_tags(repo, true)
            .await
            .map(Result::unwrap)
            .collect::<Vec<quay::v1::Tag>>()
            .await
    };
    let tags = rt.block_on(fetch_tags);
    let filtered_tags: Vec<quay::v1::Tag> = tags
        .into_iter()
        .filter(|tag| tag.name == tag_name)
        .collect();
    assert_eq!(filtered_tags.len(), 1);

    let tag = &filtered_tags[0];
    assert_eq!(tag.name, tag_name);

    let digest = tag.manifest_digest.clone().unwrap();
    let fetch_labels = client.get_labels(repo.to_string(), digest, None);
    let labels = rt.block_on(fetch_labels).unwrap();
    assert_eq!(labels, vec![]);
}
