// Copyright 2016 Urban Hafner
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

extern crate hamcrest2;

mod type_of {

  use hamcrest2::prelude::*;

  #[test]
  fn usize_is_type_of_usize() {
    assert_that!(123usize, is(type_of::<usize>()));
  }

  #[test]
  fn str_is_type_of_str() {
    assert_that!("test", is(type_of::<&str>()));
  }
}
