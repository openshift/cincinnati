## 0.3.0 [☰](https://github.com/Valloric/hamcrest2-rust/compare/0.2.6...0.3.0)
* Full support for Rust 2018 edition. No more `#[macro_use]` or deprecation
  warnings for modern idioms. This _might_ have broken some usage of APIs that
  have been deprecated for **many** months now, thus bumping to 0.3.0. #10

## 0.2.6 [☰](https://github.com/Valloric/hamcrest2-rust/compare/0.2.4...0.2.6)

* The previous version introduced support for reference arguments and this
broke `equal_to` for slices. Thus, automatic ref argument support for
`equal_to` has been reverted.

## 0.2.4 [☰](https://github.com/Valloric/hamcrest2-rust/compare/0.2.3...0.2.4)

* `contains` matcher is now generic and supports both collection and single item
  arguments; thus, both `contains(vec![5])` and `contains(5)` work
* Almost all matchers now support reference arguments as well!

## 0.2.3 [☰](https://github.com/Valloric/hamcrest2-rust/compare/0.2.2...0.2.3)

* `contains`, `empty` and `len` matchers now work for slices, not just vectors

## 0.2.2 [☰](https://github.com/Valloric/hamcrest2-rust/compare/0.2.1...0.2.2)

* Added the `empty` matcher

## 0.2.1 [☰](https://github.com/Valloric/hamcrest2-rust/compare/0.2.0...0.2.1)

* Better message for expected output for `ok`, `err`, `some` and `none` matchers

## 0.2.0 [☰](https://github.com/Valloric/hamcrest2-rust/compare/0.1.6...0.2.0)

* Created real crate docs with doctests
* added `ok` and `err` matchers
* `of_len!` is now `len!`
* `any_of!` is now `any!`
* `all_of!` is now `all!`
* `existing_path` is now `path_exists`
* `existing_file` is now `file_exists`
* `existing_dir` is now `dir_exists`

## 0.1.6 [☰](https://github.com/Valloric/hamcrest2-rust/compare/0.1.5...0.1.6)

* Shorter names for common matchers:
    * `eq` for `equal_to`
    * `lt` for `less_than`
    * `gt` for `greater_than`
    * similarly, `geq`, `leq` etc
* Restructured examples in README to reduce verbosity
* Added `some` matcher
* Added `has` matcher (like `contains` but for `Option` and `Result`!)

## 0.1.5 [☰](https://github.com/Valloric/hamcrest2-rust/compare/0.1.4...0.1.5)

* Implemented matcher trait for boolean values, #48

## 0.1.4 [☰](https://github.com/Valloric/hamcrest2-rust/compare/0.1.3...0.1.4)

* Logical matchers `all_of`, `any_of`, comparison matchers `type_of`, `anything`, #47

## 0.1.3 [☰](https://github.com/Valloric/hamcrest2-rust/compare/0.1.2...0.1.3)

* Comparison matchers `less_than`, `less_than_or_equal_to`, `greater_than`, `greater_than_or_equal_to`. #43
* `in_order` option for `contains`. #44

## 0.1.2 [☰](https://github.com/Valloric/hamcrest2-rust/compare/0.1.1...0.1.2)

* Added the `assert_that!` macro. It produces better error messages (with correct file and line
  number).
* Deprecated the `assert_that` function.
* Improvements to `Cargo.toml` (by @killercup)

## 0.1.1 [☰](https://github.com/Valloric/hamcrest2-rust/compare/a9f18681c64e3126ef6ccbd68ec2a5b39fe5b58b...0.1.1)

* Licensing change. The crate is now dual licensed under the MIT and Apache 2 licenses.
* Adds the `prelude` submodule to simplify inclusion of all matchers.
* `matches_regex` matcher
