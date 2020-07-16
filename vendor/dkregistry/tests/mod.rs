#[cfg(feature = "test-net")]
mod net;

#[cfg(feature = "test-net")]
#[macro_use]
extern crate error_chain;

#[cfg(feature = "test-mock")]
mod mock;
