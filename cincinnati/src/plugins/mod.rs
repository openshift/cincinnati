//! The plugin defines and implements the plugin interface, conversion boilerplate,
//! internal plugins, and web plugin helpers

#[macro_use]
pub mod macros;

pub mod catalog;
pub mod external;
pub mod interface;
pub mod internal;

use crate as cincinnati;

use self::cincinnati::plugins::interface::{PluginError, PluginExchange};

use async_trait::async_trait;
pub use commons::prelude_errors::*;
use commons::tracing::get_tracer;
use std::collections::HashMap;
use std::convert::{TryFrom, TryInto};
use std::fmt::Debug;

use opentelemetry::{
    trace::{mark_span_as_active, FutureExt, Tracer},
    Context as ot_context,
};

pub mod prelude {
    use crate as cincinnati;

    use self::cincinnati::plugins;

    pub use plugins::{BoxedPlugin, InternalPluginWrapper};

    pub use plugins::catalog::PluginSettings;
    pub use plugins::internal::arch_filter::ArchFilterPlugin;
    pub use plugins::internal::channel_filter::ChannelFilterPlugin;
    pub use plugins::internal::cincinnati_graph_fetch::CincinnatiGraphFetchPlugin;
    pub use plugins::internal::edge_add_remove::EdgeAddRemovePlugin;
    pub use plugins::internal::github_openshift_secondary_metadata_scraper::{
        GithubOpenshiftSecondaryMetadataScraperPlugin,
        GithubOpenshiftSecondaryMetadataScraperSettings,
    };
    pub use plugins::internal::metadata_fetch_quay::QuayMetadataFetchPlugin;
    pub use plugins::internal::node_remove::NodeRemovePlugin;
    pub use plugins::internal::openshift_secondary_metadata_parser::{
        OpenshiftSecondaryMetadataParserPlugin, OpenshiftSecondaryMetadataParserSettings,
    };
    pub use plugins::internal::release_scrape_dockerv2::{
        ReleaseScrapeDockerv2Plugin, ReleaseScrapeDockerv2Settings,
    };

    pub use std::iter::FromIterator;

    pub use commons::prelude_errors::*;
}

pub mod prelude_plugin_impl {
    use self::cincinnati::plugins;
    use crate as cincinnati;

    pub use self::cincinnati::{daggy, ReleaseId};
    pub use plugins::catalog::PluginSettings;
    pub use plugins::{BoxedPlugin, InternalIO, InternalPlugin, InternalPluginWrapper};

    pub use async_trait::async_trait;
    pub use commons::prelude_errors::*;
    pub use custom_debug_derive::Debug as CustomDebug;
    pub use futures::TryFutureExt;
    pub use log::{debug, error, info, trace, warn};
    pub use serde::{de::DeserializeOwned, Deserialize};
    pub use smart_default::SmartDefault;
    pub use std::path::PathBuf;
    pub use std::str::FromStr;
}

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
    #[error("PluginError: {:?}", 0)]
    PluginError(PluginError),
}

/// Enum for wrapping the interface plugin output types
#[derive(Debug, PartialEq)]
pub enum PluginResult {
    PluginExchange(interface::PluginExchange),
    PluginError(interface::PluginError),
}

/// Struct used by the ExternalPlugin trait impl's
#[derive(Debug)]
#[cfg_attr(test, derive(Clone, PartialEq))]
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
#[async_trait]
pub trait Plugin<T>
where
    Self: Sync + Send + Debug,
    T: TryInto<PluginIO> + TryFrom<PluginIO>,
    T: Sync + Send,
{
    async fn run(self: &Self, t: T) -> Fallible<T>;

    fn get_name(self: &Self) -> &'static str;
}

/// Trait to be implemented by internal plugins with their native IO type
#[async_trait]
pub trait InternalPlugin {
    const PLUGIN_NAME: &'static str;

    async fn run_internal(self: &Self, input: InternalIO) -> Fallible<InternalIO>;

    fn get_name(self: &Self) -> &'static str {
        Self::PLUGIN_NAME
    }
}

/// Trait to be implemented by external plugins with its native IO type
///
/// There's a gotcha in that this type can't be used to access the information
/// directly as it's merely bytes.
#[async_trait]
pub trait ExternalPlugin
where
    Self: Debug,
{
    const PLUGIN_NAME: &'static str;

    async fn run_external(self: &Self, input: ExternalIO) -> Fallible<ExternalIO>;

    fn get_name(self: &Self) -> &'static str {
        Self::PLUGIN_NAME
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
    type Error = Error;

    fn try_from(external_io: ExternalIO) -> Fallible<Self> {
        protobuf::Message::parse_from_bytes(&external_io.bytes)
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
        protobuf::Message::parse_from_bytes(&external_io.bytes)
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
#[async_trait]
impl<T> Plugin<PluginIO> for InternalPluginWrapper<T>
where
    T: InternalPlugin,
    T: Sync + Send + Debug,
{
    async fn run(self: &Self, plugin_io: PluginIO) -> Fallible<PluginIO> {
        let internal_io: InternalIO = plugin_io.try_into()?;

        Ok(self.0.run_internal(internal_io).await?.into())
    }

    fn get_name(&self) -> &'static str {
        <T as InternalPlugin>::PLUGIN_NAME
    }
}

/// This implementation allows the process function to run ipmlementors of
/// ExternalPlugin
#[async_trait]
impl<T> Plugin<PluginIO> for ExternalPluginWrapper<T>
where
    T: ExternalPlugin,
    T: Sync + Send + Debug,
{
    async fn run(self: &Self, plugin_io: PluginIO) -> Fallible<PluginIO> {
        let external_io: ExternalIO = plugin_io.try_into()?;

        Ok(self.0.run_external(external_io).await?.into())
    }

    fn get_name(&self) -> &'static str {
        <T as ExternalPlugin>::PLUGIN_NAME
    }
}

/// Processes all given Plugins sequentially.
///
/// This function automatically converts between the different IO representations
/// if necessary.
pub async fn process<T>(plugins: T, initial_io: PluginIO) -> Fallible<InternalIO>
where
    T: Iterator<Item = &'static BoxedPlugin>,
    T: Sync + Send,
    T: 'static,
{
    let mut io = initial_io;

    let span = get_tracer().start("plugins");
    let _active_span = mark_span_as_active(span);

    for next_plugin in plugins {
        let plugin_name = next_plugin.get_name();
        log::trace!("Running next plugin '{}'", plugin_name);

        let plugin_span = get_tracer().start(plugin_name);
        let _active_plugin_span = mark_span_as_active(plugin_span);
        let cx = ot_context::current();
        io = next_plugin.run(io).with_context(cx).await?;
    }

    io.try_into()
}

/// Wrapper around `process` with an optional timeout.
///
/// It creates a new runtime per call which is moved to a new thread.
/// This has the desired effect of timing out even if the runtime is blocked by a task.
/// It has the sideeffect that these threads are unrecoverably leaked.
///
/// These two strategies are used in combination to implement the timeout:
/// 1. Use the runtime's internal timeout implementation which works for proper async tasks.
/// 2. Spawn a separate sleeper thread to enforce a deadline of 101% of the timeout
///    in case the async timeout is not effective.
pub fn process_blocking<T>(
    plugins: T,
    initial_io: PluginIO,
    timeout: Option<std::time::Duration>,
) -> Fallible<InternalIO>
where
    T: Iterator<Item = &'static BoxedPlugin>,
    T: Sync + Send,
    T: 'static,
{
    let runtime = tokio::runtime::Runtime::new()?;

    let timeout = match timeout {
        None => return runtime.block_on(process(plugins, initial_io)),
        Some(timeout) => timeout,
    };
    let deadline = timeout + (timeout / 100);

    let (tx, rx) = std::sync::mpsc::channel::<Fallible<InternalIO>>();

    {
        let tx = tx.clone();

        std::thread::spawn(move || {
            let io_future =
                async { tokio::time::timeout(timeout, process(plugins, initial_io)).await };
            let io_result = runtime
                .block_on(io_future)
                .context(format!(
                    "Processing all plugins with a timeout of {:?}",
                    timeout
                ))
                .map_err(Error::from)
                .unwrap_or_else(Err);

            // This may fail if it's attempted after the timeout is exceeded.
            let _ = tx.send(io_result);
        });
    };

    std::thread::spawn(move || {
        std::thread::sleep(deadline);

        // This may fail if it's attempted after processing is finished.
        let _ = tx.send(Err(format_err!("Exceeded timeout of {:?}", &timeout)));
    });

    rx.recv()?
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::generate_graph;
    use futures::lock::Mutex as FuturesMutex;
    use lazy_static::lazy_static;
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

    #[derive(custom_debug_derive::Debug)]
    #[allow(clippy::type_complexity)]
    struct TestInternalPlugin {
        counter: AtomicUsize,
        dict: Arc<FuturesMutex<HashMap<usize, bool>>>,
        #[debug(skip)]
        inner_fn: Option<Arc<dyn Fn() -> Fallible<()> + Sync + Send>>,
    }

    #[async_trait]
    impl InternalPlugin for TestInternalPlugin {
        const PLUGIN_NAME: &'static str = "test_internal_plugin";

        async fn run_internal(self: &Self, mut io: InternalIO) -> Fallible<InternalIO> {
            if let Some(inner_fn) = &self.inner_fn {
                inner_fn()?;
            }

            self.counter.fetch_add(1, Ordering::SeqCst);
            let counter = self.counter.load(Ordering::SeqCst);

            let mut dict_guard = self.dict.lock().await;

            (*dict_guard).insert(counter, true);

            io.parameters
                .insert("COUNTER".to_string(), format!("{}", counter));

            Ok(io)
        }
    }

    #[derive(Debug)]
    struct TestExternalPlugin {}
    #[async_trait]
    impl ExternalPlugin for TestExternalPlugin {
        const PLUGIN_NAME: &'static str = "test_internal_plugin";

        async fn run_external(self: &Self, io: ExternalIO) -> Fallible<ExternalIO> {
            Ok(io)
        }
    }

    #[test]
    fn process_plugins_roundtrip_external_internal() -> Fallible<()> {
        let runtime = commons::testing::init_runtime()?;

        lazy_static! {
            static ref PLUGINS: Vec<BoxedPlugin> = new_plugins!(
                ExternalPluginWrapper(TestExternalPlugin {}),
                InternalPluginWrapper(TestInternalPlugin {
                    counter: Default::default(),
                    dict: Arc::new(FuturesMutex::new(Default::default())),
                    inner_fn: None,
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

        let plugins_future = super::process(
            PLUGINS.iter(),
            PluginIO::InternalIO(initial_internalio.clone()),
        );

        let result_internalio: InternalIO = runtime.block_on(plugins_future)?;

        assert_eq!(expected_internalio, result_internalio);

        Ok(())
    }

    #[test]
    fn process_plugins_loop() -> Fallible<()> {
        let runtime = commons::testing::init_runtime()?;

        lazy_static! {
            static ref PLUGINS: Vec<BoxedPlugin> = new_plugins!(
                ExternalPluginWrapper(TestExternalPlugin {}),
                InternalPluginWrapper(TestInternalPlugin {
                    counter: Default::default(),
                    dict: Arc::new(FuturesMutex::new(Default::default())),
                    inner_fn: None,
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

            let plugins_future = process(
                PLUGINS.iter(),
                PluginIO::InternalIO(initial_internalio.clone()),
            );

            let result_internalio: InternalIO = runtime.block_on(plugins_future)?;

            assert_eq!(expected_internalio, result_internalio);
        }

        Ok(())
    }

    #[test]
    fn process_blocking_succeeds() -> Fallible<()> {
        lazy_static! {
            static ref PLUGIN_DELAY: std::time::Duration = std::time::Duration::from_secs(1);
            static ref PLUGINS: Vec<BoxedPlugin> =
                new_plugins!(InternalPluginWrapper(TestInternalPlugin {
                    counter: Default::default(),
                    dict: Arc::new(FuturesMutex::new(Default::default())),
                    inner_fn: Some(Arc::new(|| {
                        std::thread::sleep(*PLUGIN_DELAY);
                        Ok(())
                    })),
                }));
        }

        let initial_internalio = InternalIO {
            graph: Default::default(),
            parameters: Default::default(),
        };

        let timeout = *PLUGIN_DELAY * 2;
        let before_process = std::time::Instant::now();
        let result_internalio = super::process_blocking(
            PLUGINS.iter(),
            PluginIO::InternalIO(initial_internalio),
            Some(timeout),
        );
        let process_duration = before_process.elapsed();

        assert!(
            process_duration < timeout,
            "took {:?} despite timeout of {:?}",
            process_duration,
            timeout,
        );

        assert!(
            result_internalio.is_ok(),
            "Expected Ok, got {:?}",
            result_internalio
        );

        Ok(())
    }

    #[test]
    fn process_blocking_times_out() -> Fallible<()> {
        lazy_static! {
            static ref PLUGIN_DELAY: std::time::Duration = std::time::Duration::from_secs(100);
            static ref PLUGINS: Vec<BoxedPlugin> =
                new_plugins!(InternalPluginWrapper(TestInternalPlugin {
                    counter: Default::default(),
                    dict: Arc::new(FuturesMutex::new(Default::default())),
                    inner_fn: Some(Arc::new(|| {
                        std::thread::sleep(*PLUGIN_DELAY);
                        Ok(())
                    })),
                }));
        }

        let initial_internalio = InternalIO {
            graph: Default::default(),
            parameters: Default::default(),
        };

        // timeout hit
        let timeout = *PLUGIN_DELAY / 100;
        for _ in 0..10 {
            let before_process = std::time::Instant::now();
            let result_internalio = super::process_blocking(
                PLUGINS.iter(),
                PluginIO::InternalIO(initial_internalio.clone()),
                Some(timeout),
            );
            let process_duration = before_process.elapsed();

            assert!(
                process_duration < *PLUGIN_DELAY,
                "took {:?} despite timeout of {:?}",
                process_duration,
                timeout,
            );

            assert!(
                result_internalio.is_err(),
                "Expected error, got {:?}",
                result_internalio
            );
        }

        Ok(())
    }

    #[test]
    fn plugin_names() -> Fallible<()> {
        lazy_static! {
            static ref PLUGINS: Vec<BoxedPlugin> = new_plugins!(
                ExternalPluginWrapper(TestExternalPlugin {}),
                InternalPluginWrapper(TestInternalPlugin {
                    counter: Default::default(),
                    dict: Arc::new(FuturesMutex::new(Default::default())),
                    inner_fn: None,
                }),
                ExternalPluginWrapper(TestExternalPlugin {})
            );
        }

        assert_eq!(PLUGINS[0].get_name(), TestExternalPlugin::PLUGIN_NAME);
        assert_eq!(PLUGINS[1].get_name(), TestInternalPlugin::PLUGIN_NAME);

        Ok(())
    }
}
