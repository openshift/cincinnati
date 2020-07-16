#![cfg(windows)]
#![warn(missing_docs)]
/*!
 This crate provides a wrapper for console-related functions in the Windows API.

 [![Crate](https://img.shields.io/crates/v/winconsole.svg)](https://crates.io/crates/winconsole) ![License](https://img.shields.io/crates/l/winconsole.svg)

 # Usage
 Add the following to `Cargo.toml`:
 ```toml
 [dependencies]
 winconsole = "0.10"
 ```
 Then, add the following to your code:
 ```rust
 extern crate winconsole;
 ```

 ---

 There are a few optional features:
 * `input` - Includes input-related functions.
 * `serde` - Support for [serde](https://serde.rs/).
 * `window` - Includes window-related functions.

 These features must be added to `Cargo.toml`:
 ```toml
 [dependencies.winconsole]
 version = "0.10"
 features = ["input", "serde", "window"]
 ```
 */
extern crate cgmath;
#[macro_use]
extern crate lazy_static;
extern crate rgb;
#[cfg(feature = "serde")] #[macro_use]
extern crate serde;
extern crate winapi;

#[macro_use]
mod macros;

/// Contains console-related functions, structs, and enums.
pub mod console;
/// Contains various error types.
pub mod errors;
/// Contains input-related functions, structs, and enums.
#[cfg(feature = "input")]
pub mod input;
/// Contains window-related functions, structs, and enums.
#[cfg(feature = "window")]
pub mod window;
