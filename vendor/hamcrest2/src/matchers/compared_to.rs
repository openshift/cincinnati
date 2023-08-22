// Copyright 2014 Carl Lerche, Steve Klabnik, Alex Crichton, Yehuda Katz,
//                Ben Longbons
// Copyright 2015 Carl Lerche, Alex Crichton, Robin Gloster
// Copyright 2016 Urban Hafner
// Copyright 2017 Matt LaChance
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

use std::borrow::Borrow;
use std::fmt;

use crate::core::*;

enum CompareOperation {
  LessOrEqual,
  LessThan,
  GreaterOrEqual,
  GreaterThan,
}

pub struct ComparedTo<T> {
  operation: CompareOperation,
  right_hand_side: T,
}

impl<T: fmt::Debug> fmt::Display for ComparedTo<T> {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    let operation = match self.operation {
      CompareOperation::LessOrEqual => "<=",
      CompareOperation::LessThan => "<",
      CompareOperation::GreaterOrEqual => ">=",
      CompareOperation::GreaterThan => ">",
    };

    write!(f, "{} {:?}", operation, &self.right_hand_side)
  }
}

impl<T: PartialOrd + fmt::Debug, B: Borrow<T>> Matcher<B> for ComparedTo<T> {
  fn matches(&self, actual: B) -> MatchResult {
    let actual_borrowed = actual.borrow();
    let it_succeeded = match self.operation {
      CompareOperation::LessOrEqual => actual_borrowed <= &self.right_hand_side,
      CompareOperation::LessThan => actual_borrowed < &self.right_hand_side,
      CompareOperation::GreaterOrEqual => {
        actual_borrowed >= &self.right_hand_side
      }
      CompareOperation::GreaterThan => actual_borrowed > &self.right_hand_side,
    };

    if it_succeeded {
      success()
    } else {
      Err(format!("was {:?}", actual_borrowed))
    }
  }
}

pub fn less_than<T: PartialOrd + fmt::Debug>(
  right_hand_side: T,
) -> ComparedTo<T> {
  ComparedTo {
    operation: CompareOperation::LessThan,
    right_hand_side,
  }
}

pub fn less_than_or_equal_to<T: PartialOrd + fmt::Debug>(
  right_hand_side: T,
) -> ComparedTo<T> {
  ComparedTo {
    operation: CompareOperation::LessOrEqual,
    right_hand_side,
  }
}

pub fn greater_than<T: PartialOrd + fmt::Debug>(
  right_hand_side: T,
) -> ComparedTo<T> {
  ComparedTo {
    operation: CompareOperation::GreaterThan,
    right_hand_side,
  }
}

pub fn greater_than_or_equal_to<T: PartialOrd + fmt::Debug>(
  right_hand_side: T,
) -> ComparedTo<T> {
  ComparedTo {
    operation: CompareOperation::GreaterOrEqual,
    right_hand_side,
  }
}
