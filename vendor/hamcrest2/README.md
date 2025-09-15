[![Build Status](https://travis-ci.org/Valloric/hamcrest2-rust.svg?branch=master)](https://travis-ci.org/Valloric/hamcrest2-rust)

# Hamcrest2

A port of [Hamcrest](http://hamcrest.org/) to [Rust](http://rust-lang.org).
Fork of original hamcrest-rust (which is unmaintained) with extra matchers,
better docs, support for Rust 2018 edition etc.

## Installing

To use Hamcrest, add this to your `Cargo.toml`:

```
[dev-dependencies]
hamcrest2 = "*"
```

After a quick `cargo build`, you should be good to go!

## Usage

Hamcrest2 supports a number of matchers. The easiest way is to just `use` them all like this:

```rust
use hamcrest2::prelude::*;
```

If you want to be more selective make sure that you also import the `HamcrestMatcher` trait.

## General Matchers

### eq, not

```rust
assert_that!(1, eq(1));  // also equal_to()
assert_that!(1, not(eq(2)));
```

### compared_to

```rust
assert_that!(1, lt(2));   // also less_than()
assert_that!(1, leq(1));  // also less_than_or_equal_to()
assert_that!(2, gt(1));   // also greater_than()
assert_that!(2, geq(2));  // also greater_than_or_equal_to()
```

### type_of

```rust
assert_that!(123usize, type_of::<usize>());
assert_that!("test", type_of::<&str>());
```

### matches_regex

```rust
assert_that!("1234", matches_regex(r"\d"));
assert_that!("abc", does_not(match_regex(r"\d")));
```

## Numerical Matchers

### close_to

```rust
assert_that!(1e-40f32, close_to(0.0, 0.01));
assert_that!(1e-40f32, not(close_to(0.0, 0.000001)));
```

## Filesystem Matchers

### path_exists, file_exists, dir_exists

```rust
let path = Path::new("./README.md");
assert_that!(path, path_exists());
assert_that!(path, file_exists());
assert_that!(path, not(dir_exists()));
```

## Option and Result

### has

```rust
let var: Option<i8> = Some(5);
assert_that!(var, has(5));

let var: Result<i8, String> = Ok(5);
assert_that!(var, has(5));
```

### ok
```rust
let var: Result<i8, String> = Ok(5);
assert_that!(var, ok());
assert_that!(&var, ok());

assert_that!(Ok(5), ok::<i8, String>());
assert_that!(&Ok(5), ok::<i8, String>());

let var: Result<i8, String> = Err("bad!".to_string());
assert_that!(var, not(ok()));
assert_that!(&var, not(ok()));
```

### err

```rust
let var: Result<i8, String> = Err("bad!".to_string());
assert_that!(var, err());
assert_that!(&var, err());

assert_that!(Err("bad!".to_string()), err::<i8, String>());
assert_that!(&Err("bad!".to_string()), err::<i8, String>());

let var: Result<i8, String> = Ok(5);
assert_that!(var, not(err()));
assert_that!(&var, not(err()));
```

### some

```rust
let var: Option<i8> = Some(5);
assert_that!(var, some());
assert_that!(&var, some());

assert_that!(Some(1), some::<u8>());
assert_that!(&Some(1), some::<u8>());

let var: Option<i8> = None;
assert_that!(var, not(some()));
assert_that!(&var, not(some()));
```

### none

```rust
let var: Option<i8> = None;
assert_that!(var, none());
assert_that!(&var, none());

assert_that!(None, none::<u8>());
assert_that!(&None, none::<u8>());
assert_that!(Some(1), not(none::<u8>()));
assert_that!(&Some(1), not(none::<u8>()));
```

## Collection Matchers

### contains, contains exactly, contains in order

```rust
assert_that!(&vec!(1, 2, 3), contains(vec!(1, 2)));
assert_that!(&vec!(1, 2, 3), contains(1));
assert_that!(&vec!(1, 2, 3), not(contains(4i)));

assert_that!(&vec!(1, 2, 3), contains(vec!(1, 2, 3)).exactly());
assert_that!(&vec!(1, 2, 3), not(contains(vec!(1, 2)).exactly()));

assert_that!(&vec!(1, 2, 3), contains(vec!(1, 2)).in_order());
assert_that!(&vec!(1, 2, 3), not(contains(vec!(1, 3)).in_order()));
```

## len
```rust
assert_that!(&vec!(1, 2, 3), len(3));
assert_that!(&vec!(1, 2, 3), not(len(4)));
```

## empty
```rust
assert_that!(&Vec::<i32>::new(), empty());
assert_that!(&vec![1, 2, 3], not(empty()));
```

## Compound Matchers

### all

```rust
assert_that!(4, all!(lt(5), gt(3)));  // also and!()
assert_that!(
    &vec![1, 2, 3],
    all!(contains(vec![1, 2]), not(contains(vec![4])))
);
```

### any

```rust
assert_that!(4, any!(less_than(2), greater_than(3)));  // also or!()
assert_that!(
    &vec![1, 2, 3],
    any!(contains(vec![1, 2, 5]), not(contains(vec![4])))
);
```

## Misc Matchers

### is(bool)

```rust
assert_that!(true, is(true));
assert_that!(false, is(false));
```

### anything

```rust
assert_that!(42, anything());
assert_that!("test", is(anything()));
```

## License

Licensed under either of

 * Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or
   http://www.apache.org/licenses/LICENSE-2.0)
 * MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.

### Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in the
work by you, as defined in the Apache-2.0 license, shall be dual licensed as above, without any
additional terms or conditions.
