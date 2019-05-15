extern crate cincinnati;
extern crate commons;
extern crate graph_builder;
extern crate quay;
extern crate semver;
extern crate url;

use self::graph_builder::registry::{self, fetch_releases, Release};
use self::graph_builder::release::{Metadata, MetadataKind::V0};
use self::semver::Version;
use failure::Fallible;
use self::cincinnati::plugins::internal::metadata_fetch_quay::DEFAULT_QUAY_MANIFESTREF_KEY as MANIFESTREF_KEY;
use self::cincinnati::Empty;
use self::cincinnati::WouldCycle;
use self::graph_builder::graph::create_graph;
use self::graph_builder::registry::Registry;
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
    payload_shas: Option<Vec<&str>>,
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
            source: if let Some(payload_shas) = &payload_shas {
                format!("{}@{}", source_base, payload_shas[i])
            } else {
                format!("{}:0.0.{}", source_base, i + start)
            },
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
        read_credentials(credentials_path.as_ref(), &registry.host_port_string()).unwrap();
    let mut releases = fetch_releases(
        &registry,
        &repo,
        username.as_ref().map(String::as_ref),
        password.as_ref().map(String::as_ref),
        &mut cache,
        MANIFESTREF_KEY,
    )
    .expect("fetch_releases failed: ");
    assert_eq!(2, releases.len());

    remove_metadata_by_key(&mut releases, MANIFESTREF_KEY);
    assert_eq!(
        expected_releases(
            &registry,
            repo,
            2,
            0,
            None,
            Some(vec![
                "sha256:4a17dfe4b891de1edf7604bb51246924cb1caf93a46474603501812074665cd9",
                "sha256:a01f4b9f291c0a687b9ff4bf904bf5f5ce57d25c2536ec5afc92393481023313",
            ])
        ),
        releases
    )
}

#[test]
fn fetch_release_public_without_credentials_must_fail() {
    init_logger();

    let registry = Registry::try_from_str("quay.io").unwrap();
    let repo = "redhat/openshift-cincinnati-test-private-manual";
    let mut cache = HashMap::new();
    let releases = fetch_releases(&registry, &repo, None, None, &mut cache, MANIFESTREF_KEY);
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
    let releases = fetch_releases(&registry, &repo, None, None, &mut cache, MANIFESTREF_KEY)
        .expect("should not error on emtpy repo");
    assert!(releases.is_empty())
}

#[test]
fn fetch_release_public_with_first_empty_tag_must_succeed() {
    init_logger();

    let registry = Registry::try_from_str("quay.io").unwrap();
    let repo = "redhat/openshift-cincinnati-test-emptyfirsttag-public-manual";
    let mut cache = HashMap::new();
    let mut releases = fetch_releases(&registry, &repo, None, None, &mut cache, MANIFESTREF_KEY)
        .expect("fetch_releases failed: ");
    assert_eq!(2, releases.len());
    remove_metadata_by_key(&mut releases, MANIFESTREF_KEY);
    assert_eq!(
        expected_releases(
            &registry,
            repo,
            2,
            1,
            None,
            Some(vec![
                "sha256:e1f04336e1c78ae92c54a799bf6705bfe1f7edbda2f8f9c206ac1bb8f6019eb8",
                "sha256:b5e7677333bdbfd69d749cb3a7045dd5f0ef891adb91f1f675c65ef4e8515442"
            ])
        ),
        releases
    )
}

fn expected_releases_labels_test_annoated(registry: &Registry, repo: &str) -> Vec<Release> {
    let metadata: HashMap<usize, HashMap<String, String>> = [
        (0, HashMap::new()),
        (
            1,
            [
                (String::from("kind"), String::from("test")),
                (
                    String::from("io.openshift.upgrades.graph.previous.remove"),
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
                String::from("io.openshift.upgrades.graph.release.remove"),
                String::from("true"),
            )]
            .iter()
            .cloned()
            .collect(),
        ),
        (
            3,
            [(
                String::from("io.openshift.upgrades.graph.previous.add"),
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

    expected_releases(registry, repo, 4, 0, Some(metadata), None)
}

#[test]
#[ignore]
// TODO(steveeJ): fix and/or move this test to the plugin network tests
fn fetch_and_annotate_releases_with_quay_labels() {
    init_logger();

    let registry = Registry::try_from_str("quay.io").unwrap();
    let mut runtime = tokio::runtime::current_thread::Runtime::new().unwrap();
    let repo = "redhat/openshift-cincinnati-test-labels-public-manual";

    let mut cache = HashMap::new();
    let (username, password) = (None, None);
    // let (label_filter, api_token, api_base) = (DEFAULT_QUAY_LABEL_FILTER, None, DEFAULT_API_BASE);

    let mut releases = fetch_releases(
        &registry,
        &repo,
        username,
        password,
        &mut cache,
        MANIFESTREF_KEY,
    )
    .expect("fetch_releases failed: ");

    // if let Some(metadata_fetcher) = &metadata_fetcher {
    //     let populated_releases: Vec<registry::Release> = runtime
    //         .block_on(fetch_and_populate_dynamic_metadata(
    //             metadata_fetcher,
    //             releases,
    //         ))
    //         .expect("fetch and population of dynamic metadata to work");

    //     releases = populated_releases;
    // };

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
            MANIFESTREF_KEY,
        )
        .expect("fetch_releases failed: ");
        assert_eq!(2, releases.len());
        remove_metadata_by_key(&mut releases, MANIFESTREF_KEY);
        assert_eq!(
            expected_releases(
                &registry,
                repo,
                2,
                0,
                None,
                Some(vec![
                    "sha256:a264db3ac5288c9903dc3db269fca03a0b122fe4af80b57fc5087b329995013d",
                    "sha256:73df5efa869eaf57d4125f7655e05e1a72b59d05e55fea06d3701ea5b59234ff"
                ])
            ),
            releases
        );
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

#[test]
fn fetch_release_with_cyclic_metadata_fails() -> Fallible<()> {
    init_logger();

    let registry = Registry::try_from_str("quay.io").unwrap();
    let repo = "redhat/openshift-cincinnati-test-cyclic-public-manual";

    let mut cache = HashMap::new();
    let (username, password) = (None, None);

    let releases = fetch_releases(
        &registry,
        &repo,
        username,
        password,
        &mut cache,
        MANIFESTREF_KEY,
    )
    .expect("fetch_releases failed: ");

    match create_graph(releases) {
        Ok(_) => bail!("create_graph succeeded despite cyclic metadata"),
        Err(err) => {
            ensure!(
                err.downcast_ref::<WouldCycle<Empty>>().is_some(),
                "error {:#?} has wrong type",
                err,
            );
            Ok(())
        }
    }
}
