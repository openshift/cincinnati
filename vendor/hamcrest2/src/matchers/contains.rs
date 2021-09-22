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
use std::vec::Vec;

use crate::core::*;
use crate::utils::*;

#[derive(Clone)]
pub struct Contains<T> {
  items: Vec<T>,
  exactly: bool,
  in_order: bool,
}

impl<T> Contains<T> {
  /// Constructs new `Contains` matcher with the default options: order is not checked and
  /// actual vector can have more items.
  pub fn new(items: Vec<T>) -> Self {
    Self {
      items,
      exactly: false,
      in_order: false,
    }
  }

  pub fn exactly(mut self) -> Contains<T> {
    self.exactly = true;
    self
  }

  pub fn in_order(mut self) -> Contains<T> {
    self.in_order = true;
    self
  }
}

impl<T> From<Vec<T>> for Contains<T> {
  fn from(items: Vec<T>) -> Contains<T> {
    Contains::new(items)
  }
}

impl<T> From<T> for Contains<T> {
  fn from(item: T) -> Contains<T> {
    Contains::new(vec![item])
  }
}

impl<T: fmt::Debug> fmt::Display for Contains<T> {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    if self.exactly {
      write!(f, "containing exactly {}", Pretty(&self.items))
    } else {
      write!(f, "containing {}", Pretty(&self.items))
    }
  }
}

impl<'a, T: fmt::Debug + PartialEq + Clone> Matcher<&'a [T]> for Contains<T> {
  fn matches(&self, actual: &[T]) -> MatchResult {
    let mut rem = actual.to_vec();

    for item in &self.items {
      match rem.iter().position(|a| *item == *a) {
        Some(idx) => {
          rem.remove(idx);
        }
        None => return Err(format!("was {}", Pretty(&actual))),
      }
    }

    if self.exactly && !rem.is_empty() {
      return Err(format!("also had {}", Pretty(&rem)));
    }

    if self.in_order && !contains_in_order(actual, &self.items) {
      return Err(format!(
        "{} does not contain {} in order",
        Pretty(&actual),
        Pretty(&self.items)
      ));
    }

    success()
  }
}

fn contains_in_order<T: fmt::Debug + PartialEq>(
  actual: &[T],
  items: &[T],
) -> bool {
  let mut previous = None;

  for item in items.iter() {
    match actual.iter().position(|a| *item == *a) {
      Some(current) => {
        if !is_next_index(current, &previous) {
          return false;
        }
        previous = Some(current);
      }
      None => return false,
    }
  }
  true
}

fn is_next_index(current_index: usize, previous_index: &Option<usize>) -> bool {
  if let Some(index) = *previous_index {
    return current_index == index + 1;
  }
  true
}

/// Creates matcher that checks if actual data contains give item(s).
pub fn contains<T, I>(item: I) -> Contains<T>
where
  I: Into<Contains<T>>,
{
  item.into()
}
