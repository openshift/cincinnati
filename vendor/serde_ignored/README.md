# Serde ignored

[<img alt="github" src="https://img.shields.io/badge/github-dtolnay/serde--ignored-8da0cb?style=for-the-badge&labelColor=555555&logo=github" height="20">](https://github.com/dtolnay/serde-ignored)
[<img alt="crates.io" src="https://img.shields.io/crates/v/serde_ignored.svg?style=for-the-badge&color=fc8d62&logo=rust" height="20">](https://crates.io/crates/serde_ignored)
[<img alt="docs.rs" src="https://img.shields.io/badge/docs.rs-serde__ignored-66c2a5?style=for-the-badge&labelColor=555555&logoColor=white&logo=data:image/svg+xml;base64,PHN2ZyByb2xlPSJpbWciIHhtbG5zPSJodHRwOi8vd3d3LnczLm9yZy8yMDAwL3N2ZyIgdmlld0JveD0iMCAwIDUxMiA1MTIiPjxwYXRoIGZpbGw9IiNmNWY1ZjUiIGQ9Ik00ODguNiAyNTAuMkwzOTIgMjE0VjEwNS41YzAtMTUtOS4zLTI4LjQtMjMuNC0zMy43bC0xMDAtMzcuNWMtOC4xLTMuMS0xNy4xLTMuMS0yNS4zIDBsLTEwMCAzNy41Yy0xNC4xIDUuMy0yMy40IDE4LjctMjMuNCAzMy43VjIxNGwtOTYuNiAzNi4yQzkuMyAyNTUuNSAwIDI2OC45IDAgMjgzLjlWMzk0YzAgMTMuNiA3LjcgMjYuMSAxOS45IDMyLjJsMTAwIDUwYzEwLjEgNS4xIDIyLjEgNS4xIDMyLjIgMGwxMDMuOS01MiAxMDMuOSA1MmMxMC4xIDUuMSAyMi4xIDUuMSAzMi4yIDBsMTAwLTUwYzEyLjItNi4xIDE5LjktMTguNiAxOS45LTMyLjJWMjgzLjljMC0xNS05LjMtMjguNC0yMy40LTMzLjd6TTM1OCAyMTQuOGwtODUgMzEuOXYtNjguMmw4NS0zN3Y3My4zek0xNTQgMTA0LjFsMTAyLTM4LjIgMTAyIDM4LjJ2LjZsLTEwMiA0MS40LTEwMi00MS40di0uNnptODQgMjkxLjFsLTg1IDQyLjV2LTc5LjFsODUtMzguOHY3NS40em0wLTExMmwtMTAyIDQxLjQtMTAyLTQxLjR2LS42bDEwMi0zOC4yIDEwMiAzOC4ydi42em0yNDAgMTEybC04NSA0Mi41di03OS4xbDg1LTM4Ljh2NzUuNHptMC0xMTJsLTEwMiA0MS40LTEwMi00MS40di0uNmwxMDItMzguMiAxMDIgMzguMnYuNnoiPjwvcGF0aD48L3N2Zz4K" height="20">](https://docs.rs/serde_ignored)
[<img alt="build status" src="https://img.shields.io/github/workflow/status/dtolnay/serde-ignored/CI/master?style=for-the-badge" height="20">](https://github.com/dtolnay/serde-ignored/actions?query=branch%3Amaster)

Find out about keys that are ignored when deserializing data. This crate
provides a wrapper that works with any existing Serde `Deserializer` and invokes
a callback on every ignored field.

You can use this to warn users about extraneous keys in a config file, for
example.

Note that if you want unrecognized fields to be an error, consider using the
`#[serde(deny_unknown_fields)]` [attribute] instead.

[attribute]: https://serde.rs/attributes.html

```toml
[dependencies]
serde = "1.0"
serde_ignored = "0.1"
```

```rust
use serde::Deserialize;
use std::collections::{BTreeSet as Set, BTreeMap as Map};

#[derive(Debug, PartialEq, Deserialize)]
struct Package {
    name: String,
    dependencies: Map<String, Dependency>,
}

#[derive(Debug, PartialEq, Deserialize)]
struct Dependency {
    version: String,
}

fn main() {
    let j = r#"{
        "name": "demo",
        "dependencies": {
            "serde": {
                "version": "1.0",
                "typo1": ""
            }
        },
        "typo2": {
            "inner": ""
        },
        "typo3": {}
    }"#;

    // Some Deserializer.
    let jd = &mut serde_json::Deserializer::from_str(j);

    // We will build a set of paths to the unused elements.
    let mut unused = Set::new();

    let p: Package = serde_ignored::deserialize(jd, |path| {
        unused.insert(path.to_string());
    }).unwrap();

    // Deserialized as normal.
    println!("{:?}", p);

    // There were three ignored keys.
    let mut expected = Set::new();
    expected.insert("dependencies.serde.typo1".to_owned());
    expected.insert("typo2".to_owned());
    expected.insert("typo3".to_owned());
    assert_eq!(unused, expected);
}
```

<br>

#### License

<sup>
Licensed under either of <a href="LICENSE-APACHE">Apache License, Version
2.0</a> or <a href="LICENSE-MIT">MIT license</a> at your option.
</sup>

<br>

<sub>
Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in this crate by you, as defined in the Apache-2.0 license, shall
be dual licensed as above, without any additional terms or conditions.
</sub>
