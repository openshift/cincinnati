// Copyright 2016 Urban Hafner
// Copyright 2018 Val Markovic
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

extern crate hamcrest2;

mod none {
  use hamcrest2::prelude::*;

  mod as_move {
    use super::*;

    #[test]
    fn none_no_explicit_type() {
      let var: Option<i8> = None;
      assert_that!(var, none());
    }

    #[test]
    fn none_is_none() {
      assert_that!(None, is(none::<i8>()));
    }

    #[test]
    fn some_is_not_none() {
      assert_that!(Some(1), is_not(none()));
    }
  }

  mod as_ref {
    use super::*;

    #[test]
    fn none_no_explicit_type() {
      let var: Option<i8> = None;
      assert_that!(&var, none());
    }

    #[test]
    fn none_is_none() {
      assert_that!(&None, is(none::<i8>()));
    }

    #[test]
    fn some_is_not_none() {
      assert_that!(&Some(1), is_not(none()));
    }
  }

  mod as_mut_ref {
    use super::*;

    #[test]
    fn none_no_explicit_type() {
      let mut var: Option<i8> = None;
      assert_that!(&mut var, none());
    }

    #[test]
    fn none_is_none() {
      assert_that!(&mut None, is(none::<i8>()));
    }

    #[test]
    fn some_is_not_none() {
      assert_that!(&mut Some(1), is_not(none()));
    }
  }
}
