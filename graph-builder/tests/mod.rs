#[cfg(feature = "test-net")]
#[macro_use]
extern crate failure;

#[cfg(feature = "test-net")]
mod net;

#[cfg(feature = "test-e2e")]
mod e2e;
