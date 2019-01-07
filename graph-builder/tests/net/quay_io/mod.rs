extern crate cincinnati;
extern crate graph_builder;
extern crate semver;

use self::graph_builder::registry::{fetch_releases, read_credentials, Release};
use self::graph_builder::release::Metadata;
use self::graph_builder::release::MetadataKind::V0;
use self::semver::Version;

fn init_logger() {
    let _ = env_logger::try_init_from_env(env_logger::Env::default());
}

fn expected_releases(source_base: &str, count: usize, start: usize) -> Vec<Release> {
    use std::collections::HashMap;

    let mut releases = Vec::new();

    let mut metadata: HashMap<usize, HashMap<String, String>> = [
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
    .collect();
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

                metadata: metadata.remove(&i).unwrap(),
            },
        });
    }
    releases
}

#[cfg(feature = "test-net-private")]
#[test]
fn fetch_release_private_with_credentials_must_succeed() {
    use std::path::PathBuf;

    init_logger();

    let registry = "quay.io";
    let repo = "redhat/openshift-cincinnati-test-private-manual";
    let credentials_path = match std::env::var("CINCINNATI_TEST_CREDENTIALS_PATH") {
        Ok(value) => Some(PathBuf::from(value)),
        Err(_) => {
            panic!("CINCINNATI_TEST_CREDENTIALS_PATH unset, skipping...");
        }
    };
    let (username, password) = read_credentials(credentials_path.as_ref(), registry).unwrap();
    let releases = fetch_releases(
        &registry,
        &repo,
        username.as_ref().map(String::as_ref),
        password.as_ref().map(String::as_ref),
    )
    .expect("fetch_releases failed: ");
    assert_eq!(2, releases.len());
    assert_eq!(
        expected_releases(&format!("{}/{}", registry, repo), 2, 0),
        releases
    )
}

#[test]
fn fetch_release_public_without_credentials_must_fail() {
    init_logger();

    let registry = "quay.io";
    let repo = "redhat/openshift-cincinnati-test-private-manual";
    let releases = fetch_releases(&registry, &repo, None, None);
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

    let registry = "quay.io";
    let repo = "redhat/openshift-cincinnati-test-nojson-public-manual";
    let releases =
        fetch_releases(&registry, &repo, None, None).expect("should not error on emtpy repo");
    assert!(releases.is_empty())
}

#[test]
fn fetch_release_public_with_first_empty_tag_must_succeed() {
    init_logger();

    let registry = "quay.io";
    let repo = "redhat/openshift-cincinnati-test-emptyfirsttag-public-manual";
    let releases = fetch_releases(&registry, &repo, None, None).expect("fetch_releases failed: ");
    assert_eq!(2, releases.len());
    assert_eq!(
        expected_releases(&format!("{}/{}", registry, repo), 2, 1),
        releases
    )
}
