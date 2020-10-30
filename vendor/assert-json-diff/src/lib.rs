//! This crate includes macros for comparing two JSON values. It is designed to give much
//! more helpful error messages than the standard [`assert_eq!`]. It basically does a diff of the
//! two objects and tells you the exact differences. This is useful when asserting that two large
//! JSON objects are the same.
//!
//! It uses the [`serde_json::Value`] type to represent JSON.
//!
//! [`serde_json::Value`]: https://docs.serde.rs/serde_json/value/enum.Value.html
//! [`assert_eq!`]: https://doc.rust-lang.org/std/macro.assert_eq.html
//!
//! ## Install
//!
//! ```toml
//! [dependencies]
//! assert-json-diff = "1.0.0"
//! ```
//!
//! ## Partial matching
//!
//! If you want to assert that one JSON value is "included" in another use
//! [`assert_json_include`](macro.assert_json_include.html):
//!
//! ```should_panic
//! #[macro_use]
//! extern crate assert_json_diff;
//! #[macro_use]
//! extern crate serde_json;
//!
//! fn main() {
//!     let a = json!({
//!         "data": {
//!             "users": [
//!                 {
//!                     "id": 1,
//!                     "country": {
//!                         "name": "Denmark"
//!                     }
//!                 },
//!                 {
//!                     "id": 24,
//!                     "country": {
//!                         "name": "Denmark"
//!                     }
//!                 }
//!             ]
//!         }
//!     });
//!
//!     let b = json!({
//!         "data": {
//!             "users": [
//!                 {
//!                     "id": 1,
//!                     "country": {
//!                         "name": "Sweden"
//!                     }
//!                 },
//!                 {
//!                     "id": 2,
//!                     "country": {
//!                         "name": "Denmark"
//!                     }
//!                 }
//!             ]
//!         }
//!     });
//!
//!     assert_json_include!(actual: a, expected: b)
//! }
//! ```
//!
//! This will panic with the error message:
//!
//! ```text
//! json atoms at path ".data.users[0].country.name" are not equal:
//!     expected:
//!         "Sweden"
//!     actual:
//!         "Denmark"
//!
//! json atoms at path ".data.users[1].id" are not equal:
//!     expected:
//!         2
//!     actual:
//!         24
//! ```
//!
//! [`assert_json_include`](macro.assert_json_include.html) allows extra data in `actual` but not in `expected`. That is so you can verify just a part
//! of the JSON without having to specify the whole thing. For example this test passes:
//!
//! ```
//! #[macro_use]
//! extern crate assert_json_diff;
//! #[macro_use]
//! extern crate serde_json;
//!
//! fn main() {
//!     assert_json_include!(
//!         actual: json!({
//!             "a": { "b": 1 },
//!         }),
//!         expected: json!({
//!             "a": {},
//!         })
//!     )
//! }
//! ```
//!
//! However `expected` cannot contain additional data so this test fails:
//!
//! ```should_panic
//! #[macro_use]
//! extern crate assert_json_diff;
//! #[macro_use]
//! extern crate serde_json;
//!
//! fn main() {
//!     assert_json_include!(
//!         actual: json!({
//!             "a": {},
//!         }),
//!         expected: json!({
//!             "a": { "b": 1 },
//!         })
//!     )
//! }
//! ```
//!
//! That will print
//!
//! ```text
//! json atom at path ".a.b" is missing from actual
//! ```
//!
//! ## Exact matching
//!
//! If you want to ensure two JSON values are *exactly* the same, use [`assert_json_eq`](macro.assert_json_eq.html).
//!
//! ```rust,should_panic
//! #[macro_use]
//! extern crate assert_json_diff;
//! #[macro_use]
//! extern crate serde_json;
//!
//! fn main() {
//!     assert_json_eq!(
//!         json!({ "a": { "b": 1 } }),
//!         json!({ "a": {} })
//!     )
//! }
//! ```
//!
//! This will panic with the error message:
//!
//! ```text
//! json atom at path ".a.b" is missing from lhs
//! ```

#![deny(
    missing_docs,
    unused_imports,
    missing_debug_implementations,
    missing_copy_implementations,
    trivial_casts,
    trivial_numeric_casts,
    unsafe_code,
    unstable_features,
    unused_import_braces,
    unused_qualifications
)]
#![doc(html_root_url = "https://docs.rs/assert-json-diff/1.0.0")]

extern crate serde;
#[allow(unused_imports)]
#[macro_use]
extern crate serde_json;

use serde::{Serialize, Serializer};
use serde_json::Value;
use std::collections::HashSet;
use std::default::Default;
use std::fmt;

mod core_ext;
use core_ext::{Indent, Indexes};

/// The macro used to compare two JSON values for an inclusive match.
///
/// It allows `actual` to contain additional data. If you want an exact match use
/// [`assert_json_eq`](macro.assert_json_eq.html) instead.
///
/// See [crate documentation](index.html) for examples.
#[macro_export]
macro_rules! assert_json_include {
    (actual: $actual:expr, expected: $expected:expr) => {{
        use $crate::{Actual, Comparison, Expected};
        let actual: serde_json::Value = $actual;
        let expected: serde_json::Value = $expected;
        let comparison = Comparison::Include(Actual::new(actual), Expected::new(expected));
        if let Err(error) = $crate::assert_json_no_panic(comparison) {
            panic!("\n\n{}\n\n", error);
        }
    }};
    (actual: $actual:expr, expected: $expected:expr,) => {{
        $crate::assert_json_include!(actual: $actual, expected: $expected)
    }};
    (expected: $expected:expr, actual: $actual:expr) => {{
        $crate::assert_json_include!(actual: $actual, expected: $expected)
    }};
    (expected: $expected:expr, actual: $actual:expr,) => {{
        $crate::assert_json_include!(actual: $actual, expected: $expected)
    }};
}

/// The macro used to compare two JSON values for an exact match.
///
/// If you want an inclusive match use [`assert_json_include`](macro.assert_json_include.html) instead.
///
/// See [crate documentation](index.html) for examples.
#[macro_export]
macro_rules! assert_json_eq {
    ($lhs:expr, $rhs:expr) => {{
        use $crate::{Actual, Comparison, Expected};
        let lhs: serde_json::Value = $lhs;
        let rhs: serde_json::Value = $rhs;
        let comparison = Comparison::Exact(lhs, rhs);
        if let Err(error) = $crate::assert_json_no_panic(comparison) {
            panic!("\n\n{}\n\n", error);
        }
    }};
    ($lhs:expr, $rhs:expr,) => {{
        $crate::assert_json_eq!($lhs, $rhs)
    }};
}

/// Perform the matching and return the error text rather than panicing.
///
/// The [macros](index.html#macros) call this function and panics if the result is an `Err(_)`
#[doc(hidden)]
pub fn assert_json_no_panic(comparison: Comparison) -> Result<(), String> {
    let mut errors = MatchErrors::default();
    match comparison {
        Comparison::Include(actual, expected) => {
            partial_match_at_path(actual, expected, Path::Root, &mut errors);
        }

        Comparison::Exact(lhs, rhs) => {
            exact_match_at_path(lhs, rhs, Path::Root, &mut errors);
        }
    }
    errors.to_output()
}

/// The type of comparison you want to make.
///
/// The [macros](index.html#macros) use this type, but you shouldn't have to use it explicitly.
#[doc(hidden)]
#[derive(Debug)]
pub enum Comparison {
    /// An inclusive match. Allows additional data in actual, but not in expected.
    Include(Actual, Expected),

    /// An exact match.
    Exact(Value, Value),
}

/// A wrapper for the actual value in a match.
///
/// The purpose of this wrapper is to not mix up the actual and expected values.
#[doc(hidden)]
#[derive(Clone, Debug)]
pub struct Actual(Value);

impl std::ops::Deref for Actual {
    type Target = Value;
    fn deref(&self) -> &Value {
        &self.0
    }
}

impl Actual {
    /// Create a new value from a [`serde_json::Value`].
    ///
    /// [`serde_json::Value`]: https://docs.serde.rs/serde_json/value/enum.Value.html
    pub fn new(value: Value) -> Self {
        Actual(value)
    }
}

impl Serialize for Actual {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        <Value>::serialize(self, serializer)
    }
}

impl From<Value> for Actual {
    fn from(v: Value) -> Actual {
        Actual(v)
    }
}

/// A wrapper for the expected value in a match.
///
/// The purpose of this wrapper is to not mix up the actual and expected values.
#[doc(hidden)]
#[derive(Clone, Debug)]
pub struct Expected(Value);

impl Expected {
    /// Create a new value from a [`serde_json::Value`].
    ///
    /// [`serde_json::Value`]: https://docs.serde.rs/serde_json/value/enum.Value.html
    pub fn new(value: Value) -> Self {
        Expected(value)
    }
}

impl std::ops::Deref for Expected {
    type Target = Value;
    fn deref(&self) -> &Value {
        &self.0
    }
}

impl Serialize for Expected {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        <Value>::serialize(self, serializer)
    }
}

impl From<Value> for Expected {
    fn from(v: Value) -> Expected {
        Expected(v)
    }
}

enum Either<A, B> {
    Left(A),
    Right(B),
}

fn partial_match_at_path(actual: Actual, expected: Expected, path: Path, errors: &mut MatchErrors) {
    if let Some(expected) = expected.as_object() {
        let keys = expected.keys();
        match_with_keys(keys, &actual, expected, path, errors);
    } else if let Some(expected) = expected.as_array() {
        let keys = if expected.is_empty() {
            vec![]
        } else {
            expected.indexes()
        };

        match_with_keys(keys.iter(), &actual, expected, path, errors);
    } else {
        if expected.0 != actual.0 {
            errors.push(ErrorType::NotEq(
                Either::Left((actual.clone(), expected.clone())),
                path,
            ));
        }
    }
}

fn match_with_keys<
    Key: Copy,
    Keys: Iterator<Item = Key>,
    Path: Dot<Key>,
    ActualCollection: Collection<Key, Item = ActualValue>,
    ActualValue: Clone + Into<Actual>,
    ExpectedCollection: Collection<Key, Item = ExpectedValue>,
    ExpectedValue: Clone + Into<Expected>,
>(
    keys: Keys,
    actual: &ActualCollection,
    expected: &ExpectedCollection,
    path: Path,
    errors: &mut MatchErrors,
) {
    for key in keys {
        match (expected.get(key), actual.get(key)) {
            (Some(expected), Some(actual)) => {
                partial_match_at_path(
                    actual.clone().into(),
                    expected.clone().into(),
                    path.dot(key),
                    errors,
                );
            }

            (Some(_), None) => {
                errors.push(ErrorType::MissingPath(Either::Left(path.dot(key))));
            }

            (None, _) => unreachable!(),
        }
    }
}

fn exact_match_at_path(lhs: Value, rhs: Value, path: Path, errors: &mut MatchErrors) {
    if let (Some(lhs), Some(rhs)) = (lhs.as_object(), rhs.as_object()) {
        let keys = lhs
            .keys()
            .chain(rhs.keys())
            .map(|s| s.to_string())
            .collect::<HashSet<String>>();

        exact_match_with_keys(keys.iter(), lhs, rhs, path, errors);
    } else if let (Some(lhs), Some(rhs)) = (lhs.as_array(), rhs.as_array()) {
        let lhs_keys = lhs.indexes();
        let rhs_keys = rhs.indexes();
        let keys = lhs_keys
            .iter()
            .chain(rhs_keys.iter())
            .map(|s| s.clone())
            .collect::<HashSet<usize>>();

        exact_match_with_keys(keys.iter(), lhs, rhs, path, errors);
    } else {
        if lhs != rhs {
            errors.push(ErrorType::NotEq(
                Either::Right((lhs.clone(), rhs.clone())),
                path,
            ));
        }
    }
}

fn exact_match_with_keys<
    Key: Copy,
    Keys: Iterator<Item = Key>,
    Path: Dot<Key>,
    ValueCollection: Collection<Key, Item = Value>,
>(
    keys: Keys,
    lhs: &ValueCollection,
    rhs: &ValueCollection,
    path: Path,
    errors: &mut MatchErrors,
) {
    for key in keys {
        match (lhs.get(key), rhs.get(key)) {
            (Some(lhs), Some(rhs)) => {
                exact_match_at_path(
                    lhs.clone().into(),
                    rhs.clone().into(),
                    path.dot(key),
                    errors,
                );
            }

            (Some(_), None) => {
                errors.push(ErrorType::MissingPath(Either::Right((
                    path.dot(key),
                    SideWithoutPath::Rhs,
                ))));
            }

            (None, Some(_)) => {
                errors.push(ErrorType::MissingPath(Either::Right((
                    path.dot(key),
                    SideWithoutPath::Lhs,
                ))));
            }

            (None, None) => unreachable!(),
        }
    }
}

trait Collection<Idx> {
    type Item;
    fn get(&self, index: Idx) -> Option<&Self::Item>;
}

impl<'a> Collection<&'a String> for serde_json::Map<String, Value> {
    type Item = Value;

    fn get(&self, index: &'a String) -> Option<&Self::Item> {
        self.get(index)
    }
}

impl<'a> Collection<&'a usize> for Vec<Value> {
    type Item = Value;

    fn get(&self, index: &'a usize) -> Option<&Self::Item> {
        <[Value]>::get(self, index.clone())
    }
}

impl<'a> Collection<&'a String> for Actual {
    type Item = Value;

    fn get(&self, index: &'a String) -> Option<&Self::Item> {
        <Value>::get(self, index.clone())
    }
}

impl<'a> Collection<&'a usize> for Actual {
    type Item = Value;

    fn get(&self, index: &'a usize) -> Option<&Self::Item> {
        <Value>::get(self, index.clone())
    }
}

#[derive(Clone)]
enum Path {
    Root,
    Trail(Vec<PathComp>),
}

impl fmt::Display for Path {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Path::Root => write!(f, "(root)"),
            Path::Trail(trail) => write!(
                f,
                "{}",
                trail
                    .iter()
                    .map(|comp| comp.to_string())
                    .collect::<Vec<_>>()
                    .join("")
            ),
        }
    }
}

impl Path {
    fn extend(&self, next: PathComp) -> Path {
        match self {
            Path::Root => Path::Trail(vec![next]),
            Path::Trail(trail) => {
                let mut trail = trail.clone();
                trail.push(next);
                Path::Trail(trail)
            }
        }
    }
}

trait Dot<T> {
    fn dot(&self, next: T) -> Path;
}

impl<'a> Dot<&'a String> for Path {
    fn dot(&self, next: &'a String) -> Path {
        let comp = PathComp::String(next.to_string());
        self.extend(comp)
    }
}

impl<'a> Dot<&'a str> for Path {
    fn dot(&self, next: &'a str) -> Path {
        let comp = PathComp::String(next.to_string());
        self.extend(comp)
    }
}

impl Dot<usize> for Path {
    fn dot(&self, next: usize) -> Path {
        let comp = PathComp::Index(next);
        self.extend(comp)
    }
}

impl<'a> Dot<&'a usize> for Path {
    fn dot(&self, next: &'a usize) -> Path {
        let comp = PathComp::Index(next.clone());
        self.extend(comp)
    }
}

#[derive(Clone)]
enum PathComp {
    String(String),
    Index(usize),
}

impl fmt::Display for PathComp {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            PathComp::String(s) => write!(f, ".{}", s),
            PathComp::Index(i) => write!(f, "[{}]", i),
        }
    }
}

struct MatchErrors {
    errors: Vec<ErrorType>,
}

impl Default for MatchErrors {
    fn default() -> Self {
        MatchErrors { errors: vec![] }
    }
}

impl MatchErrors {
    fn to_output(self) -> Result<(), String> {
        if self.errors.is_empty() {
            Ok(())
        } else {
            let messages = self
                .errors
                .iter()
                .map(|error| match error {
                    ErrorType::NotEq(Either::Left((actual, expected)), path) => format!(
                        r#"json atoms at path "{}" are not equal:
    expected:
{}
    actual:
{}"#,
                        path,
                        serde_json::to_string_pretty(expected)
                            .expect("failed to pretty print JSON")
                            .indent(8),
                        serde_json::to_string_pretty(actual)
                            .expect("failed to pretty print JSON")
                            .indent(8),
                    ),

                    ErrorType::NotEq(Either::Right((lhs, rhs)), path) => format!(
                        r#"json atoms at path "{}" are not equal:
    lhs:
{}
    rhs:
{}"#,
                        path,
                        serde_json::to_string_pretty(lhs)
                            .expect("failed to pretty print JSON")
                            .indent(8),
                        serde_json::to_string_pretty(rhs)
                            .expect("failed to pretty print JSON")
                            .indent(8),
                    ),
                    ErrorType::MissingPath(Either::Left(path)) => {
                        format!(r#"json atom at path "{}" is missing from actual"#, path)
                    }
                    ErrorType::MissingPath(Either::Right((path, SideWithoutPath::Lhs))) => {
                        format!(r#"json atom at path "{}" is missing from lhs"#, path)
                    }
                    ErrorType::MissingPath(Either::Right((path, SideWithoutPath::Rhs))) => {
                        format!(r#"json atom at path "{}" is missing from rhs"#, path)
                    }
                })
                .collect::<Vec<_>>();
            Err(messages.join("\n\n"))
        }
    }

    fn push(&mut self, error: ErrorType) {
        self.errors.push(error);
    }
}

enum ErrorType {
    NotEq(Either<(Actual, Expected), (Value, Value)>, Path),
    MissingPath(Either<Path, (Path, SideWithoutPath)>),
}

enum SideWithoutPath {
    Lhs,
    Rhs,
}

#[cfg(test)]
mod tests {
    #[macro_use]
    use super::*;

    #[test]
    fn boolean_root() {
        let result = test_partial_match(Actual(json!(true)), Expected(json!(true)));
        assert_output_eq(result, Ok(()));

        let result = test_partial_match(Actual(json!(false)), Expected(json!(false)));
        assert_output_eq(result, Ok(()));

        let result = test_partial_match(Actual(json!(false)), Expected(json!(true)));
        assert_output_eq(
            result,
            Err(r#"json atoms at path "(root)" are not equal:
    expected:
        true
    actual:
        false"#),
        );

        let result = test_partial_match(Actual(json!(true)), Expected(json!(false)));
        assert_output_eq(
            result,
            Err(r#"json atoms at path "(root)" are not equal:
    expected:
        false
    actual:
        true"#),
        );
    }

    #[test]
    fn string_root() {
        let result = test_partial_match(Actual(json!("true")), Expected(json!("true")));
        assert_output_eq(result, Ok(()));

        let result = test_partial_match(Actual(json!("false")), Expected(json!("false")));
        assert_output_eq(result, Ok(()));

        let result = test_partial_match(Actual(json!("false")), Expected(json!("true")));
        assert_output_eq(
            result,
            Err(r#"json atoms at path "(root)" are not equal:
    expected:
        "true"
    actual:
        "false""#),
        );

        let result = test_partial_match(Actual(json!("true")), Expected(json!("false")));
        assert_output_eq(
            result,
            Err(r#"json atoms at path "(root)" are not equal:
    expected:
        "false"
    actual:
        "true""#),
        );
    }

    #[test]
    fn number_root() {
        let result = test_partial_match(Actual(json!(1)), Expected(json!(1)));
        assert_output_eq(result, Ok(()));

        let result = test_partial_match(Actual(json!(0)), Expected(json!(0)));
        assert_output_eq(result, Ok(()));

        let result = test_partial_match(Actual(json!(0)), Expected(json!(1)));
        assert_output_eq(
            result,
            Err(r#"json atoms at path "(root)" are not equal:
    expected:
        1
    actual:
        0"#),
        );

        let result = test_partial_match(Actual(json!(1)), Expected(json!(0)));
        assert_output_eq(
            result,
            Err(r#"json atoms at path "(root)" are not equal:
    expected:
        0
    actual:
        1"#),
        );
    }

    #[test]
    fn null_root() {
        let result = test_partial_match(Actual(json!(null)), Expected(json!(null)));
        assert_output_eq(result, Ok(()));

        let result = test_partial_match(Actual(json!(null)), Expected(json!(1)));
        assert_output_eq(
            result,
            Err(r#"json atoms at path "(root)" are not equal:
    expected:
        1
    actual:
        null"#),
        );

        let result = test_partial_match(Actual(json!(1)), Expected(json!(null)));
        assert_output_eq(
            result,
            Err(r#"json atoms at path "(root)" are not equal:
    expected:
        null
    actual:
        1"#),
        );
    }

    #[test]
    fn into_object() {
        let result =
            test_partial_match(Actual(json!({ "a": true })), Expected(json!({ "a": true })));
        assert_output_eq(result, Ok(()));

        let result = test_partial_match(
            Actual(json!({ "a": false })),
            Expected(json!({ "a": true })),
        );
        assert_output_eq(
            result,
            Err(r#"json atoms at path ".a" are not equal:
    expected:
        true
    actual:
        false"#),
        );

        let result = test_partial_match(
            Actual(json!({ "a": { "b": true } })),
            Expected(json!({ "a": { "b": true } })),
        );
        assert_output_eq(result, Ok(()));

        let result = test_partial_match(
            Actual(json!({ "a": true })),
            Expected(json!({ "a": { "b": true } })),
        );
        assert_output_eq(
            result,
            Err(r#"json atom at path ".a.b" is missing from actual"#),
        );

        let result = test_partial_match(Actual(json!({})), Expected(json!({ "a": true })));
        assert_output_eq(
            result,
            Err(r#"json atom at path ".a" is missing from actual"#),
        );

        let result = test_partial_match(
            Actual(json!({ "a": { "b": true } })),
            Expected(json!({ "a": true })),
        );
        assert_output_eq(
            result,
            Err(r#"json atoms at path ".a" are not equal:
    expected:
        true
    actual:
        {
          "b": true
        }"#),
        );
    }

    #[test]
    fn into_array() {
        let result = test_partial_match(Actual(json!([1])), Expected(json!([1])));
        assert_output_eq(result, Ok(()));

        let result = test_partial_match(Actual(json!([2])), Expected(json!([1])));
        assert_output_eq(
            result,
            Err(r#"json atoms at path "[0]" are not equal:
    expected:
        1
    actual:
        2"#),
        );

        let result = test_partial_match(Actual(json!([1, 2, 4])), Expected(json!([1, 2, 3])));
        assert_output_eq(
            result,
            Err(r#"json atoms at path "[2]" are not equal:
    expected:
        3
    actual:
        4"#),
        );

        let result = test_partial_match(
            Actual(json!({ "a": [1, 2, 3]})),
            Expected(json!({ "a": [1, 2, 4]})),
        );
        assert_output_eq(
            result,
            Err(r#"json atoms at path ".a[2]" are not equal:
    expected:
        4
    actual:
        3"#),
        );

        let result = test_partial_match(
            Actual(json!({ "a": [1, 2, 3]})),
            Expected(json!({ "a": [1, 2]})),
        );
        assert_output_eq(result, Ok(()));

        let result = test_partial_match(
            Actual(json!({ "a": [1, 2]})),
            Expected(json!({ "a": [1, 2, 3]})),
        );
        assert_output_eq(
            result,
            Err(r#"json atom at path ".a[2]" is missing from actual"#),
        );
    }

    #[test]
    fn exact_matching() {
        let result = test_exact_match(json!(true), json!(true));
        assert_output_eq(result, Ok(()));

        let result = test_exact_match(json!("s"), json!("s"));
        assert_output_eq(result, Ok(()));

        let result = test_exact_match(json!("a"), json!("b"));
        assert_output_eq(
            result,
            Err(r#"json atoms at path "(root)" are not equal:
    lhs:
        "a"
    rhs:
        "b""#),
        );

        let result = test_exact_match(
            json!({ "a": [1, { "b": 2 }] }),
            json!({ "a": [1, { "b": 3 }] }),
        );
        assert_output_eq(
            result,
            Err(r#"json atoms at path ".a[1].b" are not equal:
    lhs:
        2
    rhs:
        3"#),
        );
    }

    #[test]
    fn exact_match_output_message() {
        let result = test_exact_match(json!({ "a": { "b": 1 } }), json!({ "a": {} }));
        assert_output_eq(
            result,
            Err(r#"json atom at path ".a.b" is missing from rhs"#),
        );

        let result = test_exact_match(json!({ "a": {} }), json!({ "a": { "b": 1 } }));
        assert_output_eq(
            result,
            Err(r#"json atom at path ".a.b" is missing from lhs"#),
        );
    }

    fn assert_output_eq(actual: Result<(), String>, expected: Result<(), &str>) {
        match (actual, expected) {
            (Ok(()), Ok(())) => return,

            (Err(actual_error), Ok(())) => {
                println!("Did not expect error, but got");
                println!("{}", actual_error);
            }

            (Ok(()), Err(expected_error)) => {
                let expected_error = expected_error.to_string();
                println!("Expected error, but did not get one. Expected error:");
                println!("{}", expected_error);
            }

            (Err(actual_error), Err(expected_error)) => {
                let expected_error = expected_error.to_string();
                if actual_error == expected_error {
                    return;
                } else {
                    println!("Errors didn't match");
                    println!("Expected:");
                    println!("{}", expected_error);
                    println!("Got:");
                    println!("{}", actual_error);
                }
            }
        }

        panic!("assertion error, see stdout");
    }

    fn test_partial_match(actual: Actual, expected: Expected) -> Result<(), String> {
        let comparison = Comparison::Include(actual, expected);
        assert_json_no_panic(comparison)
    }

    fn test_exact_match(lhs: Value, rhs: Value) -> Result<(), String> {
        let comparison = Comparison::Exact(lhs, rhs);
        assert_json_no_panic(comparison)
    }
}
