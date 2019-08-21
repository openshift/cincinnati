//! The web module can be used to talk to webservice which expose an endpoint
//! according to the protobuf scheme

use std::path::PathBuf;
use url::Url;

/// Struct for implementing the client side of a web plugin
#[derive(Debug)]
pub struct _WebPluginClient {
    pub url: Url,
    pub timeout: std::time::Duration,
    pub ca_cert_path: Option<PathBuf>,
    pub client_cert_path: Option<PathBuf>,
}

#[cfg(test)]
mod tests {
    use crate as cincinnati;
    use crate::plugins::{interface, ExternalIO, ExternalPlugin, InternalIO, PluginResult};
    use crate::tests::generate_graph;
    use commons::testing::init_runtime;
    use failure::Fallible;
    use plugins::AsyncIO;
    use std::convert::TryInto;

    struct DummyWebClient {
        callback: Box<Fn(interface::PluginExchange) -> PluginResult + Send + Sync>,
    }

    impl std::fmt::Debug for DummyWebClient {
        fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
            write!(f, "DummyWebClient")
        }
    }

    impl ExternalPlugin for DummyWebClient {
        fn run_external(self: &Self, io: ExternalIO) -> AsyncIO<ExternalIO> {
            let closure = || -> Fallible<ExternalIO> {
                let input: interface::PluginExchange = io.try_into()?;

                match (self.callback)(input) {
                    PluginResult::PluginExchange(exchange) => exchange.try_into(),
                    PluginResult::PluginError(error) => error.into(),
                }
            };

            Box::new(futures::future::result(closure()))
        }
    }

    #[test]
    fn detect_external_success() {
        let mut runtime = init_runtime().unwrap();

        fn callback(mut input: interface::PluginExchange) -> PluginResult {
            let graph: cincinnati::Graph = input.take_graph().into();

            trace!(
                "[external passthrough plugin] got graph with {} nodes",
                graph.releases_count()
            );

            let mut exchange: interface::PluginExchange = interface::PluginExchange::new();
            exchange.set_graph(graph.into());
            exchange.set_parameters(input.get_parameters().to_owned());

            PluginResult::PluginExchange(exchange)
        }

        let plugin = Box::new(DummyWebClient {
            callback: Box::new(callback),
        });

        let input_internal = InternalIO {
            graph: generate_graph(),
            parameters: [("hello".to_string(), "plugin".to_string())]
                .iter()
                .cloned()
                .collect(),
        };

        let input: ExternalIO = input_internal.clone().try_into().unwrap();

        let future_output_external = plugin.run_external(input);

        let output_external: ExternalIO = runtime.block_on(future_output_external).unwrap();
        let output_internal: InternalIO = output_external.try_into().unwrap();

        assert_eq!(output_internal, input_internal);
    }

    #[test]
    fn detect_external_error() {
        let mut runtime = init_runtime().unwrap();

        fn callback(_: interface::PluginExchange) -> PluginResult {
            let mut given_error = interface::PluginError::new();
            given_error.set_kind(interface::PluginError_Kind::INTERNAL_FAILURE);
            given_error.set_value("test succeeds on error".to_string());
            PluginResult::PluginError(given_error)
        };
        let expected_result = callback(interface::PluginExchange::new());

        let plugin = Box::new(DummyWebClient {
            callback: Box::new(callback),
        });

        let input_internal = InternalIO {
            graph: generate_graph(),
            parameters: [("hello".to_string(), "plugin".to_string())]
                .iter()
                .cloned()
                .collect(),
        };

        let input: ExternalIO = input_internal.clone().try_into().unwrap();

        let future_output_result_external = plugin.run_external(input);

        let output_result_external = runtime.block_on(future_output_result_external);
        let output_result: PluginResult = output_result_external.try_into().unwrap();

        assert_eq!(expected_result, output_result);
    }
}
