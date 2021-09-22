// Copyright 2014 Carl Lerche, Alex Crichton, Michael Gehring, Yehuda Katz
// Copyright 2015 Carl Lerche, Alex Crichton, Robin Gloster
// Copyright 2018 Val Markovic
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

use std::fmt;
use std::marker::PhantomData;

use crate::core::*;

pub struct Is<T, M> {
  matcher: M,
  marker: PhantomData<T>,
}

impl<T, M: Matcher<T>> fmt::Display for Is<T, M> {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    self.matcher.fmt(f)
  }
}

impl<T, M: Matcher<T>> Matcher<T> for Is<T, M> {
  fn matches(&self, actual: T) -> MatchResult {
    self.matcher.matches(actual)
  }
}

pub fn is<T, M: Matcher<T>>(matcher: M) -> Is<T, M> {
  Is {
    matcher,
    marker: PhantomData,
  }
}

pub struct IsNot<T, M> {
  matcher: M,
  marker: PhantomData<T>,
}

impl<T, M: Matcher<T>> fmt::Display for IsNot<T, M> {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    write!(f, "not {}", self.matcher)
  }
}

impl<T, M: Matcher<T>> Matcher<T> for IsNot<T, M> {
  fn matches(&self, actual: T) -> MatchResult {
    match self.matcher.matches(actual) {
      Ok(_) => Err("matched".to_string()),
      Err(_) => Ok(()),
    }
  }
}

pub fn is_not<T, M: Matcher<T>>(matcher: M) -> IsNot<T, M> {
  IsNot {
    matcher,
    marker: PhantomData,
  }
}
