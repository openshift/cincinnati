//! The web module can be used to talk to webservice which expose an endpoint
//! according to the protobuf scheme

use std::path::PathBuf;
use url::Url;

/// Struct for implementing the client side of a web plugin
pub struct _WebPluginClient {
    pub url: Url,
    pub timeout: std::time::Duration,
    pub ca_cert_path: Option<PathBuf>,
    pub client_cert_path: Option<PathBuf>,
}

#[cfg(test)]
mod tests {
    use crate as cincinnati;
    use failure::Fallible;
    use crate::plugins::{interface, ExternalIO, ExternalPlugin, InternalIO, PluginResult};
    use crate::tests::generate_graph;
    use try_from::TryInto;

    struct DummyWebClient {
        callback: Box<Fn(interface::PluginExchange) -> PluginResult>,
    }

    impl ExternalPlugin for DummyWebClient {
        fn run_external(&self, io: ExternalIO) -> Fallible<ExternalIO> {
            let input: interface::PluginExchange = io.try_into()?;

            match (self.callback)(input) {
                PluginResult::PluginExchange(exchange) => exchange.try_into(),
                PluginResult::PluginError(error) => error.into(),
            }
        }
    }

    #[test]
    fn detect_external_success() {
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

        let plugin = DummyWebClient {
            callback: Box::new(callback),
        };

        let input_internal = InternalIO {
            graph: generate_graph(),
            parameters: [("hello".to_string(), "plugin".to_string())]
                .iter()
                .cloned()
                .collect(),
        };

        let input: ExternalIO = input_internal.clone().try_into().unwrap();

        let output_external: ExternalIO = plugin.run_external(input).unwrap();
        let output_internal: InternalIO = output_external.try_into().unwrap();

        assert_eq!(output_internal, input_internal);
    }

    #[test]
    fn detect_external_error() {
        fn callback(_: interface::PluginExchange) -> PluginResult {
            let mut given_error = interface::PluginError::new();
            given_error.set_kind(interface::PluginError_Kind::INTERNAL_FAILURE);
            given_error.set_value("test succeeds on error".to_string());
            PluginResult::PluginError(given_error)
        };
        let expected_result = callback(interface::PluginExchange::new());

        let plugin = DummyWebClient {
            callback: Box::new(callback),
        };

        let input_internal = InternalIO {
            graph: generate_graph(),
            parameters: [("hello".to_string(), "plugin".to_string())]
                .iter()
                .cloned()
                .collect(),
        };

        let input: ExternalIO = input_internal.clone().try_into().unwrap();

        let output_result_external = plugin.run_external(input);
        let output_result: Fallible<PluginResult> = output_result_external.try_into();

        assert_eq!(expected_result, output_result.unwrap());
    }
}
