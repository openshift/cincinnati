// Copyright 2014 Carl Lerche, Oliver Mader, Alex Crichton, Thiago Pontes,
//                Yehuda Katz
// Copyright 2015 Carl Lerche, Oliver Mader
// Copyright 2016 Urban Hafner
// Copyright 2018 Val Markovic
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! ## Usage
//!
//! Hamcrest2 supports a number of matchers. The easiest way is to just `use`
//! them all like this:
//!
//! ```
//! use hamcrest2::prelude::*;
//! ```
//!
//! If you want to be more selective make sure that you also import the
//! `HamcrestMatcher` trait.
//!
//! ## General Matchers
//!
//! ### eq, not
//!
//! ```
//! # use hamcrest2::prelude::*;
//! assert_that!(1, eq(1));  // also equal_to()
//! assert_that!(1, not(eq(2)));
//! ```
//!
//! ### compared\_to
//!
//! ```
//! # use hamcrest2::prelude::*;
//! assert_that!(1, lt(2));   // also less_than()
//! assert_that!(1, leq(1));  // also less_than_or_equal_to()
//! assert_that!(2, gt(1));   // also greater_than()
//! assert_that!(2, geq(2));  // also greater_than_or_equal_to()
//! ```
//!
//! ### type_of
//!
//! ```
//! # use hamcrest2::prelude::*;
//! assert_that!(123usize, type_of::<usize>());
//! assert_that!("test", type_of::<&str>());
//! ```
//!
//! ### matches_regex
//!
//! ```
//! # use hamcrest2::prelude::*;
//! assert_that!("1234", matches_regex(r"\d"));
//! assert_that!("abc", does_not(match_regex(r"\d")));
//! ```
//!
//! ## Numerical Matchers
//!
//! ### close_to
//!
//! ```
//! # use hamcrest2::prelude::*;
//! assert_that!(1e-40f32, close_to(0.0, 0.01));
//! assert_that!(1e-40f32, not(close_to(0.0, 0.000001)));
//! ```
//!
//! ## Filesystem Matchers
//!
//! ### path_exists, file_exists, dir_exists
//!
//! ```
//! # use hamcrest2::prelude::*;
//! # pub use std::path::Path;
//! let path = Path::new("./README.md");
//! assert_that!(path, path_exists());
//! assert_that!(path, file_exists());
//! assert_that!(path, not(dir_exists()));
//! ```
//!
//! ## Option and Result
//!
//! ### has
//!
//! ```
//! # use hamcrest2::prelude::*;
//! let var: Option<i8> = Some(5);
//! assert_that!(var, has(5));
//!
//! let var: Result<i8, String> = Ok(5);
//! assert_that!(var, has(5));
//! ```
//! ### ok
//!
//! ```
//! # use hamcrest2::prelude::*;
//! let var: Result<i8, String> = Ok(5);
//! assert_that!(var, ok());
//!
//! assert_that!(Ok(5), ok::<i8, String>());
//!
//! let var: Result<i8, String> = Err("bad!".to_string());
//! assert_that!(var, not(ok()));
//! ```
//!
//! ### err
//!
//! ```
//! # use hamcrest2::prelude::*;
//! let var: Result<i8, String> = Err("bad!".to_string());
//! assert_that!(var, err());
//!
//! assert_that!(Err("bad!".to_string()), err::<i8, String>());
//!
//! let var: Result<i8, String> = Ok(5);
//! assert_that!(var, not(err()));
//! ```
//!
//! ### some
//!
//! ```
//! # use hamcrest2::prelude::*;
//! let var: Option<i8> = Some(5);
//! assert_that!(var, some());
//!
//! assert_that!(Some(1), some::<u8>());
//!
//! let var: Option<i8> = None;
//! assert_that!(var, not(some()));
//! ```
//!
//! ### none
//!
//! ```
//! # use hamcrest2::prelude::*;
//! let var: Option<i8> = None;
//! assert_that!(var, none());
//!
//! assert_that!(None, none::<u8>());
//! assert_that!(Some(1), not(none::<u8>()));
//! ```
//!
//! ## Collection Matchers
//!
//! ### contains, contains exactly, contains in order
//!
//! ```
//! # use hamcrest2::prelude::*;
//! assert_that!(&vec!(1, 2, 3), contains(vec!(1, 2)));
//! assert_that!(&vec!(1, 2, 3), not(contains(vec!(4))));
//!
//! assert_that!(&vec!(1, 2, 3), contains(vec!(1, 2, 3)).exactly());
//! assert_that!(&vec!(1, 2, 3), not(contains(vec!(1, 2)).exactly()));
//!
//! assert_that!(&vec!(1, 2, 3), contains(vec!(1, 2)).in_order());
//! assert_that!(&vec!(1, 2, 3), not(contains(vec!(1, 3)).in_order()));
//! ```
//!
//! ## len
//! ```
//! # use hamcrest2::prelude::*;
//! assert_that!(&vec!(1, 2, 3), len(3));
//! assert_that!(&vec!(1, 2, 3), not(len(4)));
//! ```
//!
//! ## empty
//! ```
//! # use hamcrest2::prelude::*;
//! assert_that!(&Vec::<i32>::new(), empty());
//! assert_that!(&vec![1, 2, 3], not(empty()));
//! ```
//!
//! ## Compound Matchers
//!
//! ### all
//!
//! ```
//! # use hamcrest2::prelude::*;
//! assert_that!(4, all!(lt(5), gt(3)));  // also and!()
//! assert_that!(
//!     &vec![1, 2, 3],
//!     all!(contains(vec![1, 2]), not(contains(vec![4])))
//! );
//! ```
//!
//! ### any
//!
//! ```
//! # use hamcrest2::prelude::*;
//! assert_that!(4, any!(less_than(2), greater_than(3)));  // also or!()
//! assert_that!(
//!     &vec![1, 2, 3],
//!     any!(contains(vec![1, 2, 5]), not(contains(vec![4])))
//! );
//! ```
//!
//! ## Misc Matchers
//!
//! ### is(bool)
//!
//! ```
//! # use hamcrest2::prelude::*;
//! assert_that!(true, is(true));
//! assert_that!(false, is(false));
//! ```
//!
//! ### anything
//!
//! ```
//! # use hamcrest2::prelude::*;
//! assert_that!(42, anything());
//! assert_that!("test", is(anything()));
//! ```

extern crate num;
extern crate regex;

pub use crate::prelude::*;

#[macro_export]
macro_rules! assert_that {
  ($actual:expr, $matcher:expr) => {{
    // The separate statement is necessary to keep the compiler happy.
    let m = $matcher;
    match m.matches($actual) {
      Ok(_) => {}
      Err(mismatch) => {
        // The panic macro produces the correct file and line number
        // when used in a macro like this, i.e. it's the line where
        // the macro was originally written.
        panic!("\nExpected: {}\n    but: {}", m, mismatch);
      }
    }
  }};
}

pub mod core;
pub mod matchers;
mod utils;
pub mod prelude {
  pub use crate::all;
  #[deprecated(since = "0.2.0", note = "Use all() instead")]
  pub use crate::all as all_of;
  pub use crate::any;
  #[deprecated(since = "0.2.0", note = "Use any() instead")]
  pub use crate::any as any_of;
  #[deprecated(since = "0.2.0", note = "Use any() instead")]
  pub use crate::any as or;
  pub use crate::assert_that;
  #[allow(deprecated)]
  pub use crate::core::assert_that as assert_that_fn;
  pub use crate::core::Matcher as HamcrestMatcher;
  pub use crate::matchers::anything::anything;
  pub use crate::matchers::close_to::close_to;
  pub use crate::matchers::compared_to::greater_than;
  pub use crate::matchers::compared_to::greater_than as gt;
  pub use crate::matchers::compared_to::greater_than_or_equal_to;
  pub use crate::matchers::compared_to::greater_than_or_equal_to as geq;
  pub use crate::matchers::compared_to::less_than;
  pub use crate::matchers::compared_to::less_than as lt;
  pub use crate::matchers::compared_to::less_than_or_equal_to;
  pub use crate::matchers::compared_to::less_than_or_equal_to as leq;
  pub use crate::matchers::contains::contains;
  pub use crate::matchers::empty::empty;
  pub use crate::matchers::equal_to::equal_to;
  pub use crate::matchers::equal_to::equal_to as eq;
  pub use crate::matchers::err::err;
  pub use crate::matchers::has::has;
  pub use crate::matchers::is::is;
  pub use crate::matchers::is::is_not as does_not;
  pub use crate::matchers::is::is_not as not;
  pub use crate::matchers::is::is_not;
  pub use crate::matchers::len::len;
  #[deprecated(since = "0.2.0", note = "Use len() instead")]
  pub use crate::matchers::len::len as of_len;
  pub use crate::matchers::none::none;
  pub use crate::matchers::ok::ok;
  pub use crate::matchers::path_exists::dir_exists;
  #[deprecated(since = "0.2.0", note = "Use dir_exists() instead")]
  pub use crate::matchers::path_exists::dir_exists as existing_dir;
  pub use crate::matchers::path_exists::file_exists;
  #[deprecated(since = "0.2.0", note = "Use file_exists() instead")]
  pub use crate::matchers::path_exists::file_exists as existing_file;
  pub use crate::matchers::path_exists::path_exists;
  #[deprecated(since = "0.2.0", note = "Use path_exists() instead")]
  pub use crate::matchers::path_exists::path_exists as existing_path;
  pub use crate::matchers::regex::matches_regex as match_regex;
  pub use crate::matchers::regex::matches_regex;
  pub use crate::matchers::some::some;
  pub use crate::matchers::type_of::type_of;
}
