use super::pixel::*;
use crate::alt::Gray;
use crate::alt::GrayAlpha;
use crate::alt::BGR;
use crate::alt::BGRA;
use crate::RGB;
use crate::RGBA;
use core::convert::*;
use core::mem;
use core::slice;

mod array;
mod tuple;

/// Casts a slice of bytes into a slice of pixels, e.g. `[u8]` to `[RGB8]`.
///
/// See also `FromSlice`
pub trait AsPixels<PixelType> {
    /// Reinterpret the slice as a read-only/shared slice of pixels.
    /// Multiple consecutive elements in the slice are intepreted as a single pixel
    /// (depending on format, e.g. 3 for RGB, 4 for RGBA).
    ///
    /// Leftover elements are ignored if the slice isn't evenly divisible into pixels.
    ///
    /// Use this method only when the type is known from context.
    /// See also `FromSlice`.
    fn as_pixels(&self) -> &[PixelType];
    /// Reinterpret the slice as a mutable/exclusive slice of pixels.
    /// Multiple consecutive elements in the slice are intepreted as a single pixel
    /// (depending on format, e.g. 3 for RGB, 4 for RGBA).
    ///
    /// Leftover elements are ignored if the slice isn't evenly divisible into pixels.
    ///
    /// Use this method only when the type is known from context.
    /// See also `FromSlice`.
    fn as_pixels_mut(&mut self) -> &mut [PixelType];
}

macro_rules! as_pixels_impl {
    ($typ:ident, $elems:expr) => {
        impl<T> AsPixels<$typ<T>> for [T] {
            fn as_pixels(&self) -> &[$typ<T>] {
                unsafe {
                    slice::from_raw_parts(self.as_ptr() as *const _, self.len() / $elems)
                }
            }
            fn as_pixels_mut(&mut self) -> &mut [$typ<T>] {
                unsafe {
                    slice::from_raw_parts_mut(self.as_ptr() as *mut _, self.len() / $elems)
                }
            }
        }
    }
}

as_pixels_impl!{RGB, 3}
as_pixels_impl!{RGBA, 4}
as_pixels_impl!{BGR, 3}
as_pixels_impl!{BGRA, 3}
as_pixels_impl!{Gray, 1}
as_pixels_impl!{GrayAlpha, 2}

/// Cast a slice of component values (bytes) as a slice of RGB/RGBA pixels
///
/// If there's any incomplete pixel at the end of the slice it is ignored.
pub trait FromSlice<T: Copy> {
    /// Reinterpert slice as RGB pixels
    fn as_rgb(&self) -> &[RGB<T>];
    /// Reinterpert slice as RGBA pixels
    fn as_rgba(&self) -> &[RGBA<T>];
    /// Reinterpert mutable slice as RGB pixels
    fn as_rgb_mut(&mut self) -> &mut [RGB<T>];
    /// Reinterpert mutable slice as RGBA pixels
    fn as_rgba_mut(&mut self) -> &mut [RGBA<T>];

    /// Reinterpert slice as reverse-order BGR pixels
    fn as_bgr(&self) -> &[BGR<T>];
    /// Reinterpert slice as reverse-order BGRA pixels
    fn as_bgra(&self) -> &[BGRA<T>];
    /// Reinterpert ntable slice as reverse-order BGR pixels
    fn as_bgr_mut(&mut self) -> &mut [BGR<T>];
    /// Reinterpert mutable slice as reverse-order BGRA pixels
    fn as_bgra_mut(&mut self) -> &mut [BGRA<T>];
}

impl<T: Copy> FromSlice<T> for [T] {
    fn as_rgb(&self) -> &[RGB<T>] {
        debug_assert_eq!(3, mem::size_of::<RGB<T>>() / mem::size_of::<T>());
        unsafe {
            slice::from_raw_parts(self.as_ptr() as *const _, self.len() / 3)
        }
    }
    fn as_rgba(&self) -> &[RGBA<T>] {
        debug_assert_eq!(4, mem::size_of::<RGBA<T>>() / mem::size_of::<T>());
        unsafe {
            slice::from_raw_parts(self.as_ptr() as *const _, self.len() / 4)
        }
    }
    fn as_rgb_mut(&mut self) -> &mut [RGB<T>] {
        debug_assert_eq!(3, mem::size_of::<RGB<T>>() / mem::size_of::<T>());
        unsafe {
            slice::from_raw_parts_mut(self.as_ptr() as *mut _, self.len() / 3)
        }
    }
    fn as_rgba_mut(&mut self) -> &mut [RGBA<T>] {
        debug_assert_eq!(4, mem::size_of::<RGBA<T>>() / mem::size_of::<T>());
        unsafe {
            slice::from_raw_parts_mut(self.as_ptr() as *mut _, self.len() / 4)
        }
    }

    fn as_bgr(&self) -> &[BGR<T>] {
        debug_assert_eq!(3, mem::size_of::<BGR<T>>() / mem::size_of::<T>());
        unsafe {
            slice::from_raw_parts(self.as_ptr() as *const _, self.len() / 3)
        }
    }
    fn as_bgra(&self) -> &[BGRA<T>] {
        debug_assert_eq!(4, mem::size_of::<BGRA<T>>() / mem::size_of::<T>());
        unsafe {
            slice::from_raw_parts(self.as_ptr() as *const _, self.len() / 4)
        }
    }
    fn as_bgr_mut(&mut self) -> &mut [BGR<T>] {
        debug_assert_eq!(3, mem::size_of::<BGR<T>>() / mem::size_of::<T>());
        unsafe {
            slice::from_raw_parts_mut(self.as_ptr() as *mut _, self.len() / 3)
        }
    }
    fn as_bgra_mut(&mut self) -> &mut [BGRA<T>] {
        debug_assert_eq!(4, mem::size_of::<BGRA<T>>() / mem::size_of::<T>());
        unsafe {
            slice::from_raw_parts_mut(self.as_ptr() as *mut _, self.len() / 4)
        }
    }
}

macro_rules! rgb_impl_from {
    ($typename:ident, $from:ty, $to:ty) => {
        impl From<$typename<$from>> for $typename<$to> {

            #[inline(always)]
            fn from(other: $typename<$from>) -> Self {
                other.map(core::convert::Into::into)
            }
        }
    }
}

rgb_impl_from!{RGB, u8,i16}
rgb_impl_from!{RGB, u16,i32}

rgb_impl_from!{RGB, u8,f32}
rgb_impl_from!{RGB, u8,f64}
rgb_impl_from!{RGB, u16,f32}
rgb_impl_from!{RGB, u16,f64}

rgb_impl_from!{RGB, i16,f32}
rgb_impl_from!{RGB, i16,f64}

rgb_impl_from!{RGB, i32,f64}
rgb_impl_from!{RGB, f32,f64}


rgb_impl_from!{RGBA, u16,i32}

rgb_impl_from!{RGBA, u8,f32}
rgb_impl_from!{RGBA, u8,f64}
rgb_impl_from!{RGBA, u16,f32}
rgb_impl_from!{RGBA, u16,f64}

rgb_impl_from!{RGBA, i16,f32}
rgb_impl_from!{RGBA, i16,f64}

rgb_impl_from!{RGBA, i32,f64}
rgb_impl_from!{RGBA, f32,f64}

impl<T: Clone> From<Gray<T>> for RGB<T> {
    fn from(other: Gray<T>) -> Self {
        Self {
            r: other.0.clone(),
            g: other.0.clone(),
            b: other.0,
        }
    }
}

impl<T: Clone,A> From<GrayAlpha<T,A>> for RGBA<T,A> {
    fn from(other: GrayAlpha<T,A>) -> Self {
        Self {
            r: other.0.clone(),
            g: other.0.clone(),
            b: other.0,
            a: other.1,
        }
    }
}

impl<T> From<RGB<T>> for BGR<T> {
    fn from(other: RGB<T>) -> Self {
        Self {
            r: other.r,
            g: other.g,
            b: other.b,
        }
    }
}

impl<T> From<RGBA<T>> for BGRA<T> {
    fn from(other: RGBA<T>) -> Self {
        Self {
            r: other.r,
            g: other.g,
            b: other.b,
            a: other.a,
        }
    }
}

impl<T> From<BGR<T>> for RGB<T> {
    fn from(other: BGR<T>) -> Self {
        Self {
            r: other.r,
            g: other.g,
            b: other.b,
        }
    }
}

impl<T> From<BGRA<T>> for RGBA<T> {
    fn from(other: BGRA<T>) -> Self {
        Self {
            r: other.r,
            g: other.g,
            b: other.b,
            a: other.a,
        }
    }
}

impl<T> AsRef<T> for Gray<T> {
    fn as_ref(&self) -> &T {
        &self.0
    }
}

impl<T> AsRef<[T]> for RGB<T> {
    fn as_ref(&self) -> &[T] {
        self.as_slice()
    }
}

impl<T> AsRef<[T]> for RGBA<T> {
    fn as_ref(&self) -> &[T] {
        self.as_slice()
    }
}

impl<T> AsRef<T> for GrayAlpha<T> {
    fn as_ref(&self) -> &T {
        &self.0
    }
}


impl<T> AsMut<T> for Gray<T> {
    fn as_mut(&mut self) -> &mut T {
        &mut self.0
    }
}

impl<T> AsMut<[T]> for RGB<T> {
    fn as_mut(&mut self) -> &mut [T] {
        self.as_mut_slice()
    }
}

impl<T> AsMut<[T]> for RGBA<T> {
    fn as_mut(&mut self) -> &mut [T] {
        self.as_mut_slice()
    }
}

impl<T> AsMut<T> for GrayAlpha<T> {
    fn as_mut(&mut self) -> &mut T {
        &mut self.0
    }
}


#[test]
fn converts() {
    assert_eq!(RGBA::new(1u8,2,3,255), RGB::new(1u8,2,3).into());
    assert_eq!(RGBA::new(1u16,2,3,65535), RGB::new(1u16,2,3).into());
    assert_eq!(BGRA{r:1u8,g:2u8,b:3u8,a:255u8}, BGR{r:1u8,g:2u8,b:3u8}.into());
    assert_eq!(BGRA{r:1u8,g:2u8,b:3u8,a:255u8}, RGB{r:1u8,g:2u8,b:3u8}.into());
    assert_eq!(RGBA {r:1u8,g:2,b:3,a:4u8}, BGRA{r:1u8,g:2u8,b:3u8,a:4u8}.into());
    assert_eq!(BGR {r:1u8,g:2,b:3u8}, RGB {r:1u8,g:2,b:3u8}.into());
    assert_eq!(RGB {r:1u16,g:0x5678,b:0xABCDu16}, BGR {r:1u16,g:0x5678,b:0xABCDu16}.into());
    assert_eq!(BGR {r:0x1234567u32,g:2,b:3u32}, RGB {r:0x1234567u32,g:2,b:3u32}.into());

    assert_eq!(&[1u8,2,3,4], RGBA {r:1u8,g:2,b:3,a:4u8}.as_slice());
    assert_eq!(&[1u8,2,3,4], RGBA {r:1u8,g:2,b:3,a:4u8}.as_ref());
    assert_eq!(&[1u8,2,3], RGB {r:1u8,g:2,b:3}.as_slice());
    assert_eq!(&[1u8,2,3], RGB {r:1u8,g:2,b:3}.as_ref());

    assert_eq!(&[1u8,2,3], RGB {r:1u8,g:2,b:3}.as_mut_slice());
    assert_eq!(&[1u8,2,3], RGB {r:1u8,g:2,b:3}.as_mut());
}

