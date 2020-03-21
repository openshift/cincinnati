use crate as cincinnati;

use cincinnati::plugins::internal::graph_builder::commons::tests::{
    common_init, remove_metadata_by_key,
};
use cincinnati::plugins::internal::graph_builder::release::{
    create_graph, Metadata, MetadataKind::V0, Release,
};
use cincinnati::plugins::internal::graph_builder::release_scrape_dockerv2::registry::{
    self, fetch_releases, Registry,
};
use cincinnati::plugins::internal::metadata_fetch_quay::DEFAULT_QUAY_MANIFESTREF_KEY as MANIFESTREF_KEY;
use cincinnati::{Empty, MapImpl, WouldCycle};
use failure::{bail, ensure, Fallible};
use itertools::Itertools;
use semver::Version;
use std::collections::HashMap;

#[cfg(feature = "test-net-private")]
use cincinnati::plugins::internal::graph_builder::release_scrape_dockerv2::registry::read_credentials;

lazy_static::lazy_static! {

    static ref FETCH_CONCURRENCY: usize = {
        cincinnati::plugins::internal::graph_builder::release_scrape_dockerv2::DEFAULT_FETCH_CONCURRENCY
    };

}

fn expected_releases(
    registry: &Registry,
    repo: &str,
    count: usize,
    start: usize,
    metadata: Option<HashMap<usize, MapImpl<String, String>>>,
    payload_shas: Option<Vec<&str>>,
) -> Vec<Release> {
    let source_base = &format!("{}/{}", registry.host_port_string(), repo);

    let mut releases = Vec::new();
    let mut metadata: HashMap<usize, MapImpl<String, String>> = metadata.unwrap_or_else(|| {
        [
            (0, MapImpl::new()),
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

#[cfg(feature = "test-net-private")]
#[test]
fn fetch_release_private_with_credentials_must_succeed() {
    use std::path::PathBuf;

    let (mut runtime, cache) = common_init();

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
            cache,
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
    let (mut runtime, cache) = common_init();

    let registry = Registry::try_from_str("quay.io").unwrap();
    let repo = "redhat/openshift-cincinnati-test-private-manual";
    let releases = runtime.block_on(fetch_releases(
        &registry,
        &repo,
        None,
        None,
        cache,
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
    let (mut runtime, cache) = common_init();

    let registry = Registry::try_from_str("quay.io").unwrap();
    let repo = "redhat/openshift-cincinnati-test-nojson-public-manual";
    let releases = runtime
        .block_on(fetch_releases(
            &registry,
            &repo,
            None,
            None,
            cache,
            MANIFESTREF_KEY,
            *FETCH_CONCURRENCY,
        ))
        .expect("should not error on emtpy repo");
    assert!(releases.is_empty())
}

#[test]
fn fetch_release_public_with_first_empty_tag_must_succeed() {
    let (mut runtime, cache) = common_init();

    let registry = Registry::try_from_str("quay.io").unwrap();
    let repo = "redhat/openshift-cincinnati-test-emptyfirsttag-public-manual";
    let mut releases = runtime
        .block_on(fetch_releases(
            &registry,
            &repo,
            None,
            None,
            cache,
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
    let (mut runtime, cache) = common_init();

    let test = |registry: Registry| {
        let repo = "redhat/openshift-cincinnati-test-public-manual";
        let (username, password) = (None, None);
        let mut releases = runtime
            .block_on(fetch_releases(
                &registry,
                &repo,
                username.as_ref().map(String::as_ref),
                password.as_ref().map(String::as_ref),
                cache.clone(),
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
    let (mut runtime, cache) = common_init();

    let registry = Registry::try_from_str("quay.io").unwrap();
    let repo = "redhat/openshift-cincinnati-test-cyclic-public-manual";

    let (username, password) = (None, None);

    let releases = runtime
        .block_on(fetch_releases(
            &registry,
            &repo,
            username,
            password,
            cache,
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
    let (mut runtime, cache) = common_init();

    let registry = registry::Registry::try_from_str("https://quay.io")?;
    let repo = "redhat/openshift-cincinnati-test-public-multiarch-manual";
    let (username, password) = (None, None);
    let releases = runtime
        .block_on(fetch_releases(
            &registry,
            &repo,
            username.as_ref().map(String::as_ref),
            password.as_ref().map(String::as_ref),
            cache,
            MANIFESTREF_KEY,
            *FETCH_CONCURRENCY,
        ))
        .expect("fetch_releases failed: ");

    assert_eq!(7, releases.len());

    Ok(())
}
