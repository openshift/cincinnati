#[macro_use]
extern crate commons;
#[macro_use]
extern crate lazy_static;
#[macro_use]
extern crate log;
#[macro_use]
extern crate serde_derive;
#[macro_use]
extern crate smart_default;
#[macro_use]
extern crate structopt;
#[macro_use]
extern crate cincinnati;

pub mod config;
pub mod graph;
pub mod status;

#[allow(dead_code)]
/// Build info
mod built_info {
    include!(concat!(env!("OUT_DIR"), "/built.rs"));
}
