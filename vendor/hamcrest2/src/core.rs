// Copyright 2014 Carl Lerche, Steve Klabnik, Alex Crichton
// Copyright 2015 Carl Lerche
// Copyright 2016 Urban Hafner
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

use std::fmt;

pub type MatchResult = Result<(), String>;

pub fn success() -> MatchResult {
  Ok(())
}

pub fn expect(predicate: bool, msg: String) -> MatchResult {
  if predicate {
    success()
  } else {
    Err(msg)
  }
}

#[deprecated(since = "0.1.2", note = "Use the assert_that! macro instead")]
pub fn assert_that<T, U: Matcher<T>>(actual: T, matcher: &U) {
  match matcher.matches(actual) {
    Ok(_) => {}
    Err(mismatch) => {
      panic!("\nExpected: {}\n    but: {}", matcher, mismatch);
    }
  }
}

pub trait Matcher<T>: fmt::Display {
  fn matches(&self, actual: T) -> MatchResult;
}
