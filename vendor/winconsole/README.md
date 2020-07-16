# winconsole
This crate provides a wrapper for console-related functions in the Windows API.

[![Crate](https://img.shields.io/crates/v/winconsole.svg)](https://crates.io/crates/winconsole) [![Documentation](https://docs.rs/winconsole/badge.svg)](https://omarkmu.github.io/docs/winconsole/) ![License](https://img.shields.io/crates/l/winconsole.svg)

## Usage
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