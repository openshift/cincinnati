# actix-derive [![Build Status](https://travis-ci.org/actix/actix-derive.svg?branch=master)](https://travis-ci.org/actix/actix-derive) [![crates.io](http://meritbadge.herokuapp.com/actix-derive)](https://crates.io/crates/actix-derive)

Actix is a rust actor framework.

* [API Documentation (Development)](http://actix.github.io/actix/actix/)
* [API Documentation (Releases)](https://docs.rs/actix/)
* Cargo package: [actix](https://crates.io/crates/actix)

---

Actix is licensed under the [Apache-2.0 license](http://opensource.org/licenses/APACHE-2.0).

## Features

* `actix-derive` adds support for Rust Custom Derive / Macros 1.1 to `actix`

## Usage

```rust
#[macro_use] extern crate actix_derive;

use std::io::Error;

#[derive(Message)]
#[rtype(result="Result<usize, Error>")]
struct Sum(usize, usize);

#[derive(MessageResponse)]
struct Added(usize);

fn main() {}
```

This code expands into following code:

```rust
extern crate actix;
use std::io::Error;
use actix::dev::MessageResponse;
use actix::dev::ResponseChannel;
use actix::Message;

struct Sum(usize, Error);

impl Message for Sum {
    type Result = Result<usize, Error>;
}

struct Added(usize);

impl<A, M> MessageResponse<A, M> for Added
where
  A: Actor,
  M: Message<Result = Added>
{
    fn handle<R: ResponseChannel<M>>(self, _: &mut A::Context, tx: Option<R>) {
        if let Some(tx) = tx {
            tx.send(self);
        }
    }
}

fn main() {}
```

## License

This project is licensed under either of

 * Apache License, Version 2.0, ([LICENSE-APACHE](LICENSE-APACHE) or
   http://www.apache.org/licenses/LICENSE-2.0)
 * MIT license ([LICENSE-MIT](LICENSE-MIT) or
   http://opensource.org/licenses/MIT)

at your option.
