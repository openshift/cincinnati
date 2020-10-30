//! A library providing `SourceFiles`, a concatenated list of files with information for resolving
//! points and spans.

use std::fs;
use std::io::{self, BufRead};
use std::path::Path;

#[derive(Default)]
/// A concatenated string of files, with sourcemap information.
#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct SourceFile {
    /// The full contents of all the files
    pub contents: String,
    /// The names of the files (same length as `files`).
    file_names: Vec<String>,
    /// The number of lines in each file.
    file_lines: Vec<usize>,
    /// The length of each line in all source files
    line_lengths: Vec<usize>,
}

impl SourceFile {
    /// Concatenate a file to the end of `contents`, and record info needed to resolve spans.
    ///
    /// If the last line doesn't end with a newline character, it will still be a 'line' for the
    /// purposes of this calcuation.
    ///
    /// Consumes `self` because the structure would be inconsistent after an error.
    pub fn add_file(mut self, filename: impl AsRef<Path>) -> Result<Self, io::Error> {
        let filename = filename.as_ref();
        let mut file = io::BufReader::new(fs::File::open(filename)?);

        // We should skip this file if it is completely empty.
        let line_len = file.read_line(&mut self.contents)?;
        if line_len == 0 {
            return Ok(self);
        }
        self.line_lengths.push(line_len);

        let mut num_lines = 1; // We already got one above.
        loop {
            let line_len = file.read_line(&mut self.contents)?;
            if line_len == 0 { //EOF
                break;
            }
            self.line_lengths.push(line_len);
            num_lines += 1;
        }

        // Record the name
        self.file_names.push(format!("{}", filename.display()));
        // Record the number of lines
        self.file_lines.push(num_lines);
        Ok(self)
    }

    /// Get the file, line, and col position of a byte offset.
    ///
    /// # Panics
    ///
    /// This function will panic if `offset` is not on a character boundary.
    pub fn resolve_offset<'a>(&'a self, offset: usize) -> Option<Position<'a>> {
        // If there isn't a single line, always return None.
        let mut line_acc = *self.line_lengths.get(0)?;
        let mut line_idx = 0;
        while line_acc <= offset {
            line_idx += 1;
            // If we have exhaused all the lines, return None
            line_acc += *self.line_lengths.get(line_idx)?;
        }
        // Go back to the start of the line (for working out the column).
        line_acc -= self.line_lengths[line_idx];

        // Can't panic - if we have a line we have a file
        let mut file_acc = self.file_lines[0];
        let mut file_idx = 0;
        while file_acc <= line_idx {
            file_idx += 1;
            file_acc += self.file_lines[file_idx];
        }
        // Go back to the start of the file (for working out the line).
        file_acc -= self.file_lines[file_idx];

        Some(Position::new(&self.file_names[file_idx], line_idx - file_acc, offset - line_acc))
    }

    /// Get the file, line, and col position of each end of a span
    // TODO this could be more efficient by using the fact that end is after (and probably near to)
    // start.
    pub fn resolve_offset_span<'a>(&'a self, start: usize, end: usize) -> Option<Span<'a>> {
        if end < start {
            return None;
        }
        Some(Span {
            start: self.resolve_offset(start)?,
            end: self.resolve_offset(end)?,
        })
    }
}

/// A position in a source file.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct Position<'a> {
    /// Name of the file the position is in.
    pub filename: &'a str,
    /// 0-indexed line number of position.
    pub line: usize,
    /// 0-indexed column number of position.
    pub col: usize,
}

impl<'a> Position<'a> {
    /// Constructor for tests.
    fn new(filename: &'a str, line: usize, col: usize) -> Position<'a> {
        Position { filename: filename.as_ref(), line, col }
    }
}

/// A span in a source file
#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct Span<'a> {
    pub start: Position<'a>,
    pub end: Position<'a>,
}

#[cfg(test)]
mod tests {
    extern crate tempfile;

    use super::{SourceFile, Position, Span};
    use self::tempfile::NamedTempFile;
    use std::io::Write;

    #[test]
    fn empty() {
        let sourcefile = SourceFile::default();
        assert!(sourcefile.resolve_offset(0).is_none());
    }

    #[test]
    fn smoke() {
        test_files(
            &["A file with\ntwo lines.\n",
              "Another file with\ntwo more lines.\n",
            ],
            &[(0, (0, 0, 0)), // start
              (5, (0, 0, 5)), // last char first line first file
              (11, (0, 0, 11)), // first char second line first file
              (12, (0, 1, 0)), // ..
              (13, (0, 1, 1)),
              (13, (0, 1, 1)),
              (22, (0, 1, 10)),
              (23, (1, 0, 0)),
              (24, (1, 0, 1)),
              (40, (1, 0, 17)),
              (41, (1, 1, 0)),
              (42, (1, 1, 1)),
              (56, (1, 1, 15)),
              //(57, (1, 1, 16)), // should panic
            ],
            &[((0, 5), (0, 0, 0), (0, 0, 5)),
            ]
        )
    }

    fn test_files<'a>(files: &[impl AsRef<str>],
                      offset_tests: &[(usize,
                                       (usize, usize, usize))],
                      offset_span_tests: &[((usize, usize),
                                            (usize, usize, usize),
                                            (usize, usize, usize))])
    {
        let mut sourcefile = SourceFile::default();
        let mut file_handles = Vec::new(); // don't clean me up please
        for contents in files {
            let mut file = NamedTempFile::new().unwrap();
            write!(file, "{}", contents.as_ref()).unwrap();
            sourcefile = sourcefile.add_file(file.path()).unwrap();
            file_handles.push(file);
        }

        for &(offset, (file_idx, line, col)) in offset_tests {
            let filename = format!("{}", file_handles[file_idx].path().display());
            let pos = sourcefile.resolve_offset(offset);
            assert!(pos.is_some());
            assert_eq!(pos.unwrap(), Position::new(&filename, line, col));
        }

        for &((start, end),
              (file_idx_start, line_start, col_start),
              (file_idx_end, line_end, col_end)) in offset_span_tests
        {
            let start_filename = format!("{}", file_handles[file_idx_start].path().display());
            let end_filename = format!("{}", file_handles[file_idx_end].path().display());
            assert_eq!(sourcefile.resolve_offset_span(start, end).unwrap(),
                       Span {
                           start: Position::new(&start_filename, line_start, col_start),
                           end: Position::new(&end_filename, line_end, col_end),
                       });
        }
    }
}
