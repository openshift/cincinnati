`iter-read`
===========

[![Build status](https://img.shields.io/github/actions/workflow/status/birkenfeld/iter-read/main.yml?branch=master&logo=github&style=)](https://github.com/birkenfeld/iter-read/actions)
[![Latest Version](https://img.shields.io/crates/v/iter-read.svg)](https://crates.io/crates/iter-read)

[Documentation](https://docs.rs/iter-read)

This crate is a small library that provides a type that implements
`std::io::Read` for iterators over bytes (`u8`) and sequences of it, and also
`Result<u8, E>`, `Result<Vec<u8>, E>` etc.

Installation
============

This crate works with Cargo and can be found on
[crates.io](https://crates.io/crates/iter-read) with a `Cargo.toml` like:

```toml
[dependencies]
iter-read = "1.0"
```

Requirements
============

Minimum supported Rust version is 1.58.0.  No other dependencies.

Usage
=====

A simple example:

```rust
use std::io::Read;
use iter_read::IterRead;
let source = vec![1, 2, 7, 42, 123];
let mut reader = IterRead::new(source.iter());
let mut buf = vec![0; 3];
reader.read_exact(&mut buf).unwrap();
assert_eq!(buf, b"\x01\x02\x07");
```
