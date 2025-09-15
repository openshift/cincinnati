// Copyright 2016 Urban Hafner
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

extern crate hamcrest2;

mod path_exists {
  pub use hamcrest2::prelude::*;
  pub use std::env;
  pub use std::path::Path;
  pub use std::path::PathBuf;

  #[test]
  fn deprecated_names() {
    let path = path(env::var("TEST_EXISTS_FILE"), "./README.md");
    assert_that!(&path, is(existing_path()));
    assert_that!(&path, is(existing_file()));
    assert_that!(&path, not(existing_dir()));
  }

  #[test]
  fn a_file_exists() {
    let path = path(env::var("TEST_EXISTS_FILE"), "./README.md");
    assert_that!(&path, is(path_exists()));
    assert_that!(&path, is(file_exists()));
    assert_that!(&path, not(dir_exists()));
  }

  #[test]
  fn a_dir_exists() {
    let path = path(env::var("TEST_EXISTS_DIR"), "./target");
    assert_that!(&path, is(path_exists()));
    assert_that!(&path, is(dir_exists()));
    assert_that!(&path, not(file_exists()));
  }

  #[test]
  fn a_nonpath_exists() {
    let path = path(env::var("TEST_EXISTS_NONE"), "./zomg.txt");
    assert_that!(&path, not(path_exists()));
    assert_that!(&path, not(file_exists()));
    assert_that!(&path, not(dir_exists()));
  }

  pub fn path(path: Result<String, env::VarError>, default: &str) -> PathBuf {
    Path::new(&path.unwrap_or_else(|_| default.to_string())).to_owned()
  }
}
