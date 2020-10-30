use crate::alt::BGR;
use crate::alt::BGRA;
use crate::RGB;
use crate::RGBA;
use core::convert::*;

impl<T> From<(T,T,T)> for RGB<T> {
    #[inline]
    fn from(other: (T,T,T)) -> Self {
        Self {
            r: other.0,
            g: other.1,
            b: other.2,
        }
    }
}

impl<T> Into<(T,T,T)> for RGB<T> {
    #[inline]
    fn into(self) -> (T,T,T) {
        (self.r, self.g, self.b)
    }
}

impl<T,A> From<(T,T,T,A)> for RGBA<T,A> {
    #[inline]
    fn from(other: (T,T,T,A)) -> Self {
        Self {
            r: other.0,
            g: other.1,
            b: other.2,
            a: other.3,
        }
    }
}

impl<T,A> Into<(T,T,T,A)> for RGBA<T,A> {
    #[inline]
    fn into(self) -> (T,T,T,A) {
        (self.r, self.g, self.b, self.a)
    }
}

impl<T> From<(T,T,T)> for BGR<T> {
    fn from(other: (T,T,T)) -> Self {
        Self {
            b: other.0,
            g: other.1,
            r: other.2,
        }
    }
}

impl<T> Into<(T,T,T)> for BGR<T> {
    fn into(self) -> (T,T,T) {
        (self.b, self.g, self.r)
    }
}

impl<T,A> From<(T,T,T,A)> for BGRA<T,A> {
    fn from(other: (T,T,T,A)) -> Self {
        Self {
            b: other.0,
            g: other.1,
            r: other.2,
            a: other.3,
        }
    }
}

impl<T,A> Into<(T,T,T,A)> for BGRA<T,A> {
    fn into(self) -> (T,T,T,A) {
        (self.b, self.g, self.r, self.a)
    }
}

#[test]
fn converts() {
    assert_eq!((1,2,3), RGB {r:1u8,g:2,b:3}.into());
    assert_eq!(RGB {r:1u8,g:2,b:3}, (1,2,3).into());
    assert_eq!((1,2,3,4), RGBA {r:1,g:2,b:3,a:4}.into());
    assert_eq!(RGBA {r:1u8,g:2,b:3,a:4}, (1,2,3,4).into());
    assert_eq!(BGRA {r:1u8,g:2,b:3,a:4}, (3,2,1,4).into());
    assert_eq!(BGR {r:1u8,g:2,b:3}, (3,2,1).into());
}
