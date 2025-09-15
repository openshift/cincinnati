// Copyright 2018 Val Markovic
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

use std::fmt;

use crate::core::*;
use crate::utils::*;

#[derive(Clone, Copy)]
pub struct Empty {}

impl fmt::Display for Empty {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    write!(f, "is empty")
  }
}

impl<'a, T: fmt::Debug> Matcher<&'a [T]> for Empty {
  fn matches(&self, actual: &[T]) -> MatchResult {
    if actual.is_empty() {
      success()
    } else {
      Err(format!("was {}", Pretty(&actual)))
    }
  }
}

pub fn empty() -> Empty {
  Empty {}
}
