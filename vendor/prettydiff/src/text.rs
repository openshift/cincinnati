//! Utils for diff text
pub use ansi_term::Style;

use crate::basic;
use crate::format_table;
use ansi_term::{Colour};
use prettytable::{Cell, Row};
use std::fmt;

pub struct StringSplitIter<'a, F>
where
    F: Fn(char) -> bool,
{
    last: usize,
    text: &'a str,
    matched: Option<&'a str>,
    iter: std::str::MatchIndices<'a, F>,
}

impl<'a, F> Iterator for StringSplitIter<'a, F>
where
    F: Fn(char) -> bool,
{
    type Item = &'a str;
    fn next(&mut self) -> Option<Self::Item> {
        if let Some(m) = self.matched {
            self.matched = None;
            Some(m)
        } else if let Some((idx, matched)) = self.iter.next() {
            let res = if self.last != idx {
                self.matched = Some(matched);
                &self.text[self.last..idx]
            } else {
                matched
            };
            self.last = idx + matched.len();
            Some(res)
        } else if self.last < self.text.len() {
            let res = &self.text[self.last..];
            self.last = self.text.len();
            Some(res)
        } else {
            None
        }
    }
}

pub fn collect_strings<T: ToString>(it: impl Iterator<Item = T>) -> Vec<String> {
    it.map(|s| s.to_string()).collect::<Vec<String>>()
}

/// Split string by clousure (Fn(char)->bool) keeping delemiters
pub fn split_by_char_fn<F>(text: &'_ str, pat: F) -> StringSplitIter<'_, F>
where
    F: Fn(char) -> bool,
{
    StringSplitIter {
        last: 0,
        text,
        matched: None,
        iter: text.match_indices(pat),
    }
}

/// Split string by non-alphanumeric characters keeping delemiters
pub fn split_words(text: &str) -> impl Iterator<Item = &str> {
    split_by_char_fn(text, |c: char| !c.is_alphanumeric())
}

/// Container for inline text diff result. Can be pretty-printed by Display trait.
#[derive(Debug, PartialEq)]
pub struct InlineChangeset<'a> {
    old: Vec<&'a str>,
    new: Vec<&'a str>,
    separator: &'a str,
    highlight_whitespace: bool,
    insert_style: Style,
    insert_whitespace_style: Style,
    remove_style: Style,
    remove_whitespace_style: Style,
}

impl<'a> InlineChangeset<'a> {
    pub fn new(old: Vec<&'a str>, new: Vec<&'a str>) -> InlineChangeset<'a> {
        InlineChangeset {
            old,
            new,
            separator: "",
            highlight_whitespace: true,
            insert_style: Colour::Green.normal(),
            insert_whitespace_style: Colour::White.on(Colour::Green),
            remove_style: Colour::Red.strikethrough(),
            remove_whitespace_style: Colour::White.on(Colour::Red),
        }
    }
    /// Highlight whitespaces in case of insert/remove?
    pub fn set_highlight_whitespace(mut self, val: bool) -> Self {
        self.highlight_whitespace = val;
        self
    }

    /// Style of inserted text
    pub fn set_insert_style(mut self, val: Style) -> Self {
        self.insert_style = val;
        self
    }

    /// Style of inserted whitespace
    pub fn set_insert_whitespace_style(mut self, val: Style) -> Self {
        self.insert_whitespace_style = val;
        self
    }

    /// Style of removed text
    pub fn set_remove_style(mut self, val: Style) -> Self {
        self.remove_style = val;
        self
    }

    /// Style of removed whitespace
    pub fn set_remove_whitespace_style(mut self, val: Style) -> Self {
        self.remove_whitespace_style = val;
        self
    }

    /// Set output separator
    pub fn set_separator(mut self, val: &'a str) -> Self {
        self.separator = val;
        self
    }

    /// Returns Vec of changes
    pub fn diff(&self) -> Vec<basic::DiffOp<'a, &str>> {
        basic::diff(&self.old, &self.new)
    }

    fn apply_style(&self, style: Style, whitespace_style: Style, a: &[&str]) -> String {
        let s = a.join(self.separator);
        if self.highlight_whitespace {
            collect_strings(split_by_char_fn(&s, |c| c.is_whitespace()).map(|s| {
                let style = if s
                    .chars()
                    .next()
                    .map_or_else(|| false, |c| c.is_whitespace())
                {
                    whitespace_style
                } else {
                    style
                };
                style.paint(s)
            }))
            .join("")
        } else {
            style.paint(s).to_string()
        }
    }

    fn remove_color(&self, a: &[&str]) -> String {
        self.apply_style(self.remove_style, self.remove_whitespace_style, a)
    }

    fn insert_color(&self, a: &[&str]) -> String {
        self.apply_style(self.insert_style, self.insert_whitespace_style, a)
    }
    /// Returns formatted string with colors
    pub fn format(&self) -> String {
        let diff = self.diff();
        let mut out: Vec<String> = Vec::with_capacity(diff.len());
        for op in diff {
            match op {
                basic::DiffOp::Equal(a) => out.push(a.join(self.separator)),
                basic::DiffOp::Insert(a) => out.push(self.insert_color(a)),
                basic::DiffOp::Remove(a) => out.push(self.remove_color(a)),
                basic::DiffOp::Replace(a, b) => {
                    out.push(self.remove_color(a));
                    out.push(self.insert_color(b));
                }
            }
        }
        out.join(self.separator)
    }
}

impl<'a> fmt::Display for InlineChangeset<'a> {
    fn fmt(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        write!(formatter, "{}", self.format())
    }
}

pub fn diff_chars<'a>(old: &'a str, new: &'a str) -> InlineChangeset<'a> {
    let old: Vec<&str> = old.split("").filter(|&i| i != "").collect();
    let new: Vec<&str> = new.split("").filter(|&i| i != "").collect();

    InlineChangeset::new(old, new)
}

/// Diff two strings by words (contiguous)
pub fn diff_words<'a>(old: &'a str, new: &'a str) -> InlineChangeset<'a> {
    InlineChangeset::new(split_words(old).collect(), split_words(new).collect())
}

fn color_multilines(color: Colour, s: &str) -> String {
    collect_strings(s.split('\n').map(|i| color.paint(i))).join("\n")
}

/// Container for line-by-line text diff result. Can be pretty-printed by Display trait.
#[derive(Debug, PartialEq, Eq)]
pub struct LineChangeset<'a> {
    old: Vec<&'a str>,
    new: Vec<&'a str>,

    names: Option<(&'a str, &'a str)>,
    diff_only: bool,
    show_lines: bool,
    trim_new_lines: bool,
    aling_new_lines: bool,
}

impl<'a> LineChangeset<'a> {
    pub fn new(old: Vec<&'a str>, new: Vec<&'a str>) -> LineChangeset<'a> {
        LineChangeset {
            old,
            new,
            names: None,
            diff_only: false,
            show_lines: true,
            trim_new_lines: true,
            aling_new_lines: false,
        }
    }

    /// Sets names for side-by-side diff
    pub fn names(mut self, old: &'a str, new: &'a str) -> Self {
        self.names = Some((old, new));
        self
    }
    /// Show only differences for side-by-side diff
    pub fn set_diff_only(mut self, val: bool) -> Self {
        self.diff_only = val;
        self
    }
    /// Show lines in side-by-side diff
    pub fn set_show_lines(mut self, val: bool) -> Self {
        self.show_lines = val;
        self
    }
    /// Trim new lines in side-by-side diff
    pub fn set_trim_new_lines(mut self, val: bool) -> Self {
        self.trim_new_lines = val;
        self
    }
    /// Align new lines inside diff
    pub fn set_aling_new_lines(mut self, val: bool) -> Self {
        self.aling_new_lines = val;
        self
    }
    /// Returns Vec of changes
    pub fn diff(&self) -> Vec<basic::DiffOp<'a, &str>> {
        basic::diff(&self.old, &self.new)
    }

    fn prettytable_process(&self, a: &[&str], color: Option<Colour>) -> (String, usize) {
        let mut start = 0;
        let mut stop = a.len();
        if self.trim_new_lines {
            for (index, element) in a.iter().enumerate() {
                if *element != "" {
                    break;
                }
                start = index + 1;
            }
            for (index, element) in a[start..stop].iter().rev().enumerate() {
                if *element != "" {
                    break;
                }
                stop = index;
            }
        }
        let out = &a[start..stop];
        if let Some(color) = color {
            (
                collect_strings(out.iter().map(|i| color.paint(*i).to_string()))
                    .join("\n")
                    .replace("\t", "    "),
                start,
            )
        } else {
            (out.join("\n").replace("\t", "    "), start)
        }
    }
    fn prettytable_process_replace(
        &self,
        old: &[&str],
        new: &[&str],
    ) -> ((String, String), (usize, usize)) {
        let (old, old_offset) = self.prettytable_process(old, None);
        let (new, new_offset) = self.prettytable_process(new, None);

        let mut old_out = String::new();
        let mut new_out = String::new();

        for op in diff_words(&old, &new).diff() {
            match op {
                basic::DiffOp::Equal(a) => {
                    old_out.push_str(&a.join(""));
                    new_out.push_str(&a.join(""));
                }
                basic::DiffOp::Insert(a) => {
                    new_out.push_str(&color_multilines(Colour::Green, &a.join("")));
                }
                basic::DiffOp::Remove(a) => {
                    old_out.push_str(&color_multilines(Colour::Red, &a.join("")));
                }
                basic::DiffOp::Replace(a, b) => {
                    old_out.push_str(&color_multilines(Colour::Red, &a.join("")));
                    new_out.push_str(&color_multilines(Colour::Green, &b.join("")));
                }
            }
        }

        ((old_out, new_out), (old_offset, new_offset))
    }
    /// Prints side-by-side diff in table
    pub fn prettytable(&self) {
        let mut table = format_table::new();
        if let Some((old, new)) = &self.names {
            let mut header = vec![];
            if self.show_lines {
                header.push(Cell::new(""));
            }
            header.push(Cell::new(&Colour::Cyan.paint(old.to_string()).to_string()));
            if self.show_lines {
                header.push(Cell::new(""));
            }
            header.push(Cell::new(&Colour::Cyan.paint(new.to_string()).to_string()));
            table.set_titles(Row::new(header));
        }
        let mut old_lines = 1;
        let mut new_lines = 1;
        let mut out: Vec<(usize, String, usize, String)> = Vec::new();
        for op in &self.diff() {
            match op {
                basic::DiffOp::Equal(a) => {
                    let (old, offset) = self.prettytable_process(a, None);
                    if !self.diff_only {
                        out.push((old_lines + offset, old.clone(), new_lines + offset, old));
                    }
                    old_lines += a.len();
                    new_lines += a.len();
                }
                basic::DiffOp::Insert(a) => {
                    let (new, offset) = self.prettytable_process(a, Some(Colour::Green));
                    out.push((old_lines, "".to_string(), new_lines + offset, new));
                    new_lines += a.len();
                }
                basic::DiffOp::Remove(a) => {
                    let (old, offset) = self.prettytable_process(a, Some(Colour::Red));
                    out.push((old_lines + offset, old, new_lines, "".to_string()));
                    old_lines += a.len();
                }
                basic::DiffOp::Replace(a, b) => {
                    let ((old, new), (old_offset, new_offset)) =
                        self.prettytable_process_replace(a, b);
                    out.push((old_lines + old_offset, old, new_lines + new_offset, new));
                    old_lines += a.len();
                    new_lines += b.len();
                }
            };
        }
        for (old_lines, old, new_lines, new) in out {
            if self.trim_new_lines && old.trim() == "" && new.trim() == "" {
                continue;
            }
            if self.show_lines {
                table.add_row(row![old_lines, old, new_lines, new]);
            } else {
                table.add_row(row![old, new]);
            }
        }
        table.printstd();
    }

    fn remove_color(&self, a: &[&str]) -> String {
        Colour::Red.strikethrough().paint(a.join("\n")).to_string()
    }

    fn insert_color(&self, a: &[&str]) -> String {
        Colour::Green.paint(a.join("\n")).to_string()
    }

    pub fn format(&self) -> String {
        let diff = self.diff();
        let mut out: Vec<String> = Vec::with_capacity(diff.len());
        for op in diff {
            match op {
                basic::DiffOp::Equal(a) => out.push(a.join("\n")),
                basic::DiffOp::Insert(a) => out.push(self.insert_color(a)),
                basic::DiffOp::Remove(a) => out.push(self.remove_color(a)),
                basic::DiffOp::Replace(a, b) => {
                    out.push(self.remove_color(a));
                    out.push(self.insert_color(b));
                }
            }
        }
        out.join("\n")
    }
}

impl<'a> fmt::Display for LineChangeset<'a> {
    fn fmt(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        write!(formatter, "{}", self.format())
    }
}

pub fn diff_lines<'a>(old: &'a str, new: &'a str) -> LineChangeset<'a> {
    let old: Vec<&str> = old.lines().collect();
    let new: Vec<&str> = new.lines().collect();

    LineChangeset::new(old, new)
}

fn _test_splitter_basic(text: &str, exp: &[&str]) {
    let res = collect_strings(
        split_by_char_fn(&text, |c: char| c.is_whitespace()).map(|s| s.to_string()),
    );
    assert_eq!(res, exp)
}

#[test]
fn test_splitter() {
    _test_splitter_basic(
        "  blah test2 test3  ",
        &[" ", " ", "blah", " ", "test2", " ", "test3", " ", " "],
    );
    _test_splitter_basic(
        "\tblah test2 test3  ",
        &["\t", "blah", " ", "test2", " ", "test3", " ", " "],
    );
    _test_splitter_basic(
        "\tblah test2 test3  t",
        &["\t", "blah", " ", "test2", " ", "test3", " ", " ", "t"],
    );
    _test_splitter_basic(
        "\tblah test2 test3  tt",
        &["\t", "blah", " ", "test2", " ", "test3", " ", " ", "tt"],
    );
}

#[test]
fn test_basic() {
    println!("diff_chars: {}", diff_chars("abefcd", "zadqwc"));
    println!(
        "diff_chars: {}",
        diff_chars(
            "The quick brown fox jumps over the lazy dog",
            "The quick brown dog leaps over the lazy cat"
        )
    );
    println!(
        "diff_chars: {}",
        diff_chars(
            "The red brown fox jumped over the rolling log",
            "The brown spotted fox leaped over the rolling log"
        )
    );
    println!(
        "diff_chars: {}",
        diff_chars(
            "The red    brown fox jumped over the rolling log",
            "The brown spotted fox leaped over the rolling log"
        )
        .set_highlight_whitespace(true)
    );
    println!(
        "diff_words: {}",
        diff_words(
            "The red brown fox jumped over the rolling log",
            "The brown spotted fox leaped over the rolling log"
        )
    );
    println!(
        "diff_words: {}",
        diff_words(
            "The quick brown fox jumps over the lazy dog",
            "The quick, brown dog leaps over the lazy cat"
        )
    );
}

#[test]
fn test_split_words() {
    assert_eq!(
        collect_strings(split_words("Hello World")),
        ["Hello", " ", "World"]
    );
    assert_eq!(
        collect_strings(split_words("Hello😋World")),
        ["Hello", "😋", "World"]
    );
    assert_eq!(
        collect_strings(split_words(
            "The red brown fox\tjumped, over the rolling log"
        )),
        [
            "The", " ", "red", " ", "brown", " ", "fox", "\t", "jumped", ",", " ", "over", " ",
            "the", " ", "rolling", " ", "log"
        ]
    );
}

#[test]
fn test_diff_lines() {
    let code1_a = r#"
void func1() {
    x += 1
}

void func2() {
    x += 2
}
    "#;
    let code1_b = r#"
void func1(a: u32) {
    x += 1
}

void functhreehalves() {
    x += 1.5
}

void func2() {
    x += 2
}

void func3(){}
"#;
    println!("diff_lines:");
    println!("{}", diff_lines(code1_a, code1_b));
    println!("====");
    diff_lines(code1_a, code1_b)
        .names("left", "right")
        .set_aling_new_lines(true)
        .prettytable();
}

fn _test_colors(changeset: &InlineChangeset, exp: &[(Option<Style>, &str)]) {
    let color_s: String = collect_strings(exp.iter().map(|(style_opt, s)| {
        if let Some(style) = style_opt {
            style.paint(s.to_string()).to_string()
        } else {
            s.to_string()
        }
    }))
    .join("");
    assert_eq!(format!("{}", changeset), color_s);
}

#[test]
fn test_diff_words_issue_1() {
    let insert_style = Colour::Green.normal();
    let insert_whitespace_style = Colour::White.on(Colour::Green);
    let remove_style = Colour::Red.strikethrough();
    let remove_whitespace_style = Colour::White.on(Colour::Red);
    let d1 = diff_words(
        "und meine Unschuld beweisen!",
        "und ich werde meine Unschuld beweisen!",
    )
    .set_insert_style(insert_style)
    .set_insert_whitespace_style(insert_whitespace_style)
    .set_remove_style(remove_style)
    .set_remove_whitespace_style(remove_whitespace_style);

    println!("diff_words: {} {:?}", d1, d1.diff());

    _test_colors(
        &d1,
        &[
            (None, "und "),
            (Some(insert_style), "ich"),
            (Some(insert_whitespace_style), " "),
            (Some(insert_style), "werde"),
            (Some(insert_whitespace_style), " "),
            (None, "meine Unschuld beweisen!"),
        ],
    );
    _test_colors(
        &d1.set_highlight_whitespace(false),
        &[
            (None, "und "),
            (Some(insert_style), "ich werde "),
            (None, "meine Unschuld beweisen!"),
        ],
    );
    let d2 = diff_words(
        "Campaignings aus dem Ausland gegen meine Person ausfindig",
        "Campaignings ausfindig",
    );
    println!("diff_words: {} {:?}", d2, d2.diff());
    _test_colors(
        &d2,
        &[
            (None, "Campaignings "),
            (Some(remove_style), "aus"),
            (Some(remove_whitespace_style), " "),
            (Some(remove_style), "dem"),
            (Some(remove_whitespace_style), " "),
            (Some(remove_style), "Ausland"),
            (Some(remove_whitespace_style), " "),
            (Some(remove_style), "gegen"),
            (Some(remove_whitespace_style), " "),
            (Some(remove_style), "meine"),
            (Some(remove_whitespace_style), " "),
            (Some(remove_style), "Person"),
            (Some(remove_whitespace_style), " "),
            (None, "ausfindig"),
        ],
    );
    let d3 = diff_words("des kriminellen Videos", "des kriminell erstellten Videos");
    println!("diff_words: {} {:?}", d3, d3.diff());
    _test_colors(
        &d3,
        &[
            (None, "des "),
            (Some(remove_style), "kriminellen"),
            (Some(insert_style), "kriminell"),
            (None, " "),
            (Some(insert_style), "erstellten"),
            (Some(insert_whitespace_style), " "),
            (None, "Videos"),
        ],
    );
}
