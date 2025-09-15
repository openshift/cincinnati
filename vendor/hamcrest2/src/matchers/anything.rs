// Copyright 2017 Flier Lu
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.
use std::fmt::{self, Display, Formatter};

use crate::core::*;

pub struct Anything;

impl Display for Anything {
  fn fmt(&self, f: &mut Formatter) -> fmt::Result {
    write!(f, "anything")
  }
}

impl<T> Matcher<T> for Anything {
  fn matches(&self, _: T) -> MatchResult {
    success()
  }
}

/// always matches, useful if you don't care what the object under test is
pub fn anything() -> Anything {
  Anything
}
