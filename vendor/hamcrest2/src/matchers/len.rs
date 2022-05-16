// Copyright 2014 Carl Lerche, Yehuda Katz, Steve Klabnik, Alex Crichton,
//                Ben Longbons
// Copyright 2015 Carl Lerche, Graham Dennis, Alex Crichton, Tamir Duberstein,
//                Robin Gloster
// Copyright 2016 Urban Hafner
// Copyright 2018 Val Markovic
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

use std::fmt;

use crate::core::*;

#[derive(Clone, Copy)]
pub struct OfLen {
  len: usize,
}

impl fmt::Display for OfLen {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    write!(f, "of len {}", self.len)
  }
}

impl<'a, T> Matcher<&'a [T]> for OfLen {
  fn matches(&self, actual: &[T]) -> MatchResult {
    if self.len == actual.len() {
      success()
    } else {
      Err(format!("was len {}", actual.len()))
    }
  }
}

pub fn len(len: usize) -> OfLen {
  OfLen { len }
}
