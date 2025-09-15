# dkregistry


[![Build Status](https://travis-ci.org/camallo/dkregistry-rs.svg?branch=master)](https://travis-ci.org/camallo/dkregistry-rs)
[![LoC](https://tokei.rs/b1/github/camallo/dkregistry-rs?category=code)](https://github.com/camallo/dkregistry-rs)
[![Documentation](https://docs.rs/dkregistry/badge.svg)](https://docs.rs/dkregistry)

A pure-Rust asynchronous library for Docker Registry API.

`dkregistry` provides support for asynchronous interaction with container registries
conformant to the [Docker Registry HTTP API V2][registry-v2] specification.

[registry-v2]: https://docs.docker.com/registry/spec/api/

## Configurable features

The following is a list of [Cargo features][cargo-features] that consumers can enable or disable:

 * **reqwest-default-tls** *(enabled by default)*: provides TLS support via [system-specific library][native-tls] (OpenSSL on Linux)
 * **reqwest-rustls**: provides TLS support via the [rustls][rustls] library

[rustls]: https://docs.rs/rustls
[native-tls]: https://docs.rs/native-tls
[cargo-features]: https://doc.rust-lang.org/stable/cargo/reference/manifest.html#the-features-section

## Testing

### Integration tests

This library relies on the [mockito][mockito-gh] framework for mocking.

Mock tests can be enabled via the `test-mock` feature:
```
cargo test --features test-mock
```

[mockito-gh]: https://github.com/lipanski/mockito

### Interoperability tests

This library includes additional interoperability tests against some of the most common registries.

Those tests are not run by default as they require network access and registry credentials.

They are gated behind a dedicated "test-net" feature and can be run as:
```
cargo test --features test-net
```

Credentials for those registries must be provided via environmental flags.

## License

Licensed under either of

 * Apache License, Version 2.0
   ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
 * MIT license
   ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.

## Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in the work by you, as defined in the Apache-2.0 license, shall be
dual licensed as above, without any additional terms or conditions.
