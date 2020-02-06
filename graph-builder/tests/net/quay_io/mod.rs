extern crate cincinnati;
extern crate commons;
extern crate graph_builder;
extern crate quay;
extern crate semver;
extern crate url;

use self::cincinnati::plugins::internal::metadata_fetch_quay::DEFAULT_QUAY_MANIFESTREF_KEY as MANIFESTREF_KEY;
use self::cincinnati::testing::{TestGraphBuilder, TestMetadata};
use self::cincinnati::Empty;
use self::cincinnati::WouldCycle;
use self::graph_builder::graph::create_graph;
use self::graph_builder::registry::Registry;
use self::graph_builder::registry::{self, fetch_releases, Release};
use self::graph_builder::release::{Metadata, MetadataKind::V0};
use self::semver::Version;
use failure::{Fallible, ResultExt};
use itertools::Itertools;
use std::collections::HashMap;

#[cfg(feature = "test-net-private")]
use self::graph_builder::registry::read_credentials;

lazy_static::lazy_static! {
    static ref FETCH_CONCURRENCY: usize = {
        let app_settings = graph_builder::config::AppSettings::default();
        app_settings.fetch_concurrency
    };

}

fn init_logger() {
    let _ = env_logger::try_init_from_env(env_logger::Env::default());
}

fn common_init() -> (
    tokio::runtime::Runtime,
    graph_builder::registry::cache::Cache,
) {
    init_logger();
    (
        tokio::runtime::Runtime::new().unwrap(),
        graph_builder::registry::cache::new(),
    )
}

fn expected_releases(
    registry: &Registry,
    repo: &str,
    count: usize,
    start: usize,
    metadata: Option<HashMap<usize, HashMap<String, String>>>,
    payload_shas: Option<Vec<&str>>,
) -> Vec<Release> {
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

macro_rules! assert_permutation_results {
    ($release_count:expr, $expected_releases:expr, $releases:expr) => {
        assert!(
            $expected_releases
                .clone()
                .into_iter()
                .permutations($release_count)
                .any(|expected_releases| { expected_releases == $releases }),
            "mismatch: \n{:?} didn't match any iteration of \n{:?}",
            $releases,
            $expected_releases,
        );
    };
}

fn remove_metadata_by_key(releases: &mut Vec<Release>, key: &str) {
    for release in releases.iter_mut() {
        release.metadata.metadata.remove(key);
    }
}

fn replace_sha_by_version_in_source(releases: &mut Vec<Release>) {
    for release in releases.iter_mut() {
        let version = release.metadata.version.to_string();
        let source_front = release.source.split("@").nth(0).unwrap();
        release.source = format!("{}:{}", source_front, version);
    }
}

#[cfg(feature = "test-net-private")]
#[test]
fn fetch_release_private_with_credentials_must_succeed() {
    use std::path::PathBuf;

    let (mut runtime, mut cache) = common_init();

    let registry = Registry::try_from_str("quay.io").unwrap();
    let repo = "redhat/openshift-cincinnati-test-private-manual";
    let credentials_path = match std::env::var("CINCINNATI_TEST_CREDENTIALS_PATH") {
        Ok(value) => Some(PathBuf::from(value)),
        _ => {
            panic!("CINCINNATI_TEST_CREDENTIALS_PATH unset, skipping...");
        }
    };
    let (username, password) =
        read_credentials(credentials_path.as_ref(), &registry.host_port_string()).unwrap();
    let mut releases = runtime
        .block_on(fetch_releases(
            &registry,
            &repo,
            username.as_ref().map(String::as_ref),
            password.as_ref().map(String::as_ref),
            &mut cache,
            MANIFESTREF_KEY,
            *FETCH_CONCURRENCY,
        ))
        .expect("fetch_releases failed: ");
    assert_eq!(2, releases.len());

    remove_metadata_by_key(&mut releases, MANIFESTREF_KEY);
    remove_metadata_by_key(
        &mut releases,
        &format!(
            "{}.{}",
            cincinnati::plugins::internal::arch_filter::DEFAULT_KEY_FILTER,
            cincinnati::plugins::internal::arch_filter::DEFAULT_ARCH_KEY
        ),
    );

    let release_count = 2;
    let expected_releases = expected_releases(
        &registry,
        repo,
        release_count,
        0,
        None,
        Some(vec![
            "sha256:4a17dfe4b891de1edf7604bb51246924cb1caf93a46474603501812074665cd9",
            "sha256:a01f4b9f291c0a687b9ff4bf904bf5f5ce57d25c2536ec5afc92393481023313",
        ]),
    );

    assert_permutation_results!(release_count, expected_releases, releases);
}

#[test]
fn fetch_release_private_without_credentials_must_fail() {
    let (mut runtime, mut cache) = common_init();

    let registry = Registry::try_from_str("quay.io").unwrap();
    let repo = "redhat/openshift-cincinnati-test-private-manual";
    let releases = runtime.block_on(fetch_releases(
        &registry,
        &repo,
        None,
        None,
        &mut cache,
        MANIFESTREF_KEY,
        *FETCH_CONCURRENCY,
    ));
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
    let (mut runtime, mut cache) = common_init();

    let registry = Registry::try_from_str("quay.io").unwrap();
    let repo = "redhat/openshift-cincinnati-test-nojson-public-manual";
    let releases = runtime
        .block_on(fetch_releases(
            &registry,
            &repo,
            None,
            None,
            &mut cache,
            MANIFESTREF_KEY,
            *FETCH_CONCURRENCY,
        ))
        .expect("should not error on emtpy repo");
    assert!(releases.is_empty())
}

#[test]
fn fetch_release_public_with_first_empty_tag_must_succeed() {
    let (mut runtime, mut cache) = common_init();

    let registry = Registry::try_from_str("quay.io").unwrap();
    let repo = "redhat/openshift-cincinnati-test-emptyfirsttag-public-manual";
    let mut releases = runtime
        .block_on(fetch_releases(
            &registry,
            &repo,
            None,
            None,
            &mut cache,
            MANIFESTREF_KEY,
            *FETCH_CONCURRENCY,
        ))
        .expect("fetch_releases failed: ");
    assert_eq!(2, releases.len());
    remove_metadata_by_key(&mut releases, MANIFESTREF_KEY);
    remove_metadata_by_key(
        &mut releases,
        &format!(
            "{}.{}",
            cincinnati::plugins::internal::arch_filter::DEFAULT_KEY_FILTER,
            cincinnati::plugins::internal::arch_filter::DEFAULT_ARCH_KEY
        ),
    );

    let release_count = 2;
    let expected_releases = expected_releases(
        &registry,
        repo,
        release_count,
        1,
        None,
        Some(vec![
            "sha256:e1f04336e1c78ae92c54a799bf6705bfe1f7edbda2f8f9c206ac1bb8f6019eb8",
            "sha256:b5e7677333bdbfd69d749cb3a7045dd5f0ef891adb91f1f675c65ef4e8515442",
        ]),
    );
    assert_permutation_results!(release_count, expected_releases, releases);
}

#[test]
fn fetch_release_public_must_succeed_with_schemes_missing_http_https() {
    let (mut runtime, mut cache) = common_init();

    let test = |registry: Registry| {
        let repo = "redhat/openshift-cincinnati-test-public-manual";
        let (username, password) = (None, None);
        let mut releases = runtime
            .block_on(fetch_releases(
                &registry,
                &repo,
                username.as_ref().map(String::as_ref),
                password.as_ref().map(String::as_ref),
                &mut cache,
                MANIFESTREF_KEY,
                *FETCH_CONCURRENCY,
            ))
            .expect("fetch_releases failed: ");
        assert_eq!(2, releases.len());
        remove_metadata_by_key(&mut releases, MANIFESTREF_KEY);
        remove_metadata_by_key(
            &mut releases,
            &format!(
                "{}.{}",
                cincinnati::plugins::internal::arch_filter::DEFAULT_KEY_FILTER,
                cincinnati::plugins::internal::arch_filter::DEFAULT_ARCH_KEY
            ),
        );

        let release_count = 2;
        let expected_releases = expected_releases(
            &registry,
            repo,
            release_count,
            0,
            None,
            Some(vec![
                "sha256:a264db3ac5288c9903dc3db269fca03a0b122fe4af80b57fc5087b329995013d",
                "sha256:73df5efa869eaf57d4125f7655e05e1a72b59d05e55fea06d3701ea5b59234ff",
            ]),
        );

        assert_permutation_results!(release_count, expected_releases, releases);
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
    let (mut runtime, mut cache) = common_init();

    let registry = Registry::try_from_str("quay.io").unwrap();
    let repo = "redhat/openshift-cincinnati-test-cyclic-public-manual";

    let (username, password) = (None, None);

    let releases = runtime
        .block_on(fetch_releases(
            &registry,
            &repo,
            username,
            password,
            &mut cache,
            MANIFESTREF_KEY,
            *FETCH_CONCURRENCY,
        ))
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

#[test]
fn fetch_releases_public_multiarch_manual_succeeds() -> Fallible<()> {
    let (mut runtime, mut cache) = common_init();

    let registry = registry::Registry::try_from_str("https://quay.io")?;
    let repo = "redhat/openshift-cincinnati-test-public-multiarch-manual";
    let (username, password) = (None, None);
    let releases = runtime
        .block_on(fetch_releases(
            &registry,
            &repo,
            username.as_ref().map(String::as_ref),
            password.as_ref().map(String::as_ref),
            &mut cache,
            MANIFESTREF_KEY,
            *FETCH_CONCURRENCY,
        ))
        .expect("fetch_releases failed: ");

    assert_eq!(7, releases.len());

    Ok(())
}

#[test]
fn create_graph_public_multiarch_manual_succeeds() -> Fallible<()> {
    let (mut runtime, mut cache) = common_init();

    let registry = registry::Registry::try_from_str("https://quay.io")?;
    let repo = "redhat/openshift-cincinnati-test-public-multiarch-manual";
    let (username, password) = (None, None);

    let releases = {
        let mut fetched_releases = runtime
            .block_on(fetch_releases(
                &registry,
                &repo,
                username.as_ref().map(String::as_ref),
                password.as_ref().map(String::as_ref),
                &mut cache,
                MANIFESTREF_KEY,
                *FETCH_CONCURRENCY,
            ))
            .context("fetch_releases failed: ")?;

        replace_sha_by_version_in_source(&mut fetched_releases);

        // remove unwanted metadata
        [
            MANIFESTREF_KEY,
            "io.openshift.upgrades.graph.release.channels",
            "io.openshift.upgrades.graph.previous.add",
            "io.openshift.upgrades.graph.previous.remove",
            "io.openshift.upgrades.graph.release.arch",
        ]
        .iter()
        .for_each(|key| remove_metadata_by_key(&mut fetched_releases, key));

        fetched_releases
    };

    let graph = create_graph(releases).expect("create_graph failed");

    let expected_graph: cincinnati::Graph = {
        let input_edges = Some(vec![(0, 1), (1, 2), (2, 3), (3, 4), (5, 6)]);
        let input_metadata: TestMetadata = vec![
            (
                0,
                [(String::from("version_suffix"), String::from("+amd64"))]
                    .iter()
                    .cloned()
                    .collect(),
            ),
            (
                1,
                [(String::from("version_suffix"), String::from("+amd64"))]
                    .iter()
                    .cloned()
                    .collect(),
            ),
            (
                2,
                [(String::from("version_suffix"), String::from("+amd64"))]
                    .iter()
                    .cloned()
                    .collect(),
            ),
            (
                3,
                [(String::from("version_suffix"), String::from("+amd64"))]
                    .iter()
                    .cloned()
                    .collect(),
            ),
            (
                4,
                [(String::from("version_suffix"), String::from("+amd64"))]
                    .iter()
                    .cloned()
                    .collect(),
            ),
            (
                2,
                [(String::from("version_suffix"), String::from("+arm64"))]
                    .iter()
                    .cloned()
                    .collect(),
            ),
            (
                3,
                [(String::from("version_suffix"), String::from("+arm64"))]
                    .iter()
                    .cloned()
                    .collect(),
            ),
        ];

        TestGraphBuilder::new()
            .with_image(&format!("quay.io/{}", repo))
            .with_metadata(input_metadata.clone())
            .with_edges(input_edges.clone())
            .with_version_template("0.0.{{i}}")
            .enable_payload_suffix(true)
            .build()
    };

    assert_eq!(expected_graph, graph);

    Ok(())
}
