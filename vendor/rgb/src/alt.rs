use crate::internal::pixel::*;
use core::mem;
use core::ops;
use core::slice;

#[repr(C)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Copy, Clone, Debug, Default, Eq, PartialEq, Ord, PartialOrd, Hash)]
/// RGB in reverse byte order
pub struct BGR<ComponentType> {
    /// Blue first
    pub b: ComponentType,
    /// Green
    pub g: ComponentType,
    /// Red last
    pub r: ComponentType,
}

#[repr(C)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Copy, Clone, Debug, Default, Eq, PartialEq, Ord, PartialOrd, Hash)]
/// BGR+A
pub struct BGRA<ComponentType, AlphaComponentType = ComponentType> {
    /// Blue first
    pub b: ComponentType,
    /// Green
    pub g: ComponentType,
    /// Red
    pub r: ComponentType,
    /// Alpha last
    pub a: AlphaComponentType,
}

pub type BGR8 = BGR<u8>;

/// 16-bit BGR in machine's native endian
pub type BGR16 = BGR<u16>;

pub type BGRA8 = BGRA<u8>;

/// 16-bit BGR in machine's native endian
pub type BGRA16 = BGRA<u16>;

////////////////////////////////////////

#[repr(C)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Copy, Clone, Debug, Default, Eq, PartialEq, Ord, PartialOrd, Hash)]
/// Grayscale. Use `.0` or `*` (deref) to access the value.
pub struct Gray<ComponentType>(
    /// brightness level
    pub ComponentType,
);

#[repr(C)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Copy, Clone, Debug, Default, Eq, PartialEq, Ord, PartialOrd, Hash)]
/// Grayscale with alpha. Use `.0`/`.1` to access.
pub struct GrayAlpha<ComponentType, AlphaComponentType = ComponentType>(
    /// brightness level
    pub ComponentType,
    /// alpha
    pub AlphaComponentType,
);

pub type GRAY8 = Gray<u8>;

/// 16-bit gray in machine's native endian
pub type GRAY16 = Gray<u16>;

pub type GRAYA8 = GrayAlpha<u8>;

/// 16-bit gray in machine's native endian
pub type GRAYA16 = GrayAlpha<u16>;

impl<T> ops::Deref for Gray<T> {
    type Target = T;
    fn deref(&self) -> &T {
        &self.0
    }
}

impl<T: Copy> From<T> for Gray<T> {
    fn from(component: T) -> Self {
        Gray(component)
    }
}

impl<T: Clone, A> GrayAlpha<T, A> {
    /// Copy `Gray` component out of the `GrayAlpha` struct
    #[inline(always)]
    pub fn gray(&self) -> Gray<T> {
        Gray(self.0.clone())
    }
}

impl<T, A> GrayAlpha<T, A> {
    /// Provide a mutable view of only `Gray` component (leaving out alpha).
    #[inline(always)]
    pub fn gray_mut(&mut self) -> &mut Gray<T> {
        unsafe {
            mem::transmute(self)
        }
    }
}

impl<T: Copy, A: Clone> GrayAlpha<T, A> {
    #[inline(always)]
    /// Create a new `GrayAlpha` with the new alpha value, but same gray value
    pub fn alpha(&self, a: A) -> Self {
        Self(self.0, a)
    }

    /// Create a new `GrayAlpha` with a new alpha value created by the callback.
    pub fn map_alpha<F, B>(&self, f: F) -> GrayAlpha<T, B>
        where F: FnOnce(A) -> B
    {
        GrayAlpha (self.0, f(self.1.clone()))
    }

    /// Create new `GrayAlpha` with the same alpha value, but different `Gray` value
    #[inline(always)]
    pub fn map_gray<F, U, B>(&self, f: F) -> GrayAlpha<U, B>
        where F: FnOnce(T) -> U, U: Clone, B: From<A> + Clone {
        GrayAlpha(f(self.0.clone()), self.1.clone().into())
    }
}

impl<T: Copy, B> ComponentMap<GrayAlpha<B>, T, B> for GrayAlpha<T> {
    #[inline(always)]
    fn map<F>(&self, mut f: F) -> GrayAlpha<B>
    where
        F: FnMut(T) -> B,
    {
        GrayAlpha(f(self.0), f(self.1))
    }
}

impl<T> ComponentSlice<T> for GrayAlpha<T> {
    #[inline(always)]
    fn as_slice(&self) -> &[T] {
        unsafe {
            slice::from_raw_parts(self as *const Self as *const T, 2)
        }
    }

    #[inline(always)]
    fn as_mut_slice(&mut self) -> &mut [T] {
        unsafe {
            slice::from_raw_parts_mut(self as *mut Self as *mut T, 2)
        }
    }
}

impl<T> ComponentSlice<T> for [GrayAlpha<T>] {
    #[inline]
    fn as_slice(&self) -> &[T] {
        unsafe {
            slice::from_raw_parts(self.as_ptr() as *const _, self.len() * 2)
        }
    }
    #[inline]
    fn as_mut_slice(&mut self) -> &mut [T] {
        unsafe {
            slice::from_raw_parts_mut(self.as_ptr() as *mut _, self.len() * 2)
        }
    }
}

impl<T: Copy + Send + Sync + 'static> ComponentBytes<T> for [GrayAlpha<T>] {}

impl<T> ComponentSlice<T> for Gray<T> {
    #[inline(always)]
    fn as_slice(&self) -> &[T] {
        unsafe {
            slice::from_raw_parts(self as *const Self as *const T, 1)
        }
    }

    #[inline(always)]
    fn as_mut_slice(&mut self) -> &mut [T] {
        unsafe {
            slice::from_raw_parts_mut(self as *mut Self as *mut T, 1)
        }
    }
}

impl<T> ComponentSlice<T> for [Gray<T>] {
    #[inline]
    fn as_slice(&self) -> &[T] {
        unsafe {
            slice::from_raw_parts(self.as_ptr() as *const _, self.len())
        }
    }
    #[inline]
    fn as_mut_slice(&mut self) -> &mut [T] {
        unsafe {
            slice::from_raw_parts_mut(self.as_ptr() as *mut _, self.len())
        }
    }
}

impl<T: Copy + Send + Sync + 'static> ComponentBytes<T> for [Gray<T>] {}

/// Assumes 255 is opaque
impl<T: Copy> From<Gray<T>> for GrayAlpha<T, u8> {
    fn from(other: Gray<T>) -> Self {
        GrayAlpha(other.0, 0xFF)
    }
}

/// Assumes 65535 is opaque
impl<T: Copy> From<Gray<T>> for GrayAlpha<T, u16> {
    fn from(other: Gray<T>) -> Self {
        GrayAlpha(other.0, 0xFFFF)
    }
}

#[test]
fn gray() {
    let rgb: crate::RGB<_> = Gray(1).into();
    assert_eq!(rgb.r, 1);
    assert_eq!(rgb.g, 1);
    assert_eq!(rgb.b, 1);

    let g: GRAY8 = 100.into();
    assert_eq!(110, *g + 10);
    assert_eq!(110, 10 + Gray(100).as_ref());

    let ga: GRAYA8 = GrayAlpha(1, 2);
    assert_eq!(ga.gray(), Gray(1));
    let mut g2 = ga.clone();
    *g2.gray_mut() = Gray(3);
    assert_eq!(g2.map_gray(|g| g+1), GrayAlpha(4, 2));
    assert_eq!(g2.map(|g| g+1), GrayAlpha(4, 3));
    assert_eq!(g2.0, 3);
    assert_eq!(g2.as_slice(), &[3, 2]);
    assert_eq!(g2.as_mut_slice(), &[3, 2]);
    assert_eq!(g2.alpha(13), GrayAlpha(3, 13));
    assert_eq!(g2.map_alpha(|x| x+3), GrayAlpha(3, 5));

    assert_eq!((&[Gray(1u16), Gray(2)][..]).as_slice(), &[1, 2]);
    assert_eq!((&[GrayAlpha(1u16, 2), GrayAlpha(3, 4)][..]).as_slice(), &[1, 2, 3, 4]);

    let rgba: crate::RGBA<_> = ga.into();
    assert_eq!(rgba.r, 1);
    assert_eq!(rgba.g, 1);
    assert_eq!(rgba.b, 1);
    assert_eq!(rgba.a, 2);

    let ga: GRAYA16 = GrayAlpha(1,2);
    let rgba: crate::RGBA<u16, u16> = ga.into();
    assert_eq!(rgba.r, 1);
    assert_eq!(rgba.g, 1);
    assert_eq!(rgba.b, 1);
    assert_eq!(rgba.a, 2);
}

