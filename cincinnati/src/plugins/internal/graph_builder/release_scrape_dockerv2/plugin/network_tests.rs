use super::*;

const DEFAULT_SCRAPE_REGISTRY: &str = "registry.ci.openshift.org";

use cincinnati::plugins::internal::graph_builder::commons::tests::common_init;
use cincinnati::testing::{TestGraphBuilder, TestMetadata};
use commons::prelude_errors::*;

fn get_env_credentials_path(var: &str) -> PathBuf {
    match std::env::var(var) {
        Ok(value) => PathBuf::from(value),
        _ => {
            panic!("{} unset, skipping...", var);
        }
    }
}

#[cfg(feature = "test-net-private")]
#[test]
fn scrape_private_with_credentials_must_succeed() -> Fallible<()> {
    let (mut runtime, _) = common_init();

    let registry = DEFAULT_SCRAPE_REGISTRY;
    let repo = "cincinnati-ci/cincinnati-test-private-manual";

    let plugin = Box::new(ReleaseScrapeDockerv2Plugin::try_new(
        // settings
        toml::from_str::<ReleaseScrapeDockerv2Settings>(&format!(
            r#"
                    registry = "{}"
                    repository = "{}"
                    manifestref_key = "{}"
                    fetch_concurrency = {}
                    credentials_path = {:?}
                "#,
            &registry,
            &repo,
            DEFAULT_MANIFESTREF_KEY,
            DEFAULT_FETCH_CONCURRENCY,
            get_env_credentials_path("CINCINNATI_TEST_CREDENTIALS_PATH"),
        ))?,
        // cache
        None,
        // prometheus registry
        None,
    )?);

    let graph: cincinnati::Graph = runtime
        .block_on(plugin.run_internal(InternalIO {
            graph: Default::default(),
            parameters: Default::default(),
        }))?
        .graph;

    let expected_graph = {
        let input_edges = Some(vec![(0, 1)]);
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
                [
                    (String::from("version_suffix"), String::from("+amd64")),
                    (String::from("kind"), String::from("test")),
                ]
                .iter()
                .cloned()
                .collect(),
            ),
        ];
        TestGraphBuilder::new()
            .with_image(&format!("{}/{}", registry, repo))
            .with_metadata(input_metadata)
            .with_edges(input_edges)
            .with_version_template("0.0.{{i}}")
            .enable_payload_suffix(true)
            .build()
    };

    crate::testing::compare_graphs_verbose(
        expected_graph,
        graph,
        cincinnati::testing::CompareGraphsVerboseSettings {
            unwanted_metadata_keys: &[
                DEFAULT_MANIFESTREF_KEY,
                "io.openshift.upgrades.graph.release.channels",
                "io.openshift.upgrades.graph.previous.add",
                "io.openshift.upgrades.graph.previous.remove",
                "io.openshift.upgrades.graph.release.arch",
            ],

            payload_replace_sha_by_tag_left: false,
            payload_replace_sha_by_tag_right: true,

            ..Default::default()
        },
    )
}

#[test]
fn scrape_private_without_credentials_must_fail() -> Fallible<()> {
    let (mut runtime, _) = common_init();

    let registry = DEFAULT_SCRAPE_REGISTRY;
    let repo = "cincinnati-ci/cincinnati-test-private-manual";

    let plugin = Box::new(ReleaseScrapeDockerv2Plugin::try_new(
        // settings
        toml::from_str::<ReleaseScrapeDockerv2Settings>(&format!(
            r#"
                    registry = "{}"
                    repository = "{}"
                    manifestref_key = "{}"
                    fetch_concurrency = {}
                    anonymous_auth = true
                "#,
            &registry, &repo, DEFAULT_MANIFESTREF_KEY, DEFAULT_FETCH_CONCURRENCY,
        ))?,
        // cache
        None,
        // prometheus registry
        None,
    )?);

    runtime
        .block_on(plugin.run_internal(InternalIO {
            graph: Default::default(),
            parameters: Default::default(),
        }))
        .unwrap_err();

    Ok(())
}

#[test]
#[ignore = "broken on OCP 4.x registry"]
fn scrape_public_with_no_release_metadata_must_not_error() -> Fallible<()> {
    let (mut runtime, _) = common_init();

    let registry = DEFAULT_SCRAPE_REGISTRY;
    let repo = "cincinnati-ci-public/cincinnati-test-nojson-public-manual";

    let plugin = Box::new(ReleaseScrapeDockerv2Plugin::try_new(
        // settings
        toml::from_str::<ReleaseScrapeDockerv2Settings>(&format!(
            r#"
                    registry = "{}"
                    repository = "{}"
                    manifestref_key = "{}"
                    fetch_concurrency = {}
                    anonymous_auth = true
                "#,
            &registry, &repo, DEFAULT_MANIFESTREF_KEY, DEFAULT_FETCH_CONCURRENCY,
        ))?,
        // cache
        None,
        // prometheus registry
        None,
    )?);

    let graph = runtime
        .block_on(plugin.run_internal(InternalIO {
            graph: Default::default(),
            parameters: Default::default(),
        }))
        .context("should not error on emtpy repo")?
        .graph;

    crate::testing::compare_graphs_verbose(
        Default::default(),
        graph,
        cincinnati::testing::CompareGraphsVerboseSettings::default(),
    )
}

#[test]
fn scrape_public_with_first_empty_tag_must_succeed() -> Fallible<()> {
    let (mut runtime, _) = common_init();

    let registry = DEFAULT_SCRAPE_REGISTRY;
    let repo = "cincinnati-ci-public/cincinnati-test-emptyfirsttag-public-manual";

    let plugin = Box::new(ReleaseScrapeDockerv2Plugin::try_new(
        // settings
        toml::from_str::<ReleaseScrapeDockerv2Settings>(&format!(
            r#"
                    registry = "{}"
                    repository = "{}"
                    manifestref_key = "{}"
                    fetch_concurrency = {}
                    anonymous_auth = true
                "#,
            &registry, &repo, DEFAULT_MANIFESTREF_KEY, DEFAULT_FETCH_CONCURRENCY,
        ))?,
        // cache
        None,
        // prometheus registry
        None,
    )?);

    let graph = runtime
        .block_on(plugin.run_internal(InternalIO {
            graph: Default::default(),
            parameters: Default::default(),
        }))?
        .graph;

    let expected_graph = {
        let input_edges = Some(vec![(0, 1)]);
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
                [
                    (String::from("version_suffix"), String::from("+amd64")),
                    (String::from("kind"), String::from("test")),
                ]
                .iter()
                .cloned()
                .collect(),
            ),
        ];
        TestGraphBuilder::new()
            .with_image(&format!("{}/{}", registry, repo))
            .with_metadata(input_metadata)
            .with_edges(input_edges)
            .with_version_template("0.0.{{i}}")
            .enable_payload_suffix(true)
            .build()
    };

    crate::testing::compare_graphs_verbose(
        expected_graph,
        graph,
        cincinnati::testing::CompareGraphsVerboseSettings {
            unwanted_metadata_keys: &[
                DEFAULT_MANIFESTREF_KEY,
                "io.openshift.upgrades.graph.release.channels",
                "io.openshift.upgrades.graph.previous.add",
                "io.openshift.upgrades.graph.previous.remove",
                "io.openshift.upgrades.graph.release.arch",
            ],

            payload_replace_sha_by_tag_left: false,
            payload_replace_sha_by_tag_right: true,

            ..Default::default()
        },
    )
}

#[test_case::test_case(DEFAULT_SCRAPE_REGISTRY)]
#[test_case(&format!("{}:443", DEFAULT_SCRAPE_REGISTRY))]
#[ignore = "broken on OCP 4.x registry"]
// TODO: enable this when the dkregistry-rs migration to reqwest is complete
// #[test_case(&format!("http://{}", DEFAULT_SCRAPE_REGISTRY))]
fn scrape_public_must_succeed_with_various_registry_urls(registry: &str) {
    let (mut runtime, _) = common_init();

    let repo = "cincinnati-ci-public/cincinnati-test-public-manual";

    let plugin = Box::new(
        ReleaseScrapeDockerv2Plugin::try_new(
            // settings
            toml::from_str::<ReleaseScrapeDockerv2Settings>(&format!(
                r#"
                    registry = "{}"
                    repository = "{}"
                    manifestref_key = "{}"
                    fetch_concurrency = {}
                    anonymous_auth = true
                "#,
                &registry, &repo, DEFAULT_MANIFESTREF_KEY, DEFAULT_FETCH_CONCURRENCY,
            ))
            .unwrap(),
            // cache
            None,
            // prometheus registry
            None,
        )
        .unwrap(),
    );

    let graph = runtime
        .block_on(plugin.run_internal(InternalIO {
            graph: Default::default(),
            parameters: Default::default(),
        }))
        .unwrap()
        .graph;

    let expected_graph = {
        let input_edges = Some(vec![(0, 1)]);
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
                [
                    (String::from("version_suffix"), String::from("+amd64")),
                    (String::from("kind"), String::from("test")),
                ]
                .iter()
                .cloned()
                .collect(),
            ),
        ];
        TestGraphBuilder::new()
            .with_image(&format!("{}/{}", registry, repo))
            .with_metadata(input_metadata)
            .with_edges(input_edges)
            .with_version_template("0.0.{{i}}")
            .enable_payload_suffix(true)
            .build()
    };

    crate::testing::compare_graphs_verbose(
        expected_graph,
        graph,
        cincinnati::testing::CompareGraphsVerboseSettings {
            unwanted_metadata_keys: &[
                DEFAULT_MANIFESTREF_KEY,
                "io.openshift.upgrades.graph.release.channels",
                "io.openshift.upgrades.graph.previous.add",
                "io.openshift.upgrades.graph.previous.remove",
                "io.openshift.upgrades.graph.release.arch",
            ],

            payload_replace_sha_by_tag_left: false,
            payload_replace_sha_by_tag_right: true,

            ..Default::default()
        },
    )
    .unwrap();
}

#[test]
fn scrape_public_with_cyclic_metadata_fails() -> Fallible<()> {
    let (mut runtime, _) = common_init();

    let registry = DEFAULT_SCRAPE_REGISTRY;
    let repo = "cincinnati-ci-public/cincinnati-test-cyclic-public-manual";

    let plugin = Box::new(ReleaseScrapeDockerv2Plugin::try_new(
        // settings
        toml::from_str::<ReleaseScrapeDockerv2Settings>(&format!(
            r#"
                    registry = "{}"
                    repository = "{}"
                    manifestref_key = "{}"
                    fetch_concurrency = {}
                    anonymous_auth = true
                "#,
            &registry, &repo, DEFAULT_MANIFESTREF_KEY, DEFAULT_FETCH_CONCURRENCY,
        ))?,
        // cache
        None,
        // prometheus registry
        None,
    )?);

    let err = runtime
        .block_on(plugin.run_internal(InternalIO {
            graph: Default::default(),
            parameters: Default::default(),
        }))
        .expect_err("create_graph succeeded despite cyclic metadata");

    ensure!(
        err.downcast_ref::<crate::WouldCycle<crate::Empty>>()
            .is_some(),
        "error {:#?} has wrong type",
        err,
    );

    Ok(())
}

#[test]
fn scrape_public_multiarch_manual_succeeds() -> Fallible<()> {
    let (mut runtime, _) = common_init();

    let registry = DEFAULT_SCRAPE_REGISTRY;
    let repo = "cincinnati-ci-public/cincinnati-test-public-multiarch-manual";

    let plugin = Box::new(ReleaseScrapeDockerv2Plugin::try_new(
        // settings
        toml::from_str::<ReleaseScrapeDockerv2Settings>(&format!(
            r#"
                    registry = "{}"
                    repository = "{}"
                    manifestref_key = "{}"
                    fetch_concurrency = {}
                    anonymous_auth = true
                "#,
            &registry, &repo, DEFAULT_MANIFESTREF_KEY, DEFAULT_FETCH_CONCURRENCY,
        ))?,
        // cache
        None,
        // prometheus registry
        None,
    )?);

    let graph: cincinnati::Graph = runtime
        .block_on(plugin.run_internal(InternalIO {
            graph: Default::default(),
            parameters: Default::default(),
        }))?
        .graph;

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
            .with_image(&format!("{}/{}", registry, repo))
            .with_metadata(input_metadata)
            .with_edges(input_edges)
            .with_version_template("0.0.{{i}}")
            .enable_payload_suffix(true)
            .build()
    };

    crate::testing::compare_graphs_verbose(
        expected_graph,
        graph,
        cincinnati::testing::CompareGraphsVerboseSettings {
            unwanted_metadata_keys: &[
                DEFAULT_MANIFESTREF_KEY,
                "io.openshift.upgrades.graph.release.channels",
                "io.openshift.upgrades.graph.previous.add",
                "io.openshift.upgrades.graph.previous.remove",
                "io.openshift.upgrades.graph.release.arch",
            ],

            payload_replace_sha_by_tag_left: false,
            payload_replace_sha_by_tag_right: true,

            ..Default::default()
        },
    )
}
