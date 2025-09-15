// Copyright 2016 Urban Hafner
// Copyright 2018 Val Markovic
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

extern crate hamcrest2;

mod contains {
  use hamcrest2::prelude::*;

  #[test]
  fn vec_contains() {
    let vec = vec![1, 2, 3];
    assert_that!(&vec, contains(vec![1, 2]));
    assert_that!(&vec, not(contains(vec![4])));
  }

  #[test]
  fn slice_contains() {
    let slice: &[i32] = &[1, 2, 3];
    assert_that!(slice, contains(vec![1, 2]));
    assert_that!(slice, not(contains(vec![4])));
  }

  #[test]
  fn single_item_contains() {
    assert_that!(&[1, 2, 3], contains(2));
    assert_that!(&[1, 2, 3], not(contains(4)));
  }

  #[test]
  fn vec_contains_exactly() {
    assert_that!(&[1, 2, 3], contains(vec![1, 2, 3]).exactly());
    assert_that!(&[1, 2, 3], not(contains(vec![1, 2]).exactly()));
  }

  #[test]
  fn it_contains_elements_in_order() {
    assert_that!(&[1, 2, 3], contains(vec![1, 2]).in_order());
  }

  #[test]
  fn it_does_not_contain_elements_in_order() {
    assert_that!(&[1, 2, 3], not(contains(vec![1, 3]).in_order()));
  }

  #[test]
  #[should_panic]
  fn it_unsuccessfully_contains_elements_in_order() {
    assert_that!(&[1, 2, 3], contains(vec![1, 3]).in_order());
  }

  #[test]
  #[should_panic]
  fn it_unsuccessfully_does_not_contain_elements_in_order() {
    assert_that!(&[1, 2, 3], not(contains(vec![2, 3]).in_order()));
  }
}
