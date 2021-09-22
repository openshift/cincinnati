// Copyright 2016 Urban Hafner
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

extern crate hamcrest2;

mod close_to {
  use hamcrest2::prelude::*;
  use std::f64;

  #[test]
  fn equality_of_floats() {
    assert_that!(1.0f64, close_to(1.0, 0.00001));
    assert_that!(1e-40f32, close_to(0.0, 0.01));
    assert_that!(1e-40f32, not(close_to(0.0, 0.000_001)));
    assert_that!(2.0, not(close_to(1.0f64, 0.00001)));
  }

  #[test]
  fn it_can_handle_infinity() {
    assert_that!(f64::INFINITY, close_to(f64::INFINITY, 0.00001));
  }

  #[test]
  fn it_can_handle_nan() {
    assert_that!(f64::NAN, is(not(close_to(f64::NAN, 0.00001))));
  }
}
