//! Asynchronous library for the Prometheus HTTP API
//!
//! See https://github.com/prometheus/prometheus/blob/9de0ab3c8a32f8e09ab68f793dab0c76ec3e93d0/docs/querying/api.md#http-api

#[macro_use]
extern crate failure;
extern crate futures;
extern crate reqwest;
extern crate serde;
#[macro_use]
extern crate log;
pub extern crate chrono;

#[cfg(test)]
#[macro_use]
extern crate serde_json;

pub mod v1;
