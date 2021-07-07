#[macro_use]
extern crate prettytable;

pub mod basic;
pub mod format_table;
pub mod lcs;
pub mod text;

pub use crate::basic::diff_slice;
pub use crate::text::{diff_chars, diff_lines, diff_words};
