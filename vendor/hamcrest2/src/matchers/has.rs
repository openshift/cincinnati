// Copyright 2018 Val Markovic
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

use std::fmt;

use crate::core::*;

pub struct Has<T> {
  value: T,
}

impl<T> fmt::Display for Has<T>
where
  T: fmt::Debug,
{
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    write!(f, "has {:?}", self.value)
  }
}

impl<T> Matcher<Option<T>> for Has<T>
where
  T: fmt::Debug + PartialEq,
{
  fn matches(&self, actual: Option<T>) -> MatchResult {
    match actual {
      None => Err("was None".to_string()),
      Some(v) => {
        if v == self.value {
          success()
        } else {
          Err(format!("was Some({:?})", v))
        }
      }
    }
  }
}

impl<T, E> Matcher<Result<T, E>> for Has<T>
where
  T: fmt::Debug + PartialEq,
  E: fmt::Debug,
{
  fn matches(&self, actual: Result<T, E>) -> MatchResult {
    match actual {
      Err(v) => Err(format!("was Err({:?})", v)),
      Ok(v) => {
        if v == self.value {
          success()
        } else {
          Err(format!("was Ok({:?})", v))
        }
      }
    }
  }
}

pub fn has<T>(value: T) -> Has<T> {
  Has { value }
}
