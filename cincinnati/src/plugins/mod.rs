//! The plugin defines and implements the plugin interface, conversion boilerplate,
//! internal plugins, and web plugin helpers

#[macro_use]
pub mod macros;

mod catalog;
pub mod external;
pub mod interface;
pub mod internal;

pub use self::catalog::{deserialize_config, PluginSettings};
use crate as cincinnati;
use failure::{Error, Fallible, ResultExt};
use crate::plugins::interface::{PluginError, PluginExchange};
use std::collections::HashMap;
use try_from::{TryFrom, TryInto};

/// Enum for the two IO variants used by InternalPlugin and ExternalPlugin respectively
pub enum PluginIO {
    InternalIO(InternalIO),
    ExternalIO(ExternalIO),
}

/// Error type which corresponds to interface::PluginError
#[derive(Debug, Fail)]
pub enum ExternalError {
    #[fail(display = "PluginError: {:?}", 0)]
    PluginError(PluginError),
}

/// Enum for wrapping the interface plugin output types
#[derive(Debug, PartialEq)]
pub enum PluginResult {
    PluginExchange(interface::PluginExchange),
    PluginError(interface::PluginError),
}

/// Struct used by the ExternalPlugin trait impl's
#[derive(Debug, PartialEq)]
#[cfg_attr(test, derive(Clone))]
pub struct InternalIO {
    pub graph: cincinnati::Graph,
    pub parameters: HashMap<String, String>,
}

/// Struct used by the InternalPlugin trait impl's
#[derive(Debug, PartialEq)]
#[cfg_attr(test, derive(Clone))]
pub struct ExternalIO {
    pub bytes: Vec<u8>,
}

/// Trait which fronts InternalPlugin and ExternalPlugin, allowing their trait objects to live in the same collection
pub trait Plugin<T>
where
    T: TryInto<PluginIO> + TryFrom<PluginIO>,
{
    fn run(&self, t: T) -> Fallible<T>;
}

/// Trait to be implemented by internal plugins with their native IO type
pub trait InternalPlugin {
    fn run_internal(&self, input: InternalIO) -> Fallible<InternalIO>;
}

/// Trait to be implemented by external plugins with its native IO type
///
/// There's a gotcha in that this type can't be used to access the information
/// directly as it's merely bytes.
pub trait ExternalPlugin {
    fn run_external(&self, input: ExternalIO) -> Fallible<ExternalIO>;
}

/// Dummy converter to satisfy the Trait
impl TryFrom<PluginIO> for PluginIO {
    type Err = Error;

    fn try_from(io: Self) -> Fallible<Self> {
        Ok(io)
    }
}

/// Dummy converter to satisfy the Trait
impl TryFrom<InternalIO> for PluginIO {
    type Err = Error;

    fn try_from(internal_io: InternalIO) -> Fallible<Self> {
        Ok(internal_io.into())
    }
}

/// Dummy converter to satisfty the Trait
impl TryFrom<ExternalIO> for PluginIO {
    type Err = Error;
    fn try_from(external_io: ExternalIO) -> Fallible<Self> {
        Ok(external_io.into())
    }
}

/// Convert from InternalIO to PluginIO
///
/// This merely wraps the struct into the enum variant.
impl From<InternalIO> for PluginIO {
    fn from(internal_io: InternalIO) -> Self {
        PluginIO::InternalIO(internal_io)
    }
}

/// Convert from ExternalIO to PluginIO
///
/// This merely wraps the struct into the enum variant.
impl From<ExternalIO> for PluginIO {
    fn from(external_io: ExternalIO) -> Self {
        PluginIO::ExternalIO(external_io)
    }
}

/// Converts ExternalIO to PluginExchange
///
/// This can fail because the ExternalIO bytes need to be deserialized into the
/// PluginExchange type.
impl TryFrom<ExternalIO> for PluginExchange {
    type Err = Error;

    fn try_from(external_io: ExternalIO) -> Fallible<Self> {
        protobuf::parse_from_bytes(&external_io.bytes)
            .context("could not parse ExternalIO to PluginExchange")
            .map_err(|e| e.into())
    }
}

/// Convert from ExternalIO to PluginError
///
/// This can fail because the ExternalIO bytes need to be deserialized into the
/// PluginError type.
impl TryFrom<ExternalIO> for PluginError {
    type Err = Error;

    fn try_from(external_io: ExternalIO) -> Fallible<Self> {
        protobuf::parse_from_bytes(&external_io.bytes)
            .context("could not parse ExternalIO to PluginError")
            .map_err(|e| e.into())
    }
}

/// Convert from ExternalIO to PluginResult
///
/// This can fail because the bytes of ExternalIO need to be deserialized into
/// either of PluginExchange or PluginError.
impl TryFrom<Fallible<ExternalIO>> for PluginResult {
    type Err = Error;

    fn try_from(external_io: Fallible<ExternalIO>) -> Fallible<Self> {
        match external_io {
            Ok(external_io) => {
                let exchange: interface::PluginExchange = external_io.try_into()?;
                Ok(PluginResult::PluginExchange(exchange))
            }
            Err(e) => match e.downcast::<ExternalError>() {
                Ok(ExternalError::PluginError(error)) => Ok(PluginResult::PluginError(error)),
                Err(e) => Err(e),
            },
        }
    }
}

/// Convert from interface::PluginError to a Fallible<ExternalIO> which has the PluginError embedded
impl From<interface::PluginError> for Fallible<ExternalIO> {
    fn from(plugin_error: interface::PluginError) -> Fallible<ExternalIO> {
        Err(ExternalError::PluginError(plugin_error).into())
    }
}

/// Convert from PluginExchange to ExternalIO
///
/// This can fail because PluginExchange needs to be serialized into the bytes.
impl TryFrom<PluginExchange> for ExternalIO {
    type Err = Error;

    fn try_from(exchange: PluginExchange) -> Fallible<Self> {
        use protobuf::Message;

        Ok(Self {
            bytes: exchange.write_to_bytes()?,
        })
    }
}

/// Try to convert from ExternalIO to InternalIO
///
/// This can fail because the ExternalIO bytes need to be deserialized into the
/// InternalIO type.
impl TryFrom<ExternalIO> for InternalIO {
    type Err = Error;

    fn try_from(external_io: ExternalIO) -> Fallible<Self> {
        let mut plugin_exchange: PluginExchange = external_io.try_into()?;

        Ok(Self {
            graph: plugin_exchange.take_graph().into(),
            parameters: plugin_exchange.take_parameters(),
        })
    }
}

/// Convert from an InternalIO to a PluginExchange
///
/// This conversion cannot fail.
impl From<InternalIO> for interface::PluginExchange {
    fn from(internal_io: InternalIO) -> Self {
        let mut plugin_exchange = Self::new();

        plugin_exchange.set_graph(internal_io.graph.into());
        plugin_exchange.set_parameters(internal_io.parameters);

        plugin_exchange
    }
}

/// Convert from InternalIO to ExternalIO
///
/// The serialization can apparently fail.
impl TryFrom<InternalIO> for ExternalIO {
    type Err = Error;

    fn try_from(internal_io: InternalIO) -> Fallible<Self> {
        let exchange: PluginExchange = internal_io.into();
        exchange.try_into()
    }
}

/// Try to convert from PluginIO to InternalIO
///
/// In case of the variant Plugion::ExternalIO this involves deserialization
/// which might fail.
impl TryFrom<PluginIO> for InternalIO {
    type Err = Error;

    fn try_from(plugin_io: PluginIO) -> Fallible<Self> {
        match plugin_io {
            PluginIO::InternalIO(internal_io) => Ok(internal_io),
            PluginIO::ExternalIO(external_io) => external_io.try_into(),
        }
    }
}

/// Compatibility impl to automatically convert the unvarianted PluginIO when the
/// variant struct ExternalIO is expected.
///
/// This may fail due to the possible failing conversion.
impl TryFrom<PluginIO> for ExternalIO {
    type Err = Error;

    fn try_from(plugin_io: PluginIO) -> Fallible<Self> {
        let final_io = match plugin_io {
            PluginIO::ExternalIO(external_io) => external_io,
            PluginIO::InternalIO(internal_io) => internal_io.try_into()?,
        };
        Ok(final_io)
    }
}

/// Wrapper struct for a universal implementation of Plugin<PluginIO> for all InternalPlugin implementors
pub struct InternalPluginWrapper<T>(pub T);

/// Wrapper struct for a universal implementation of Plugin<PluginIO> for all ExternalPlugin implementors
pub struct ExternalPluginWrapper<T>(pub T);

/// This implementation allows the process function to run ipmlementors of
/// InternalPlugin
impl<T> Plugin<PluginIO> for InternalPluginWrapper<T>
where
    T: InternalPlugin,
{
    fn run(&self, plugin_io: PluginIO) -> Fallible<PluginIO> {
        let internal_io = self.0.run_internal(plugin_io.try_into()?)?;
        internal_io.try_into()
    }
}

/// This implementation allows the process function to run ipmlementors of
/// ExternalPlugin
impl<T> Plugin<PluginIO> for ExternalPluginWrapper<T>
where
    T: ExternalPlugin,
{
    fn run(&self, plugin_io: PluginIO) -> Fallible<PluginIO> {
        let external_io = self.0.run_external(plugin_io.try_into()?)?;
        external_io.try_into()
    }
}

/// Processes all given Plugins sequentially.
///
/// This function automatically converts between the different IO representations
/// if necessary.
pub fn process<T>(plugins: &[Box<T>], initial_io: InternalIO) -> Fallible<cincinnati::Graph>
where
    T: Plugin<PluginIO> + ?Sized,
{
    if plugins.is_empty() {
        debug!("no plugins to process, passing through graph..");
        return Ok(initial_io.graph);
    }

    let initial_io = PluginIO::InternalIO(initial_io);
    let final_io = plugins
        .iter()
        .try_fold(initial_io, |last_io, next_plugin| next_plugin.run(last_io))?;

    match final_io {
        PluginIO::InternalIO(internal_io) => Ok(internal_io.graph),
        PluginIO::ExternalIO(external_io) => {
            let internal_io: InternalIO = external_io.try_into()?;
            Ok(internal_io.graph)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::plugins::Plugin;
    use crate::tests::generate_graph;

    #[test]
    fn convert_externalio_pluginresult() {
        let kind = interface::PluginError_Kind::INTERNAL_FAILURE;
        let value = "test succeeds on error".to_string();

        let mut original_error = interface::PluginError::new();
        original_error.set_kind(kind.clone());
        original_error.set_value(value.clone());

        let expected_result = PluginResult::PluginError(original_error.clone());

        let external_io: Fallible<ExternalIO> = original_error.into();

        let converted_result: PluginResult = external_io.try_into().unwrap();

        assert_eq!(converted_result, expected_result);
    }

    #[test]
    fn convert_roundtrip_internalio_externalio() {
        let graph = generate_graph();
        let input_internal = InternalIO {
            graph: graph.clone(),
            parameters: [("hello".to_string(), "plugin".to_string())]
                .iter()
                .cloned()
                .collect(),
        };

        let output_external: ExternalIO = input_internal.clone().try_into().unwrap();
        let output_internal: InternalIO = output_external.try_into().unwrap();

        assert_eq!(input_internal, output_internal);
    }

    struct TestInternalPlugin {}
    impl InternalPlugin for TestInternalPlugin {
        fn run_internal(&self, io: InternalIO) -> Fallible<InternalIO> {
            Ok(io)
        }
    }

    struct TestExternalPlugin {}
    impl ExternalPlugin for TestExternalPlugin {
        fn run_external(&self, io: ExternalIO) -> Fallible<ExternalIO> {
            Ok(io)
        }
    }

    impl Plugin<InternalIO> for TestInternalPlugin {
        fn run(&self, io: InternalIO) -> Fallible<InternalIO> {
            Ok(io)
        }
    }

    impl Plugin<ExternalIO> for TestExternalPlugin {
        fn run(&self, io: ExternalIO) -> Fallible<ExternalIO> {
            Ok(io)
        }
    }

    #[test]
    fn process_plugins_roundtrip_external_internal() {
        let plugins: Vec<Box<Plugin<PluginIO>>> = vec![
            Box::new(ExternalPluginWrapper(TestExternalPlugin {})),
            Box::new(InternalPluginWrapper(TestInternalPlugin {})),
            Box::new(ExternalPluginWrapper(TestExternalPlugin {})),
        ];

        let initial_io = InternalIO {
            graph: generate_graph(),
            parameters: [("hello".to_string(), "plugin".to_string())]
                .iter()
                .cloned()
                .collect(),
        };

        let final_io: cincinnati::Graph =
            process(&plugins, initial_io.clone()).expect("plugin processing failed");

        assert_eq!(initial_io.graph, final_io);
    }
}
