// Copyright 2016 Urban Hafner
// Copyright 2017 Matt LaChance
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

extern crate hamcrest2;

mod compared_to {

  use hamcrest2::prelude::*;

  #[test]
  fn ints_less_than() {
    assert_that!(4, is(less_than(5)));
    assert_that!(&4, is(less_than(5)));
    assert_that!(&mut 4, is(less_than(5)));
  }

  #[test]
  #[should_panic]
  fn unsuccessful_less_than() {
    assert_that!(4, is(less_than(3)));
  }

  #[test]
  #[should_panic]
  fn unsuccessful_less_than_ref() {
    assert_that!(&4, is(less_than(3)));
  }

  #[test]
  #[should_panic]
  fn unsuccessful_less_than_mut() {
    assert_that!(&mut 4, is(less_than(3)));
  }

  #[test]
  #[should_panic]
  fn less_than_is_not_equal() {
    assert_that!(2, is(less_than(2)));
  }

  #[test]
  #[should_panic]
  fn less_than_is_not_equal_ref() {
    assert_that!(&2, is(less_than(2)));
  }

  #[test]
  #[should_panic]
  fn less_than_is_not_equal_mut() {
    assert_that!(&mut 2, is(less_than(2)));
  }

  #[test]
  fn ints_greater_than() {
    assert_that!(8, is(greater_than(5)));
    assert_that!(&8, is(greater_than(5)));
    assert_that!(&mut 8, is(greater_than(5)));
  }

  #[test]
  #[should_panic]
  fn unsuccessful_greater_than() {
    assert_that!(1, is(greater_than(3)));
  }

  #[test]
  #[should_panic]
  fn unsuccessful_greater_than_ref() {
    assert_that!(&1, is(greater_than(3)));
  }

  #[test]
  #[should_panic]
  fn unsuccessful_greater_than_mut() {
    assert_that!(&mut 1, is(greater_than(3)));
  }

  #[test]
  #[should_panic]
  fn greater_than_is_not_equal() {
    assert_that!(2, is(greater_than(2)));
  }

  #[test]
  #[should_panic]
  fn greater_than_is_not_equal_ref() {
    assert_that!(&2, is(greater_than(2)));
  }

  #[test]
  #[should_panic]
  fn greater_than_is_not_equal_mut() {
    assert_that!(&mut 2, is(greater_than(2)));
  }

  #[test]
  fn ints_less_than_or_equal() {
    assert_that!(3, is(less_than_or_equal_to(7)));
    assert_that!(&3, is(less_than_or_equal_to(7)));
    assert_that!(&mut 3, is(less_than_or_equal_to(7)));
    assert_that!(3, is(less_than_or_equal_to(3)));
    assert_that!(&3, is(less_than_or_equal_to(3)));
    assert_that!(&mut 3, is(less_than_or_equal_to(3)));
  }

  #[test]
  #[should_panic]
  fn unsuccessful_less_than_or_equal() {
    assert_that!(4, is(less_than_or_equal_to(3)));
  }

  #[test]
  #[should_panic]
  fn unsuccessful_less_than_or_equal_ref() {
    assert_that!(&4, is(less_than_or_equal_to(3)));
  }

  #[test]
  #[should_panic]
  fn unsuccessful_less_than_or_equal_mut() {
    assert_that!(&mut 4, is(less_than_or_equal_to(3)));
  }

  #[test]
  fn ints_greater_than_or_equal() {
    assert_that!(6, is(greater_than_or_equal_to(5)));
    assert_that!(&6, is(greater_than_or_equal_to(5)));
    assert_that!(&mut 6, is(greater_than_or_equal_to(5)));
    assert_that!(8, is(greater_than_or_equal_to(8)));
    assert_that!(&8, is(greater_than_or_equal_to(8)));
    assert_that!(&mut 8, is(greater_than_or_equal_to(8)));
  }

  #[test]
  #[should_panic]
  fn unsuccessful_greater_than_or_equal() {
    assert_that!(4, is(greater_than_or_equal_to(5)));
  }

  #[test]
  #[should_panic]
  fn unsuccessful_greater_than_or_equal_ref() {
    assert_that!(&4, is(greater_than_or_equal_to(5)));
  }

  #[test]
  #[should_panic]
  fn unsuccessful_greater_than_or_equal_mut() {
    assert_that!(&mut 4, is(greater_than_or_equal_to(5)));
  }
}
