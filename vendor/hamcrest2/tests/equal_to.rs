// Copyright 2016 Urban Hafner
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

extern crate hamcrest2;

mod equal_to {
  use hamcrest2::prelude::*;

  #[derive(Debug)]
  pub struct A(u8);
  #[derive(Debug)]
  pub struct B(u8);

  impl PartialEq<B> for A {
    fn eq(&self, other: &B) -> bool {
      self.0 == other.0
    }
  }

  impl PartialEq<A> for B {
    fn eq(&self, other: &A) -> bool {
      self.0 == other.0
    }
  }

  #[test]
  fn equality_with_special_partial_eq() {
    assert_that!(A(1), eq(B(1)));
    assert_that!(B(1), eq(A(1)));
  }

  #[test]
  fn equality_of_ints() {
    assert_that!(1, is(equal_to(1)));
    assert_that!(1, eq(1));
  }

  #[test]
  #[should_panic]
  fn unsuccessful_match() {
    assert_that!(2, is(equal_to(1)));
  }

  #[test]
  #[should_panic]
  fn unsuccessful_match_short() {
    assert_that!(2, eq(1));
  }

  #[test]
  fn equality_of_strings() {
    assert_that!("", is(equal_to("")));
    assert_that!("", is(equal_to(String::new())));
    assert_that!(String::new(), is(equal_to("")));
    assert_that!("", is(equal_to(String::new())));
  }

  #[test]
  fn equality_of_floats_slice_ref() {
    let slice = [1.0, 2.0, 3.0];
    assert_that!(&slice, eq(&[1.0, 2.0, 3.0]));
  }

  #[test]
  fn equality_of_floats_slice_mut() {
    let mut slice = [1.0, 2.0, 3.0];
    assert_that!(&mut slice, eq(&[1.0, 2.0, 3.0]));
  }
}
