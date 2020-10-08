use super::pixel::*;
use crate::alt::BGR;
use crate::alt::BGRA;
use crate::RGB;
use crate::RGBA;
use core;
use core::fmt;

macro_rules! impl_rgb {
    ($RGB:ident, $RGBA:ident) => {
        impl<T> $RGB<T> {
            /// Convenience function for creating a new pixel
            /// The order of arguments is R,G,B
            #[inline(always)]
            pub const fn new(r: T, g: T, b: T) -> Self {
                Self { r, g, b }
            }
        }
        impl<T: Clone> $RGB<T> {
            /// Iterate over color components (R, G, and B)
            #[inline(always)]
            pub fn iter(&self) -> core::iter::Cloned<core::slice::Iter<'_, T>> {
                self.as_slice().iter().cloned()
            }

            // Convenience function for converting to RGBA
            #[inline(always)]
            pub fn alpha(&self, a: T) -> $RGBA<T> {
                $RGBA {
                    r: self.r.clone(),
                    g: self.g.clone(),
                    b: self.b.clone(),
                    a,
                }
            }

            // Convenience function for converting to RGBA with alpha channel of a different type than type of the pixels
            #[inline(always)]
            pub fn new_alpha<A>(&self, a: A) -> $RGBA<T, A> {
                $RGBA {
                    r: self.r.clone(),
                    g: self.g.clone(),
                    b: self.b.clone(),
                    a,
                }
            }
        }

        impl<T: Copy, B> ComponentMap<$RGB<B>, T, B> for $RGB<T> {
            #[inline(always)]
            fn map<F>(&self, mut f: F) -> $RGB<B>
                where F: FnMut(T) -> B {
                $RGB {
                    r:f(self.r),
                    g:f(self.g),
                    b:f(self.b),
                }
            }
        }

        impl<T> ComponentSlice<T> for $RGB<T> {
            #[inline(always)]
            fn as_slice(&self) -> &[T] {
                unsafe {
                    core::slice::from_raw_parts(self as *const Self as *const T, 3)
                }
            }

            #[inline(always)]
            fn as_mut_slice(&mut self) -> &mut [T] {
                unsafe {
                    core::slice::from_raw_parts_mut(self as *mut Self as *mut T, 3)
                }
            }
        }

        impl<T> ComponentSlice<T> for [$RGB<T>] {
            #[inline]
            fn as_slice(&self) -> &[T] {
                unsafe {
                    core::slice::from_raw_parts(self.as_ptr() as *const _, self.len() * 3)
                }
            }

            #[inline]
            fn as_mut_slice(&mut self) -> &mut [T] {
                unsafe {
                    core::slice::from_raw_parts_mut(self.as_ptr() as *mut _, self.len() * 3)
                }
            }
        }

        impl<T: Copy + Send + Sync + 'static> ComponentBytes<T> for [$RGB<T>] {}
    }
}

impl<T> core::iter::FromIterator<T> for RGB<T> {
    /// Takes exactly 3 elements from the iterator and creates a new instance.
    /// Panics if there are fewer elements in the iterator.
    #[inline(always)]
    fn from_iter<I: IntoIterator<Item = T>>(into_iter: I) -> Self {
        let mut iter = into_iter.into_iter();
        Self {
            r: iter.next().unwrap(),
            g: iter.next().unwrap(),
            b: iter.next().unwrap(),
        }
    }
}

impl_rgb!{RGB, RGBA}
impl_rgb!{BGR, BGRA}

impl<T: fmt::Display> fmt::Display for RGB<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f,"rgb({},{},{})", self.r,self.g,self.b)
    }
}

impl<T: fmt::UpperHex> fmt::UpperHex for RGB<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f,"RGB {{ #{:02X}{:02X}{:02X} }}", self.r, self.g, self.b)
    }
}

impl<T: fmt::LowerHex> fmt::LowerHex for RGB<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f,"RGB {{ #{:02x}{:02x}{:02x} }}", self.r, self.g, self.b)
    }
}

impl<T: fmt::Display> fmt::Display for BGR<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f,"bgr({},{},{})", self.b, self.g, self.r)
    }
}

impl<T: fmt::UpperHex> fmt::UpperHex for BGR<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f,"BGR {{ #{:02X}{:02X}{:02X} }}", self.b, self.g, self.r)
    }
}

impl<T: fmt::LowerHex> fmt::LowerHex for BGR<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f,"BGR {{ #{:02x}{:02x}{:02x} }}", self.b, self.g, self.r)
    }
}

#[cfg(test)]
mod rgb_test {
    use super::*;
    use std;
    #[test]
    fn sanity_check() {
        let neg = RGB::new(1,2,3i32).map(|x| -x);
        assert_eq!(neg.r, -1);
        assert_eq!(neg.g, -2);
        assert_eq!(neg.b, -3);

        let mut px = RGB::new(3,4,5);
        px.as_mut_slice()[1] = 111;
        assert_eq!(111, px.g);

        assert_eq!(RGBA::new(250,251,252,253), RGB::new(250,251,252).alpha(253));

        assert_eq!(RGB{r:1u8,g:2,b:3}, RGB::new(1u8,2,3));
        assert!(RGB{r:1u8,g:1,b:2} < RGB::new(2,1,1));

        let mut h = std::collections::HashSet::new();
        h.insert(px);
        assert!(h.contains(&RGB::new(3,111,5)));
        assert!(!h.contains(&RGB::new(111,5,3)));

        let v = vec![RGB::new(1u8,2,3), RGB::new(4,5,6)];
        assert_eq!(&[1,2,3,4,5,6], v.as_bytes());

        assert_eq!(RGB::new(0u8,0,0), Default::default());
    }

    #[test]
    fn test_fmt() {
        let red_rgb = RGB::new(255, 0, 0);
        let red_bgr = BGR::new(255, 0, 0);
        assert_eq!("RGB { #FF0000 }", &format!("{:X}", red_rgb));
        assert_eq!("BGR { #0000FF }", &format!("{:X}", red_bgr));

        assert_eq!("RGB { #ff0000 }", &format!("{:x}", red_rgb));
        assert_eq!("BGR { #0000ff }", &format!("{:x}", red_bgr));

        assert_eq!("rgb(255,0,0)", &format!("{}", red_rgb));
        assert_eq!("bgr(0,0,255)", &format!("{}", red_bgr));
    }
}
