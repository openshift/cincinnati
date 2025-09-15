// Copyright (c) 2015-2023 Georg Brandl.  Licensed under the Apache License,
// Version 2.0 <LICENSE-APACHE or http://www.apache.org/licenses/LICENSE-2.0>
// or the MIT license <LICENSE-MIT or http://opensource.org/licenses/MIT>, at
// your option. This file may not be copied, modified, or distributed except
// according to those terms.

use std::io;

use crate::IterReadItem;


macro_rules! impl_byte {
    ($el_ty:ty, $($deref:tt)*) => {
        impl<'a> IterReadItem for $el_ty {
            impl_byte!(@inner [$($deref)*] []);
        }

        impl<'a, E> IterReadItem for Result<$el_ty, E> where io::Error: From<E> {
            impl_byte!(@inner [$($deref)*] [?]);
        }
    };
    (@inner [$($deref:tt)*] [$($try:tt)*]) => {
        type Buffer = ();

        fn read<I: Iterator<Item=Self>>(target: &mut [u8], it: &mut I,
                                        _: &mut ()) -> io::Result<usize> {
            let mut len = 0;
            for (slot, elt) in target.iter_mut().zip(it) {
                *slot = $($deref)* elt $($try)*;
                len += 1;
            }
            Ok(len)
        }
    }
}

pub struct Buf<T> {
    bytes: T,
    consumed: usize,
}

macro_rules! impl_buf_default {
    ($ty:ty) => {
        impl<'a> Default for Buf<$ty> {
            fn default() -> Self {
                Buf { bytes: Default::default(), consumed: 0 }
            }
        }
    }
}

impl_buf_default!(&'a [u8]);
impl_buf_default!(Vec<u8>);

impl<const N: usize> Default for Buf<[u8; N]> {
    fn default() -> Self {
        // The default needs to look like "everything is consumed",
        // and the array has length N, so consumed needs to be N too.
        Buf { bytes: [0; N], consumed: N }
    }
}

macro_rules! impl_slice_like {
    ($el_ty:ty [$($const:tt)*], $buf_ty:ty, $conv:ident) => {
        impl<'a, $($const)*> IterReadItem for $el_ty {
            impl_slice_like!(@inner $buf_ty, $conv, buf:
                             Some(v) => {
                                 *buf = Buf { bytes: v.$conv(), consumed: 0 };
                             });
        }

        impl<'a, E, $($const)*> IterReadItem for Result<$el_ty, E> where io::Error: From<E> {
            impl_slice_like!(@inner $buf_ty, $conv, buf:
                             Some(Ok(v)) => {
                                 *buf = Buf { bytes: v.$conv(), consumed: 0 };
                             }
                             Some(Err(err)) => {
                                 *buf = Buf::default();
                                 return Err(err.into());
                             });
        }
    };
    (@inner $buf_ty:ty, $conv:ident, $buf:ident : $($match:tt)+) => {
        type Buffer = Buf<$buf_ty>;

        fn read<I: Iterator<Item=Self>>(target: &mut [u8], it: &mut I,
                                        $buf: &mut Self::Buffer) -> io::Result<usize> {
            while $buf.consumed == $buf.bytes.len() {
                match it.next() {
                    $($match)+
                    None => {
                        *$buf = Buf::default();
                        return Ok(0);
                    }
                }
            }
            let mut len = 0;
            for (slot, elt) in target.iter_mut().zip(&$buf.bytes[$buf.consumed..]) {
                *slot = *elt;
                len += 1;
            }
            $buf.consumed += len;
            Ok(len)
        }
    }
}

impl_byte!(u8, );
impl_byte!(&'a u8, *);

impl_slice_like!(&'a [u8] [], &'a [u8], into);
impl_slice_like!(&'a Vec<u8> [], &'a [u8], as_slice);
impl_slice_like!(Vec<u8> [], Vec<u8>, into);
impl_slice_like!([u8; N] [const N: usize], [u8; N], into);
impl_slice_like!(&'a [u8; N] [const N: usize], &'a [u8], as_slice);

impl_slice_like!(&'a str [], &'a [u8], as_bytes);
impl_slice_like!(&'a String [], &'a [u8], as_bytes);
impl_slice_like!(String [], Vec<u8>, into_bytes);
