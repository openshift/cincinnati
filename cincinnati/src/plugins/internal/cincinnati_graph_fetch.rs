//! Plugin which implements fetching a Cincinnati graph via HTTP from a `/v1/graph`-compliant endpoint.
//!
//! Instead of processing the input graph, this plugin fetches a graph from a
//! remote endpoint, which makes it effectively discard any given input graph.

use crate as cincinnati;

use self::cincinnati::plugins::prelude::*;
use self::cincinnati::plugins::prelude_plugin_impl::*;
use self::cincinnati::CONTENT_TYPE;

use commons::prelude_errors::*;
use commons::tracing::{get_tracer, set_context};
use opentelemetry::{
    trace::{mark_span_as_active, Tracer},
    Context as ot_context,
};

use commons::prelude_errors::Context;
use commons::GraphError;
use prometheus::Counter;
use reqwest;
use reqwest::header::{HeaderMap, HeaderValue, ACCEPT};
use std::time::Duration;

/// Default URL to upstream graph provider.
pub static DEFAULT_UPSTREAM_URL: &str = "http://localhost:8080/v1/graph";

/// Default graph-builder connection timeout in seconds.
pub static DEFAULT_TIMEOUT_SECS: u64 = 30;

/// Plugin settings.
#[derive(Clone, CustomDebug, Deserialize, SmartDefault)]
#[serde(default)]
struct CincinnatiGraphFetchSettings {
    #[default(DEFAULT_UPSTREAM_URL.to_string())]
    upstream: String,

    #[default(DEFAULT_TIMEOUT_SECS)]
    timeout: u64,
}

/// Graph fetcher for Cincinnati `/v1/graph` endpoints.
#[derive(CustomDebug)]
pub struct CincinnatiGraphFetchPlugin {
    /// The upstream from which to fetch the graph
    pub upstream: String,

    /// The optional metric for counting upstream requests
    #[debug(skip)]
    pub http_upstream_reqs: Counter,

    /// The optional metric for counting failed upstream requests
    #[debug(skip)]
    pub http_upstream_errors_total: Counter,

    // graph-builder connection client
    client: reqwest::Client,
}

impl PluginSettings for CincinnatiGraphFetchSettings {
    fn build_plugin(&self, registry: Option<&prometheus::Registry>) -> Fallible<BoxedPlugin> {
        let cfg = self.clone();
        let plugin = CincinnatiGraphFetchPlugin::try_new(cfg.upstream, cfg.timeout, registry)?;
        Ok(new_plugin!(InternalPluginWrapper(plugin)))
    }
}

impl CincinnatiGraphFetchPlugin {
    /// Plugin name, for configuration.
    pub const PLUGIN_NAME: &'static str = "cincinnati-graph-fetch";

    /// Validate plugin configuration and fill in defaults.
    pub fn deserialize_config(cfg: toml::Value) -> Fallible<Box<dyn PluginSettings>> {
        let settings: CincinnatiGraphFetchSettings = cfg.try_into()?;

        ensure!(!settings.upstream.is_empty(), "empty upstream");

        Ok(Box::new(settings))
    }

    fn try_new(
        upstream: String,
        timeout: u64,
        prometheus_registry: Option<&prometheus::Registry>,
    ) -> Fallible<Self> {
        let http_upstream_reqs = Counter::new(
            "http_upstream_requests_total",
            "Total number of HTTP upstream requests",
        )?;

        let http_upstream_errors_total = Counter::new(
            "http_upstream_errors_total",
            "Total number of HTTP upstream unreachable errors",
        )?;

        if let Some(registry) = &prometheus_registry {
            registry.register(Box::new(http_upstream_reqs.clone()))?;
            registry.register(Box::new(http_upstream_errors_total.clone()))?;
        };

        let client = reqwest::ClientBuilder::new()
            .gzip(true)
            .timeout(Duration::from_secs(timeout))
            .build()
            .context("Building reqwest client")?;

        Ok(Self {
            upstream,
            http_upstream_reqs,
            http_upstream_errors_total,
            client,
        })
    }
}

impl CincinnatiGraphFetchPlugin {
    async fn do_run_internal(self: &Self, io: InternalIO) -> Fallible<InternalIO> {
        // extract current trace ID from headers
        // this is required to make graph-builder trace a child of police-engine request
        let mut headers = HeaderMap::new();
        headers.insert(ACCEPT, HeaderValue::from_static(CONTENT_TYPE));
        {
            let span = get_tracer().start("");
            let _active_span = mark_span_as_active(span);
            let cx = ot_context::current();
            set_context(cx, &mut headers).context("failed to set the tracing context")?;
        }

        trace!("getting graph from upstream at {}", self.upstream);
        self.http_upstream_reqs.inc();

        let res = self
            .client
            .get(&self.upstream)
            .headers(headers)
            .send()
            .map_err(|e| GraphError::FailedUpstreamFetch(e.to_string()))
            .await?;

        if !res.status().is_success() {
            return Err(GraphError::FailedUpstreamFetch(res.status().to_string()).into());
        }

        let body = res
            // TODO(steveeJ): find a way to make this fail in a test
            .bytes()
            .map_err(move |e| GraphError::FailedUpstreamFetch(e.to_string()))
            .await?;

        let graph =
            serde_json::from_slice(&body).map_err(|e| GraphError::FailedJsonIn(e.to_string()))?;

        Ok(InternalIO {
            graph,
            parameters: io.parameters,
        })
    }
}

#[async_trait]
impl InternalPlugin for CincinnatiGraphFetchPlugin {
    const PLUGIN_NAME: &'static str = Self::PLUGIN_NAME;

    async fn run_internal(self: &Self, io: InternalIO) -> Fallible<InternalIO> {
        self.do_run_internal(io)
            .map_err(move |e| {
                error!("error fetching graph: {}", e);
                self.http_upstream_errors_total.inc();
                e
            })
            .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cincinnati::testing::generate_custom_graph;
    use commons::metrics::{self, RegistryWrapper};
    use commons::testing::{self, init_runtime};
    use memchr::memmem;
    use prometheus::Registry;

    macro_rules! fetch_upstream_success_test {
        (
            name: $name:ident,
            mock_body: $mock_body:expr,
            expected_graph: $expected_graph:expr,

        ) => {
            #[test]
            fn $name() -> Fallible<()> {
                let runtime = init_runtime()?;

                // run mock graph-builder
                let _m = mockito::mock("GET", "/")
                    .with_status(200)
                    .with_header("content-type", "application/json")
                    .with_body($mock_body.to_string())
                    .create();

                let timeout: u64 = 30;
                let plugin =
                    CincinnatiGraphFetchPlugin::try_new(mockito::server_url(), timeout, None)?;
                let http_upstream_reqs = plugin.http_upstream_reqs.clone();
                let http_upstream_errors_total = plugin.http_upstream_errors_total.clone();

                assert_eq!(0, http_upstream_reqs.clone().get() as u64);
                assert_eq!(0, http_upstream_errors_total.clone().get() as u64);

                let future_processed_graph = plugin.run_internal(InternalIO {
                    graph: Default::default(),
                    parameters: Default::default(),
                });

                let processed_graph = runtime
                    .block_on(future_processed_graph)
                    .expect("plugin run failed")
                    .graph;

                assert_eq!($expected_graph, processed_graph);

                assert_eq!(1, http_upstream_reqs.get() as u64);
                assert_eq!(0, http_upstream_errors_total.get() as u64);

                Ok(())
            }
        };
    }

    fetch_upstream_success_test!(
        name: fetch_success_empty_graph_fetch,
        mock_body: &serde_json::to_string(&cincinnati::Graph::default())?,
        expected_graph: cincinnati::Graph::default(),
    );

    fetch_upstream_success_test!(
        name: fetch_success_simple_graph_fetch,
        mock_body: &serde_json::to_string(&generate_custom_graph(
            "image",
            (0..3).into_iter().map(|i|(i, Default::default())).collect(),
            Some(vec![(0, 1), (1, 2)]),
        ))?,
        expected_graph: generate_custom_graph(
            "image",
            (0..3).into_iter().map(|i|(i, Default::default())).collect(),
            Some(vec![(0, 1), (1, 2)]),
        ),
    );

    macro_rules! fetch_upstream_failure_test {
        (
            name: $name:ident,
            upstream: $upstream:expr,
            mock_status: $mock_status:expr,
            mock_header: $mock_header:expr,
            mock_body: $mock_body:expr,
        ) => {
            #[test]
            fn $name() -> Fallible<()> {
                let runtime = init_runtime()?;
                // run mock graph-builder
                let _m = mockito::mock("GET", "/")
                    .with_status($mock_status)
                    .with_header($mock_header.0, $mock_header.1)
                    .with_body($mock_body.to_string())
                    .create();

                let plugin = CincinnatiGraphFetchPlugin::try_new($upstream.to_string(), 30, None)?;
                let http_upstream_reqs = plugin.http_upstream_reqs.clone();
                let http_upstream_errors_total = plugin.http_upstream_errors_total.clone();

                assert_eq!(0, http_upstream_reqs.clone().get() as u64);
                assert_eq!(0, http_upstream_errors_total.clone().get() as u64);

                let future_result = plugin.run_internal(InternalIO {
                    graph: Default::default(),
                    parameters: Default::default(),
                });

                assert!(runtime.block_on(future_result).is_err());

                assert_eq!(1, http_upstream_reqs.get() as usize);
                assert_eq!(1, http_upstream_errors_total.get() as usize);

                Ok(())
            }
        };
    }

    fetch_upstream_failure_test!(
        name: fetch_fail_invalid_url,
        upstream: "invalid.url",
        mock_status: 0,
        mock_header: ("", ""),
        mock_body: "",
    );

    fetch_upstream_failure_test!(
        name: fetch_fail_unreachable_server_url,
        upstream: "http://not.reachable.test",
        mock_status: 0,
        mock_header: ("", ""),
        mock_body: "",
    );

    fetch_upstream_failure_test!(
        name: fetch_fail_request_fails_with_404,
        upstream: &mockito::server_url(),
        mock_status: 404,
        mock_header: ("content-type", "application/json"),
        mock_body: "NOT_FOUND",
    );

    fetch_upstream_failure_test!(
        name: fetch_fail_graph_deserialization,
        upstream: &mockito::server_url(),
        mock_status: 200,
        mock_header: ("content-type", "application/json"),
        mock_body: "{not a valid graph}",
    );

    #[test]
    fn register_metrics() -> Fallible<()> {
        let rt = testing::init_runtime()?;

        let metrics_prefix = "test_service".to_string();
        let registry: &'static Registry = Box::leak(Box::new(metrics::new_registry(Some(
            metrics_prefix.clone(),
        ))?));

        let timeout: u64 = 30;

        let _ =
            CincinnatiGraphFetchPlugin::try_new(mockito::server_url(), timeout, Some(registry))?;

        let metrics_call = metrics::serve::<metrics::RegistryWrapper>(actix_web::web::Data::new(
            RegistryWrapper(registry),
        ));
        let resp = rt.block_on(metrics_call);

        assert_eq!(resp.status(), 200);
        if let actix_web::body::ResponseBody::Body(body) = resp.body() {
            if let actix_web::body::Body::Bytes(bytes) = body {
                assert!(!bytes.is_empty());
                println!("{:?}", std::str::from_utf8(bytes.as_ref()));
                assert!(memmem::find_iter(
                    bytes.as_ref(),
                    format!("{}_http_upstream_errors_total 0\n", &metrics_prefix).as_bytes(),
                )
                .next()
                .is_some());
                assert!(memmem::find_iter(
                    bytes.as_ref(),
                    format!("{}_http_upstream_requests_total 0\n", &metrics_prefix).as_bytes(),
                )
                .next()
                .is_some());
            } else {
                bail!("expected Body")
            }
        } else {
            bail!("expected bytes in body")
        };

        Ok(())
    }
}
