use futures::StreamExt;
use tokio::runtime::Runtime;

#[cfg(feature = "test-net-private")]
mod private;

fn common_init() -> Runtime {
    let _ = env_logger::try_init_from_env(env_logger::Env::default());
    Runtime::new().unwrap()
}

#[test]
fn test_public_stream_active_tags() {
    let rt = common_init();
    let repo = "redhat/openshift-cincinnati-test-public-manual";
    let expected = vec!["0.0.1", "0.0.0"];

    let client = quay::v1::Client::builder().build().unwrap();
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
fn test_public_get_labels() {
    let rt = common_init();
    let repo = "redhat/openshift-cincinnati-test-labels-public-manual";
    let tag_name = "0.0.1";

    let client = quay::v1::Client::builder().build().unwrap();
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
    let fetch_labels = client.get_labels(
        repo.to_string(),
        digest,
        Some("io.openshift.upgrades.graph".to_string()),
    );
    let labels = rt.block_on(fetch_labels).unwrap();
    assert_eq!(
        labels
            .into_iter()
            .map(Into::into)
            .collect::<Vec<(String, String)>>(),
        vec![(
            "io.openshift.upgrades.graph.previous.remove".to_string(),
            "0.0.0".to_string()
        )]
    );
}
