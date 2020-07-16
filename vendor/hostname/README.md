# hostname

[![docs](https://docs.rs/hostname/badge.svg?version=0.1.5 'docs')](https://docs.rs/hostname)

Get hostname. Compatible with windows and unix.

## [Document](https://docs.rs/hostname)

## Usage

Add dependency to Cargo.toml

```toml
[dependencies]
hostname = "^0.1"
```

In your `main.rs` or `lib.rs`:

```rust
extern crate hostname;
```

## Examples

```rust
use hostname::get_hostname;

assert!(get_hostname().is_some());
```

## License

hostname is primarily distributed under the terms of the MIT license.
See [LICENSE](LICENSE) for details.
