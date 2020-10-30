//! Get hostname. Compatible with windows and linux.
//!
//! # Examples
//! ```
//! extern crate hostname;
//!
//! assert!(hostname::get_hostname().is_some());
//! ```
//!
#![cfg_attr(all(feature = "unstable", test), feature(test))]

#[cfg(windows)]
extern crate winutil;

#[cfg(any(unix, target_os = "redox"))]
extern crate libc;


/// Get hostname.
#[cfg(windows)]
pub fn get_hostname() -> Option<String> {
    winutil::get_computer_name()
}


#[cfg(any(unix, target_os = "redox"))]
extern "C" {
    fn gethostname(name: *mut libc::c_char, size: libc::size_t) -> libc::c_int;
}

#[cfg(any(unix, target_os = "redox"))]
use std::ffi::CStr;

/// Get hostname.
#[cfg(any(unix, target_os = "redox"))]
pub fn get_hostname() -> Option<String> {
    let len = 255;
    let mut buf = Vec::<u8>::with_capacity(len);
    let ptr = buf.as_mut_ptr() as *mut libc::c_char;

    unsafe {
        if gethostname(ptr, len as libc::size_t) != 0 {
            return None;
        }

        Some(CStr::from_ptr(ptr).to_string_lossy().into_owned())
    }
}

#[test]
fn test_get_hostname() {
    assert!(get_hostname().is_some());
    assert!(!get_hostname().unwrap().is_empty());
}

#[cfg(all(feature = "unstable", test))]
mod benches {
    extern crate test;
    use super::get_hostname;

    #[bench]
    fn bench_get_hostname(b: &mut test::Bencher) {
        b.iter(|| get_hostname().unwrap())
    }
}
