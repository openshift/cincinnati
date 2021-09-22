// Copyright 2016 Urban Hafner
// Copyright 2018 Val Markovic
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

extern crate hamcrest2;

mod len {
  use hamcrest2::prelude::*;

  #[test]
  fn vec_len() {
    let vec = vec![1, 2, 3];
    assert_that!(&vec, len(3));
    assert_that!(&vec, is(len(3)));
  }

  #[test]
  fn slice_len() {
    let slice: &[i32] = &[1, 2, 3];
    assert_that!(slice, len(3));
    assert_that!(slice, not(len(4)));
  }
}
