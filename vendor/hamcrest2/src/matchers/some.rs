// Copyright 2018 Val Markovic
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

use std::borrow::Borrow;
use std::fmt;
use std::marker::PhantomData;

use crate::core::*;

pub struct IsSome<T> {
  marker: PhantomData<T>,
}

impl<T> fmt::Display for IsSome<T> {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    write!(f, "Some(_)")
  }
}

impl<T: fmt::Debug, B: Borrow<Option<T>>> Matcher<B> for IsSome<T> {
  fn matches(&self, actual: B) -> MatchResult {
    match actual.borrow() {
      None => Err("was None".to_string()),
      Some(_) => success(),
    }
  }
}

pub fn some<T>() -> IsSome<T> {
  IsSome {
    marker: PhantomData,
  }
}
