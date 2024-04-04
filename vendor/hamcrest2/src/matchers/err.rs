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

pub struct IsErr<T, E> {
  marker: PhantomData<T>,
  marker2: PhantomData<E>,
}

impl<T, E> fmt::Display for IsErr<T, E> {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    write!(f, "Err(_)")
  }
}

impl<T, E, B: Borrow<Result<T, E>>> Matcher<B> for IsErr<T, E>
where
  T: fmt::Debug,
  E: fmt::Debug,
{
  fn matches(&self, actual: B) -> MatchResult {
    match actual.borrow() {
      Err(_) => success(),
      v @ Ok(_) => Err(format!("was {:?}", v)),
    }
  }
}

pub fn err<T, E>() -> IsErr<T, E> {
  IsErr {
    marker: PhantomData,
    marker2: PhantomData,
  }
}
