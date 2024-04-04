// Copyright 2016 Urban Hafner
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

use regex::Regex;
use std::borrow::Borrow;
use std::fmt;

use crate::core::*;

pub struct MatchesRegex {
  regex: Regex,
}

impl fmt::Display for MatchesRegex {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    self.regex.fmt(f)
  }
}

impl<B: Borrow<str>> Matcher<B> for MatchesRegex {
  fn matches(&self, actual: B) -> MatchResult {
    if self.regex.is_match(actual.borrow()) {
      success()
    } else {
      Err(format!("was {:?}", actual.borrow()))
    }
  }
}

pub fn matches_regex(regex: &str) -> MatchesRegex {
  MatchesRegex {
    regex: Regex::new(regex).unwrap(),
  }
}
