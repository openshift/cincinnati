// Copyright 2016 Urban Hafner
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

extern crate hamcrest2;

mod anything {

  use hamcrest2::prelude::*;

  #[test]
  fn usize_is_anything() {
    assert_that!(123, is(anything()));
  }

  #[test]
  fn str_is_anything() {
    assert_that!("test", is(anything()));
  }
}
