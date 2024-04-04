// Copyright 2018 Val Markovic
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

extern crate hamcrest2;

mod has {
  use hamcrest2::prelude::*;

  #[test]
  fn has_with_some() {
    let var: Option<i8> = Some(5);
    assert_that!(var, has(5));
  }

  #[test]
  fn has_with_none() {
    assert_that!(None, not(has(5)));
  }

  #[test]
  fn has_with_ok() {
    let var: Result<i8, String> = Ok(5);
    assert_that!(var, has(5));
  }

  #[test]
  fn has_with_err() {
    let var: Result<i8, String> = Err("bad".to_string());
    assert_that!(var, not(has(5)));
  }
}
