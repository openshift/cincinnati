// Copyright 2016 Urban Hafner
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

extern crate hamcrest2;

mod all {
  use hamcrest2::prelude::*;

  #[test]
  fn ints_less_than_and_greater_than() {
    assert_that!(4, all!(less_than(5), greater_than(3)));
  }

  #[test]
  fn vec_contains() {
    assert_that!(
      &[1, 2, 3],
      all!(contains(vec![1, 2]), not(contains(vec![4])))
    );
  }
}
