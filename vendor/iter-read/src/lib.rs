// Copyright (c) 2015-2023 Georg Brandl.  Licensed under the Apache License,
// Version 2.0 <LICENSE-APACHE or http://www.apache.org/licenses/LICENSE-2.0>
// or the MIT license <LICENSE-MIT or http://opensource.org/licenses/MIT>, at
// your option. This file may not be copied, modified, or distributed except
// according to those terms.

//! A small crate that provides an adapter to Read from an iterator over bytes.
//! This is useful if you want to use an API that takes a `Read` instance, but
//! want to feed it data from an iterator.
//!
//! Example:
//!
//! ```
//! use std::io::Read;
//! use iter_read::IterRead;
//!
//! let source = vec![1, 2, 7, 42, 123];
//! let mut reader = IterRead::new(source.iter());
//! let mut buf = vec![0; 3];
//! reader.read_exact(&mut buf).unwrap();
//! assert_eq!(buf, b"\x01\x02\x07");
//! ```

#![cfg_attr(feature = "unstable", feature(test))]

use std::io::{Read, Result};
use std::iter::Fuse;


/// Trait that should be implemented for any type which can be used in an
/// iterator given to `IterRead`.
pub trait IterReadItem {
    /// Represents the type of buffer that the adapter will use.
    /// Can be `()` if no buffer is required.
    type Buffer: Default;

    /// Implements `Read::read()` for the specific type Self.
    fn read<I: Iterator<Item=Self>>(target: &mut [u8], it: &mut I, buf: &mut Self::Buffer)
                                    -> Result<usize> where Self: Sized;
}

/// An adapter that allows treating iterators of bytes (and other types that
/// implement `IterReadItem`) as a `Read`.
///
/// `IterReadItem` is implemented for `u8`, `&u8`, `Vec<u8>`, `[u8; N]` and its
/// borrowed variants as well as `String` and its borrowed variants.  It is also
/// implemented for all iterators that take a `Result` type whose `Ok` value is
/// an `IterReadItem` and whose `Err` value can be converted to an
/// `std::io::Error` with `into()`.
///
/// For inner types other than `u8` the adapter might need to buffer some
/// contents of the iterator.
pub struct IterRead<E: IterReadItem, I: Iterator<Item=E>> {
    iter: Fuse<I>,
    buf: E::Buffer,
}

impl<E: IterReadItem, I: Iterator<Item=E>> IterRead<E, I> {
    /// Create a new `IterRead` which will read from the specified `Iterator`.
    pub fn new(iter: I) -> IterRead<E, I> {
        IterRead { iter: iter.fuse(), buf: Default::default() }
    }

    /// Unwrap the inner iterator.  If the adapter uses buffering, the contents
    /// of the buffer are lost.
    pub fn into_inner(self) -> Fuse<I> {
        self.iter
    }
}

impl<E: IterReadItem, I: Iterator<Item=E>> Read for IterRead<E, I> {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize> {
        E::read(buf, &mut self.iter, &mut self.buf)
    }
}

mod impls;

#[cfg(test)]
mod tests;
