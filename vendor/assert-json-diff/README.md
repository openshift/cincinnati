[![Build Status](https://travis-ci.org/davidpdrsn/assert-json-diff.svg?branch=master)](https://travis-ci.org/davidpdrsn/assert-json-diff)

# assert-json-diff

This crate includes macros for comparing two JSON values. It is designed to give much
more helpful error messages than the standard [`assert_eq!`]. It basically does a diff of the
two objects and tells you the exact differences. This is useful when asserting that two large
JSON objects are the same.

It uses the [`serde_json::Value`] type to represent JSON.

[`serde_json::Value`]: https://docs.serde.rs/serde_json/value/enum.Value.html
[`assert_eq!`]: https://doc.rust-lang.org/std/macro.assert_eq.html

### Install

```toml
[dependencies]
assert-json-diff = "1.0.0"
```

### Partial matching

If you want to assert that one JSON value is "included" in another use
[`assert_json_include`](macro.assert_json_include.html):

```rust
#[macro_use]
extern crate assert_json_diff;
#[macro_use]
extern crate serde_json;

fn main() {
    let a = json!({
        "data": {
            "users": [
                {
                    "id": 1,
                    "country": {
                        "name": "Denmark"
                    }
                },
                {
                    "id": 24,
                    "country": {
                        "name": "Denmark"
                    }
                }
            ]
        }
    });

    let b = json!({
        "data": {
            "users": [
                {
                    "id": 1,
                    "country": {
                        "name": "Sweden"
                    }
                },
                {
                    "id": 2,
                    "country": {
                        "name": "Denmark"
                    }
                }
            ]
        }
    });

    assert_json_include!(actual: a, expected: b)
}
```

This will panic with the error message:

```
json atoms at path ".data.users[0].country.name" are not equal:
    expected:
        "Sweden"
    actual:
        "Denmark"

json atoms at path ".data.users[1].id" are not equal:
    expected:
        2
    actual:
        24
```

[`assert_json_include`](macro.assert_json_include.html) allows extra data in `actual` but not in `expected`. That is so you can verify just a part
of the JSON without having to specify the whole thing. For example this test passes:

```rust
#[macro_use]
extern crate assert_json_diff;
#[macro_use]
extern crate serde_json;

fn main() {
    assert_json_include!(
        actual: json!({
            "a": { "b": 1 },
        }),
        expected: json!({
            "a": {},
        })
    )
}
```

However `expected` cannot contain additional data so this test fails:

```rust
#[macro_use]
extern crate assert_json_diff;
#[macro_use]
extern crate serde_json;

fn main() {
    assert_json_include!(
        actual: json!({
            "a": {},
        }),
        expected: json!({
            "a": { "b": 1 },
        })
    )
}
```

That will print

```
json atom at path ".a.b" is missing from actual
```

### Exact matching

If you want to ensure two JSON values are *exactly* the same, use [`assert_json_eq`](macro.assert_json_eq.html).

```rust
#[macro_use]
extern crate assert_json_diff;
#[macro_use]
extern crate serde_json;

fn main() {
    assert_json_eq!(
        json!({ "a": { "b": 1 } }),
        json!({ "a": {} })
    )
}
```

This will panic with the error message:

```
json atom at path ".a.b" is missing from lhs
```

License: MIT
