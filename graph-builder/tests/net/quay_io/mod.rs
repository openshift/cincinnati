extern crate cincinnati;
extern crate commons;
extern crate graph_builder;
extern crate quay;
extern crate semver;
extern crate url;

use self::graph_builder::registry::{self, fetch_releases, Release};
use self::graph_builder::release::{Metadata, MetadataKind::V0};
use self::semver::Version;
use net::quay_io::graph_builder::metadata::{
    fetch_and_populate_dynamic_metadata, MetadataFetcher, QuayMetadataFetcher,
    DEFAULT_QUAY_LABEL_FILTER, MANIFESTREF_KEY,
};
use net::quay_io::graph_builder::registry::Registry;
use net::quay_io::quay::v1::DEFAULT_API_BASE;
use std::collections::HashMap;

#[cfg(feature = "test-net-private")]
use self::graph_builder::registry::read_credentials;

fn init_logger() {
    let _ = env_logger::try_init_from_env(env_logger::Env::default());
}

fn expected_releases(
    registry: &Registry,
    repo: &str,
    count: usize,
    start: usize,
    metadata: Option<HashMap<usize, HashMap<String, String>>>,
) -> Vec<Release> {
    use std::collections::HashMap;

    let source_base = &format!("{}/{}", registry.host_port_string(), repo);

    let mut releases = Vec::new();
    let mut metadata: HashMap<usize, HashMap<String, String>> = metadata.unwrap_or_else(|| {
        [
            (0, HashMap::new()),
            (
                1,
                [(String::from("kind"), String::from("test"))]
                    .iter()
                    .cloned()
                    .collect(),
            ),
        ]
        .iter()
        .cloned()
        .collect()
    });
    for i in 0..count {
        releases.push(Release {
            source: format!("{}:0.0.{}", source_base, i + start).to_string(),
            metadata: Metadata {
                kind: V0,
                version: Version {
                    major: 0,
                    minor: 0,
                    patch: i as u64,
                    pre: vec![],
                    build: vec![],
                },
                next: vec![],
                previous: if i > 0 {
                    vec![Version {
                        major: 0,
                        minor: 0,
                        patch: (i - 1) as u64,
                        pre: vec![],
                        build: vec![],
                    }]
                } else {
                    vec![]
                },

                metadata: metadata.remove(&i).unwrap_or_default(),
            },
        });
    }
    releases
}

fn remove_metadata_by_key(releases: &mut Vec<Release>, key: &str) {
    for ref mut release in releases.iter_mut() {
        release.metadata.metadata.remove(key).unwrap();
    }
}

#[cfg(feature = "test-net-private")]
#[test]
fn fetch_release_private_with_credentials_must_succeed() {
    use std::path::PathBuf;

    init_logger();

    let registry = Registry::try_from_str("quay.io").unwrap();
    let repo = "redhat/openshift-cincinnati-test-private-manual";
    let mut cache = HashMap::new();
    let credentials_path = match std::env::var("CINCINNATI_TEST_CREDENTIALS_PATH") {
        Ok(value) => Some(PathBuf::from(value)),
        Err(_) => {
            panic!("CINCINNATI_TEST_CREDENTIALS_PATH unset, skipping...");
        }
    };
    let (username, password) =
        read_credentials(credentials_path.as_ref(), registry.host().unwrap()).unwrap();
    let mut releases = fetch_releases(
        &registry,
        &repo,
        username.as_ref().map(String::as_ref),
        password.as_ref().map(String::as_ref),
        &mut cache,
    )
    .expect("fetch_releases failed: ");
    assert_eq!(2, releases.len());

    let releases = remove_metadata_by_key(&mut releases, MANIFESTREF_KEY);

    assert_eq!(expected_releases(&registry, repo, 2, 0, None), releases)
}

#[test]
fn fetch_release_public_without_credentials_must_fail() {
    init_logger();

    let registry = Registry::try_from_str("quay.io").unwrap();
    let repo = "redhat/openshift-cincinnati-test-private-manual";
    let mut cache = HashMap::new();
    let releases = fetch_releases(&registry, &repo, None, None, &mut cache);
    assert_eq!(true, releases.is_err());
    assert_eq!(
        true,
        releases
            .err()
            .unwrap()
            .to_string()
            .contains("401 Unauthorized")
    );
}

#[test]
fn fetch_release_public_with_no_release_metadata_must_not_error() {
    init_logger();

    let registry = Registry::try_from_str("quay.io").unwrap();
    let repo = "redhat/openshift-cincinnati-test-nojson-public-manual";
    let mut cache = HashMap::new();
    let releases = fetch_releases(&registry, &repo, None, None, &mut cache)
        .expect("should not error on emtpy repo");
    assert!(releases.is_empty())
}

#[test]
fn fetch_release_public_with_first_empty_tag_must_succeed() {
    init_logger();

    let registry = Registry::try_from_str("quay.io").unwrap();
    let repo = "redhat/openshift-cincinnati-test-emptyfirsttag-public-manual";
    let mut cache = HashMap::new();
    let mut releases =
        fetch_releases(&registry, &repo, None, None, &mut cache).expect("fetch_releases failed: ");
    assert_eq!(2, releases.len());
    remove_metadata_by_key(&mut releases, MANIFESTREF_KEY);
    assert_eq!(expected_releases(&registry, repo, 2, 1, None), releases)
}

fn expected_releases_labels_test_annoated(registry: &Registry, repo: &str) -> Vec<Release> {
    let metadata: HashMap<usize, HashMap<String, String>> = [
        (0, HashMap::new()),
        (
            1,
            [
                (String::from("kind"), String::from("test")),
                (
                    String::from("com.openshift.upgrades.graph.previous.remove"),
                    String::from("0.0.0"),
                ),
            ]
            .iter()
            .cloned()
            .collect(),
        ),
        (
            2,
            [(
                String::from("com.openshift.upgrades.graph.release.remove"),
                String::from("true"),
            )]
            .iter()
            .cloned()
            .collect(),
        ),
        (
            3,
            [(
                String::from("com.openshift.upgrades.graph.previous.add"),
                String::from("0.0.1,0.0.0"),
            )]
            .iter()
            .cloned()
            .collect(),
        ),
    ]
    .iter()
    .cloned()
    .collect();

    expected_releases(registry, repo, 4, 0, Some(metadata))
}

#[test]
fn fetch_release_annotates_releases_with_quay_labels() {
    init_logger();

    let registry = Registry::try_from_str("quay.io").unwrap();
    let mut runtime = tokio::runtime::current_thread::Runtime::new().unwrap();
    let repo = "redhat/openshift-cincinnati-test-labels-public-manual";

    let mut cache = HashMap::new();
    let (username, password) = (None, None);
    let (label_filter, api_token, api_base) = (DEFAULT_QUAY_LABEL_FILTER, None, DEFAULT_API_BASE);

    let metadata_fetcher: Option<MetadataFetcher> = Some(
        QuayMetadataFetcher::try_new(
            label_filter.to_string(),
            api_token,
            api_base.to_string(),
            repo.to_string(),
        )
        .expect("to try_new to yield a metadata fetcher"),
    );

    let mut releases = fetch_releases(&registry, &repo, username, password, &mut cache)
        .expect("fetch_releases failed: ");

    if let Some(metadata_fetcher) = &metadata_fetcher {
        let populated_releases: Vec<registry::Release> = runtime
            .block_on(fetch_and_populate_dynamic_metadata(
                metadata_fetcher,
                releases,
            ))
            .expect("fetch and population of dynamic metadata to work");

        releases = populated_releases;
    };

    assert_eq!(4, releases.len());
    assert_eq!(
        releases,
        expected_releases_labels_test_annoated(&registry, &repo)
    );
}

#[test]
fn fetch_release_public_must_succeed_with_schemes_missing_http_https() {
    init_logger();

    let test = |registry: Registry| {
        let repo = "redhat/openshift-cincinnati-test-public-manual";
        let mut cache = HashMap::new();
        let (username, password) = (None, None);
        let mut releases = fetch_releases(
            &registry,
            &repo,
            username.as_ref().map(String::as_ref),
            password.as_ref().map(String::as_ref),
            &mut cache,
        )
        .expect("fetch_releases failed: ");
        assert_eq!(2, releases.len());
        remove_metadata_by_key(&mut releases, MANIFESTREF_KEY);
        assert_eq!(expected_releases(&registry, repo, 2, 0, None), releases);
    };

    [
        "quay.io",
        // TODO: enable this when the dkregistry-rs migration to reqwest is complete
        //"http://quay.io",
        "https://quay.io",
    ]
    .iter()
    .map(|url| registry::Registry::try_from_str(url).unwrap())
    .for_each(test);
}
