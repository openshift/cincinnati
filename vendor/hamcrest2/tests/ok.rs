// Copyright 2018 Val Markovic
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

extern crate hamcrest2;

mod ok {
  use hamcrest2::prelude::*;

  mod as_move {
    use super::*;

    #[test]
    fn ok_no_explicit_type() {
      let var: Result<i8, ()> = Ok(5);
      assert_that!(var, ok());
    }

    #[test]
    fn ok_is_ok() {
      assert_that!(Ok(5), ok::<i8, ()>());
    }

    #[test]
    fn err_is_not_ok() {
      let var: Result<i8, ()> = Err(());
      assert_that!(var, not(ok()));
    }
  }

  mod as_ref {
    use super::*;

    #[test]
    fn ok_no_explicit_type() {
      let var: Result<i8, ()> = Ok(5);
      assert_that!(&var, ok());
    }

    #[test]
    fn ok_is_ok() {
      assert_that!(&Ok(5), ok::<i8, ()>());
    }

    #[test]
    fn err_is_not_ok() {
      let var: Result<i8, ()> = Err(());
      assert_that!(&var, not(ok()));
    }
  }

  mod as_mut_ref {
    use super::*;

    #[test]
    fn ok_no_explicit_type() {
      let mut var: Result<i8, ()> = Ok(5);
      assert_that!(&mut var, ok());
    }

    #[test]
    fn ok_is_ok() {
      assert_that!(&mut Ok(5), ok::<i8, ()>());
    }

    #[test]
    fn err_is_not_ok() {
      let mut var: Result<i8, ()> = Err(());
      assert_that!(&mut var, not(ok()));
    }
  }
}
