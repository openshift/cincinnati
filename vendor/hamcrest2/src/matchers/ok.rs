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

pub struct IsOk<T, E> {
  marker: PhantomData<T>,
  marker2: PhantomData<E>,
}

impl<T, E> fmt::Display for IsOk<T, E> {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    write!(f, "Ok(_)")
  }
}

impl<T, E, B: Borrow<Result<T, E>>> Matcher<B> for IsOk<T, E>
where
  T: fmt::Debug,
  E: fmt::Debug,
{
  fn matches(&self, actual: B) -> MatchResult {
    match actual.borrow() {
      e @ Err(_) => Err(format!("was {:?}", e)),
      Ok(_) => success(),
    }
  }
}

pub fn ok<T, E>() -> IsOk<T, E> {
  IsOk {
    marker: PhantomData,
    marker2: PhantomData,
  }
}
