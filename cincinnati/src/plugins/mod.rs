//! The plugin defines and implements the plugin interface, conversion boilerplate,
//! internal plugins, and web plugin helpers

#[macro_use]
pub mod macros;

mod catalog;
pub mod external;
pub mod interface;
pub mod internal;

pub use self::catalog::{build_plugins, deserialize_config, PluginSettings};
use crate as cincinnati;
use crate::plugins::interface::{PluginError, PluginExchange};
use failure::{Error, Fallible, ResultExt};
use futures::IntoFuture;
use futures::{Future, Stream};
use std::collections::HashMap;
use std::convert::{TryFrom, TryInto};
use std::fmt::Debug;

pub mod prelude {
    pub use super::AsyncIO;
    pub use super::BoxedPlugin;
    pub use super::ExternalPluginWrapper;
    pub use super::InternalPluginWrapper;
    pub use super::{build_plugins, deserialize_config, PluginSettings};
    pub use crate::{new_plugin, new_plugins};
    pub use futures_locks;
    pub use std::iter::FromIterator;
}

/// Convenience type to wrap other types in a Future
pub type AsyncIO<T> = Box<dyn Future<Item = T, Error = Error> + Send>;

/// Convenience type for the thread-safe storage of plugins
pub type BoxedPlugin = Box<dyn Plugin<PluginIO>>;

// NOTE(lucab): this abuses `Debug`, because `PartialEq` is not object-safe and
// thus cannot be required on the underlying trait. It is a crude hack, but
// only meant to be used by test assertions.
impl PartialEq<BoxedPlugin> for BoxedPlugin {
    fn eq(&self, other: &Self) -> bool {
        format!("{:?}", self) == format!("{:?}", other)
    }
}

/// Enum for the two IO variants used by InternalPlugin and ExternalPlugin respectively
#[derive(Debug)]
#[cfg_attr(test, derive(Clone))]
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
    Self: Sync + Send + Debug,
    T: TryInto<PluginIO> + TryFrom<PluginIO>,
    T: Sync + Send,
{
    fn run(self: &Self, t: T) -> AsyncIO<T>;
}

/// Trait to be implemented by internal plugins with their native IO type
pub trait InternalPlugin {
    fn run_internal(self: &Self, input: InternalIO) -> AsyncIO<InternalIO>;
}

/// Trait to be implemented by external plugins with its native IO type
///
/// There's a gotcha in that this type can't be used to access the information
/// directly as it's merely bytes.
pub trait ExternalPlugin
where
    Self: Debug,
{
    fn run_external(self: &Self, input: ExternalIO) -> AsyncIO<ExternalIO>;
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
    type Error = Error;

    fn try_from(external_io: ExternalIO) -> Fallible<Self> {
        protobuf::parse_from_bytes(&external_io.bytes)
            .context("could not parse ExternalIO to PluginExchange")
            .map_err(Into::into)
    }
}

/// Convert from ExternalIO to PluginError
///
/// This can fail because the ExternalIO bytes need to be deserialized into the
/// PluginError type.
impl TryFrom<ExternalIO> for PluginError {
    type Error = Error;

    fn try_from(external_io: ExternalIO) -> Fallible<Self> {
        protobuf::parse_from_bytes(&external_io.bytes)
            .context("could not parse ExternalIO to PluginError")
            .map_err(Into::into)
    }
}

/// Convert from ExternalIO to PluginResult
///
/// This can fail because the bytes of ExternalIO need to be deserialized into
/// either of PluginExchange or PluginError.
impl TryFrom<Fallible<ExternalIO>> for PluginResult {
    type Error = Error;

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
    type Error = Error;

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
    type Error = Error;

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
    type Error = Error;

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
    type Error = Error;

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
    type Error = Error;

    fn try_from(plugin_io: PluginIO) -> Fallible<Self> {
        let final_io = match plugin_io {
            PluginIO::ExternalIO(external_io) => external_io,
            PluginIO::InternalIO(internal_io) => internal_io.try_into()?,
        };
        Ok(final_io)
    }
}

/// Wrapper struct for a universal implementation of Plugin<PluginIO> for all InternalPlugin implementors
#[derive(Debug)]
pub struct InternalPluginWrapper<T>(pub T);

/// Wrapper struct for a universal implementation of Plugin<PluginIO> for all ExternalPlugin implementors
#[derive(Debug)]
pub struct ExternalPluginWrapper<T>(pub T);

/// This implementation allows the process function to run ipmlementors of
/// InternalPlugin
impl<T> Plugin<PluginIO> for InternalPluginWrapper<T>
where
    T: InternalPlugin,
    T: Sync + Send + Debug,
{
    fn run(self: &Self, plugin_io: PluginIO) -> AsyncIO<PluginIO> {
        let internal_io: InternalIO = match plugin_io.try_into() {
            Ok(internal_io) => internal_io,
            Err(e) => return Box::new(futures::future::err(e)),
        };

        Box::new(
            self.0
                .run_internal(internal_io)
                .and_then(|internal_io| -> Fallible<PluginIO> { Ok(internal_io.into()) }),
        )
    }
}

/// This implementation allows the process function to run ipmlementors of
/// ExternalPlugin
impl<T> Plugin<PluginIO> for ExternalPluginWrapper<T>
where
    T: ExternalPlugin,
    T: Sync + Send + Debug,
{
    fn run(self: &Self, plugin_io: PluginIO) -> AsyncIO<PluginIO> {
        let external_io: ExternalIO = match plugin_io.try_into() {
            Ok(external_io) => external_io,
            Err(e) => return Box::new(futures::future::err(e)),
        };

        Box::new(
            self.0
                .run_external(external_io)
                .and_then(|external_io| -> Fallible<PluginIO> { Ok(external_io.into()) }),
        )
    }
}

/// Processes all given Plugins sequentially.
///
/// This function automatically converts between the different IO representations
/// if necessary.
pub fn process<T>(plugins: T, initial_io: PluginIO) -> AsyncIO<InternalIO>
where
    T: Iterator<Item = &'static BoxedPlugin>,
    T: Sync + Send,
    T: 'static,
{
    let future_result = futures::stream::iter_ok::<_, Error>(plugins)
        .fold(initial_io, |io, next_plugin| next_plugin.run(io))
        .into_future()
        .and_then(TryInto::try_into);

    Box::new(future_result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::plugins::Plugin;
    use crate::tests::generate_graph;
    use futures_locks::Mutex as FuturesMutex;
    use std::collections::HashMap;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;

    #[test]
    fn convert_externalio_pluginresult() {
        let kind = interface::PluginError_Kind::INTERNAL_FAILURE;
        let value = "test succeeds on error".to_string();

        let mut original_error = interface::PluginError::new();
        original_error.set_kind(kind);
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

    #[derive(Debug)]
    struct TestInternalPlugin {
        counter: AtomicUsize,
        dict: Arc<FuturesMutex<HashMap<usize, bool>>>,
    }
    impl InternalPlugin for TestInternalPlugin {
        fn run_internal(self: &Self, mut io: InternalIO) -> AsyncIO<InternalIO> {
            self.counter.fetch_add(1, Ordering::SeqCst);
            let counter = self.counter.load(Ordering::SeqCst);

            let future_io = self
                .dict
                .lock()
                .map_err(|_| failure::err_msg("could not lock self.dict"))
                .map(move |mut dict_guard| {
                    (*dict_guard).insert(counter, true);

                    io.parameters
                        .insert("COUNTER".to_string(), format!("{}", counter));

                    io
                });

            Box::new(future_io)
        }
    }
    impl Plugin<InternalIO> for TestInternalPlugin {
        fn run(self: &Self, io: InternalIO) -> AsyncIO<InternalIO> {
            Box::new(futures::future::ok(io))
        }
    }

    #[derive(Debug)]
    struct TestExternalPlugin {}
    impl ExternalPlugin for TestExternalPlugin {
        fn run_external(self: &Self, io: ExternalIO) -> AsyncIO<ExternalIO> {
            Box::new(futures::future::ok(io))
        }
    }
    impl Plugin<ExternalIO> for TestExternalPlugin {
        fn run(self: &Self, io: ExternalIO) -> AsyncIO<ExternalIO> {
            Box::new(futures::future::ok(io))
        }
    }

    #[test]
    fn process_plugins_roundtrip_external_internal() -> Fallible<()> {
        let mut runtime = commons::testing::init_runtime()?;

        lazy_static! {
            static ref PLUGINS: Vec<BoxedPlugin> = new_plugins!(
                ExternalPluginWrapper(TestExternalPlugin {}),
                InternalPluginWrapper(TestInternalPlugin {
                    counter: Default::default(),
                    dict: Arc::new(FuturesMutex::new(Default::default())),
                }),
                ExternalPluginWrapper(TestExternalPlugin {})
            );
        }

        let initial_internalio = InternalIO {
            graph: generate_graph(),
            parameters: [("hello".to_string(), "plugin".to_string())]
                .iter()
                .cloned()
                .collect(),
        };

        let expected_internalio = InternalIO {
            graph: generate_graph(),
            parameters: [
                ("hello".to_string(), "plugin".to_string()),
                ("COUNTER".to_string(), "1".to_string()),
            ]
            .iter()
            .cloned()
            .collect(),
        };

        let plugins_future: AsyncIO<InternalIO> = super::process(
            PLUGINS.iter(),
            PluginIO::InternalIO(initial_internalio.clone()),
        );

        let result_internalio: InternalIO = runtime.block_on(plugins_future)?;

        assert_eq!(expected_internalio, result_internalio);

        Ok(())
    }

    #[test]
    fn process_plugins_loop() -> Fallible<()> {
        let mut runtime = commons::testing::init_runtime()?;

        lazy_static! {
            static ref PLUGINS: Vec<BoxedPlugin> = new_plugins!(
                ExternalPluginWrapper(TestExternalPlugin {}),
                InternalPluginWrapper(TestInternalPlugin {
                    counter: Default::default(),
                    dict: Arc::new(FuturesMutex::new(Default::default())),
                }),
                ExternalPluginWrapper(TestExternalPlugin {})
            );
        }

        let initial_internalio = InternalIO {
            graph: generate_graph(),
            parameters: [("hello".to_string(), "plugin".to_string())]
                .iter()
                .cloned()
                .collect(),
        };

        let runs: usize = 10;

        for i in 0..runs {
            let expected_internalio = InternalIO {
                graph: generate_graph(),
                parameters: [
                    ("hello".to_string(), "plugin".to_string()),
                    ("COUNTER".to_string(), format!("{}", i + 1)),
                ]
                .iter()
                .cloned()
                .collect(),
            };

            let plugins_future: AsyncIO<InternalIO> = process(
                PLUGINS.iter(),
                PluginIO::InternalIO(initial_internalio.clone()),
            );

            let result_internalio: InternalIO = runtime.block_on(plugins_future)?;

            assert_eq!(expected_internalio, result_internalio);
        }

        Ok(())
    }
}
