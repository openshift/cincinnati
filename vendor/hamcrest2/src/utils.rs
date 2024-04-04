// Copyright 2018 Val Markovic
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

use std::fmt;

pub(crate) struct Pretty<'a, T: 'a>(pub &'a [T]);

impl<'a, T: fmt::Debug> fmt::Display for Pretty<'a, T> {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    write!(f, "[")?;
    for (i, t) in self.0.iter().enumerate() {
      if i != 0 {
        write!(f, ", ")?;
      }
      write!(f, "{:?}", t)?;
    }
    write!(f, "]")
  }
}
