// Copyright (c) 2015-2023 Georg Brandl.  Licensed under the Apache License,
// Version 2.0 <LICENSE-APACHE or http://www.apache.org/licenses/LICENSE-2.0>
// or the MIT license <LICENSE-MIT or http://opensource.org/licenses/MIT>, at
// your option. This file may not be copied, modified, or distributed except
// according to those terms.

use std::fmt;
use std::error::Error;
use std::io::{self, Read, ErrorKind};

use crate::{IterRead, IterReadItem};

#[derive(Debug)]
struct MyError;
type MyResult<T> = Result<T, MyError>;

impl fmt::Display for MyError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "oh no")
    }
}

impl Error for MyError {}

impl From<MyError> for io::Error {
    fn from(e: MyError) -> io::Error {
        io::Error::new(ErrorKind::Other, e)
    }
}

fn err<T>() -> Result<T, MyError> {
    Err(MyError)
}

fn check_equal<E: IterReadItem, I: Iterator<Item=E>>(iter: I) {
    let mut reader = IterRead::new(iter);
    let mut buffer = Vec::new();
    reader.read_to_end(&mut buffer).unwrap();
    assert_eq!(buffer, b"abcdefghijklmnopqrstuvwxyz");
}

fn check_err<E: IterReadItem, I: Iterator<Item=E>>(iter: I) {
    let mut reader = IterRead::new(iter);
    let mut buffer = Vec::new();
    let err = reader.read_to_end(&mut buffer).unwrap_err();
    assert_eq!(err.to_string(), "oh no");
}

#[test]
fn test_u8() {
    let test: Vec<u8> = (b'a'..b'{').collect();
    check_equal(test.iter());
    check_equal(test.into_iter());
}

#[test]
fn test_slice() {
    let test: Vec<Vec<u8>> = vec![(97..100).collect(),
                                  vec![],
                                  vec![],
                                  (100..101).collect(),
                                  (101..120).collect(),
                                  (120..123).collect()];
    check_equal(test.iter());
    check_equal(test.iter().map(|v| v.as_slice()));
    check_equal(test.into_iter());
}

#[test]
fn test_array() {
    let test: Vec<[u8; 13]> = vec![*b"abcdefghijklm", *b"nopqrstuvwxyz"];
    check_equal(test.iter());
    check_equal(test.into_iter());

    let test: Vec<[u8; 2]>  = vec![*b"ab", *b"cd", *b"ef", *b"gh", *b"ij",
                                   *b"kl", *b"mn", *b"op", *b"qr", *b"st",
                                   *b"uv", *b"wx", *b"yz"];
    check_equal(test.iter());
    check_equal(test.into_iter());
}

#[test]
fn test_str() {
    let test: Vec<String> = vec!["abcdef".into(),
                                 "g".into(),
                                 "".into(),
                                 "hijklmnopqrstuvwxy".into(),
                                 "z".into()];
    check_equal(test.iter());
    check_equal(test.iter().map(|v| v.as_str()));
    check_equal(test.into_iter());
}

#[test]
fn test_result() {
    let test: Vec<MyResult<u8>> = (b'a'..b'{').map(|v| Ok(v)).collect();
    check_equal(test.into_iter());
    let test: Vec<MyResult<u8>> = vec![Ok(b'a'), Ok(b'b'),
                                       err(), Ok(b'd')];
    check_err(test.into_iter());
    let test: Vec<MyResult<Vec<u8>>> = vec![
        Ok(b"abcdefghijk".to_vec()),
        Ok(b"lmnopqrstuvwxyz".to_vec())];
    check_equal(test.into_iter());
    let test: Vec<MyResult<Vec<u8>>> = vec![Ok(b"abc".to_vec()),
                                            err()];
    check_err(test.into_iter());
}


#[cfg(feature = "unstable")]
mod benches {
    extern crate test;
    use self::test::Bencher;
    use std::io::Read;
    use std::iter;
    use crate::{IterRead, IterReadItem};

    const N: usize = 200000;
    const NB: u64 = N as u64;
    const BYTE: u8 = 42;
    // For iter-of-slices benches:
    // NS: number of slices/vecs, NB: number of bytes per slice/vec
    const NS_LARGE: usize = 20;
    const NB_LARGE: usize = N / NS_LARGE;
    const NS_SMALL: usize = 1000;
    const NB_SMALL: usize = N / NS_SMALL;

    #[bench]
    fn just_iterate(b: &mut Bencher) {
        // For comparison.
        let vec = vec![BYTE; N];
        b.bytes = NB;
        b.iter(|| {
            let _ = vec.iter().cloned().collect::<Vec<_>>();
        });
    }

    fn read_all<E: IterReadItem, I: Iterator<Item=E>>(iter: I) {
        let mut reader = IterRead::new(iter);
        let mut buffer = vec![0; N];
        reader.read_exact(&mut buffer).unwrap();
    }

    #[bench]
    fn read_u8(b: &mut Bencher) {
        let vec = vec![BYTE; N];
        b.bytes = NB;
        b.iter(|| read_all(vec.iter().cloned()));
    }

    #[bench]
    fn read_ref_u8(b: &mut Bencher) {
        let vec = vec![BYTE; N];
        b.bytes = NB;
        b.iter(|| read_all(vec.iter()));
    }

    #[bench]
    fn read_small_vec_u8(b: &mut Bencher) {
        let vec = (0..NS_SMALL).map(|_| vec![BYTE; NB_SMALL])
                               .collect::<Vec<_>>();
        b.bytes = NB;
        b.iter(|| read_all(vec.iter()));
    }

    #[bench]
    fn read_large_vec_u8(b: &mut Bencher) {
        let vec = (0..NS_LARGE).map(|_| vec![BYTE; NB_LARGE])
                               .collect::<Vec<_>>();
        b.bytes = NB;
        b.iter(|| read_all(vec.iter()));
    }

    #[bench]
    fn read_small_slice(b: &mut Bencher) {
        let origin = vec![BYTE; NB_SMALL];
        let vec = (0..NS_SMALL).map(|_| origin.as_slice()).collect::<Vec<_>>();
        b.bytes = NB;
        b.iter(|| read_all(vec.iter().cloned()));
    }

    #[bench]
    fn read_large_slice(b: &mut Bencher) {
        let origin = vec![BYTE; NB_LARGE];
        let vec = (0..NS_LARGE).map(|_| origin.as_slice()).collect::<Vec<_>>();
        b.bytes = NB;
        b.iter(|| read_all(vec.iter().cloned()));
    }

    #[bench]
    fn read_small_string(b: &mut Bencher) {
        let vec = (0..NS_SMALL)
            .map(|_| iter::repeat(BYTE as char).take(NB_SMALL).collect::<String>())
            .collect::<Vec<_>>();
        b.bytes = NB;
        b.iter(|| read_all(vec.iter()));
    }

    #[bench]
    fn read_large_string(b: &mut Bencher) {
        let vec = (0..NS_LARGE)
            .map(|_| iter::repeat(BYTE as char).take(NB_LARGE).collect::<String>())
            .collect::<Vec<_>>();
        b.bytes = NB;
        b.iter(|| read_all(vec.iter()));
    }

    #[bench]
    fn read_small_str(b: &mut Bencher) {
        let origin = iter::repeat(BYTE as char).take(NB_SMALL).collect::<String>();
        let vec = (0..NS_SMALL).map(|_| origin.as_str()).collect::<Vec<_>>();
        b.bytes = NB;
        b.iter(|| read_all(vec.iter().cloned()));
    }

    #[bench]
    fn read_large_str(b: &mut Bencher) {
        let origin = iter::repeat(BYTE as char).take(NB_LARGE).collect::<String>();
        let vec = (0..NS_LARGE).map(|_| origin.as_str()).collect::<Vec<_>>();
        b.bytes = NB;
        b.iter(|| read_all(vec.iter().cloned()));
    }
}
