// Copyright 2018 Val Markovic
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

extern crate hamcrest2;

mod empty {
  use hamcrest2::prelude::*;

  #[test]
  fn vec_empty() {
    assert_that!(&Vec::<i32>::new(), empty());
    assert_that!(&[1, 2, 3], not(empty()));
  }

  #[test]
  fn slice_empty() {
    let slice: &[i32] = &[1, 2, 3];
    assert_that!(slice, not(empty()));

    let empty_slice: &[i32] = &Vec::<i32>::new();
    assert_that!(empty_slice, empty());
  }
}
