use super::pixel::*;
use crate::RGB;
use crate::RGBA;
use core::ops::*;

/// `px + px`
impl<T: Add> Add for RGB<T> {
    type Output = RGB<<T as Add>::Output>;

    #[inline(always)]
    fn add(self, other: RGB<T>) -> Self::Output {
        RGB {
            r: self.r + other.r,
            g: self.g + other.g,
            b: self.b + other.b,
        }
    }
}

/// `px + px`
impl<T> AddAssign for RGB<T> where
    T: Add<Output = T> + Copy
{
    fn add_assign(&mut self, other: RGB<T>) {
        *self = Self {
            r: self.r + other.r,
            g: self.g + other.g,
            b: self.b + other.b,
        };
    }
}

/// `px - px`
impl<T: Sub> Sub for RGB<T> {
    type Output = RGB<<T as Sub>::Output>;

    #[inline(always)]
    fn sub(self, other: RGB<T>) -> Self::Output {
        RGB {
            r: self.r - other.r,
            g: self.g - other.g,
            b: self.b - other.b,
        }
    }
}

/// `px - px`
impl<T> SubAssign for RGB<T> where
    T: Sub<Output = T> + Copy
{
    #[inline(always)]
    fn sub_assign(&mut self, other: RGB<T>) {
        *self = Self {
            r: self.r - other.r,
            g: self.g - other.g,
            b: self.b - other.b,
        };
    }
}

/// `px - 1`
impl<T> Sub<T> for RGB<T> where
    T: Copy + Sub<Output=T> 
{
    type Output = RGB<<T as Sub>::Output>;

    #[inline(always)]
    fn sub(self, r: T) -> Self::Output {
        self.map(|l| l-r)
    }
}

/// `px - 1`
impl<T> SubAssign<T> for RGB<T> where
    T: Copy + Sub<Output=T> 
{
    #[inline(always)]
    fn sub_assign(&mut self, r: T) {
        *self = self.map(|l| l-r);
    }
}

/// `px + 1`
impl<T> Add<T> for RGB<T> where
    T: Copy + Add<Output=T>
{
    type Output = RGB<T>;

    #[inline(always)]
    fn add(self, r: T) -> Self::Output {
        self.map(|l|l+r)
    }
}

/// `px + 1`
impl<T> AddAssign<T> for RGB<T> where
    T: Copy + Add<Output=T> 
{
    #[inline(always)]
    fn add_assign(&mut self, r: T) {
        *self = self.map(|l| l+r);
    }
}

/// `px + px`
impl<T: Add, A: Add> Add<RGBA<T, A>> for RGBA<T, A> {
    type Output = RGBA<<T as Add>::Output, <A as Add>::Output>;

    #[inline(always)]
    fn add(self, other: RGBA<T, A>) -> Self::Output {
        RGBA {
            r: self.r + other.r,
            g: self.g + other.g,
            b: self.b + other.b,
            a: self.a + other.a,
        }
    }
}

impl<T, A> AddAssign<RGBA<T, A>> for RGBA<T, A> where
    T: Copy + Add<Output = T>,
    A: Copy + Add<Output = A>
{
    fn add_assign(&mut self, other: RGBA<T, A>) {
        *self = Self {
            r: self.r + other.r,
            g: self.g + other.g,
            b: self.b + other.b,
            a: self.a + other.a,
        };
    }
}

/// `px - px`
impl<T: Sub, A: Sub> Sub<RGBA<T, A>> for RGBA<T, A> {
    type Output = RGBA<<T as Sub>::Output, <A as Sub>::Output>;

    #[inline(always)]
    fn sub(self, other: RGBA<T, A>) -> Self::Output {
        RGBA {
            r: self.r - other.r,
            g: self.g - other.g,
            b: self.b - other.b,
            a: self.a - other.a,
        }
    }
}

/// `px - px`
impl<T, A> SubAssign<RGBA<T, A>> for RGBA<T, A> where
    T: Copy + Sub<Output = T>,
    A: Copy + Sub<Output = A>
{
    #[inline(always)]
    fn sub_assign(&mut self, other: RGBA<T, A>) {
        *self = RGBA {
            r: self.r - other.r,
            g: self.g - other.g,
            b: self.b - other.b,
            a: self.a - other.a,
        }
    }
}

/// `px - 1` 
/// Works only if alpha channel has same depth as RGB channels
impl<T> Sub<T> for RGBA<T> where
    T: Copy + Sub
{
    type Output = RGBA<<T as Sub>::Output, <T as Sub>::Output>;

    #[inline(always)]
    fn sub(self, r: T) -> Self::Output {
        self.map(|l| l - r)
    }
}

/// `px - 1` 
/// Works only if alpha channel has same depth as RGB channels
impl<T> SubAssign<T> for RGBA<T> where
    T: Copy + Sub<Output = T>
{
    #[inline(always)]
    fn sub_assign(&mut self, r: T) {
        *self = self.map(|l| l - r);
    }
}

/// `px + 1`
impl<T> Add<T> for RGBA<T> where
    T: Copy + Add<Output=T>
{
    type Output = RGBA<T>;

    #[inline(always)]
    fn add(self, r: T) -> Self::Output {
        self.map(|l| l+r)
    }
}

/// `px + 1`
impl<T> AddAssign<T> for RGBA<T> where
    T: Copy + Add<Output=T> 
{
    #[inline(always)]
    fn add_assign(&mut self, r: T) {
        *self = self.map(|l| l+r);
    }
}

/// `px * 1`
impl<T> Mul<T> for RGB<T> where
    T: Copy + Mul<Output=T>
{
    type Output = RGB<T>;

    #[inline(always)]
    fn mul(self, r: T) -> Self::Output {
        self.map(|l|l*r)
    }
}

/// `px * 1`
impl<T> MulAssign<T> for RGB<T> where
    T: Copy + Mul<Output=T>
{
    #[inline(always)]
    fn mul_assign(&mut self, r: T) {
        *self = self.map(|l| l*r);
    }
}

/// `px * 1`
impl<T> Mul<T> for RGBA<T> where
    T: Copy + Mul<Output=T>
{
    type Output = RGBA<T>;

    #[inline(always)]
    fn mul(self, r: T) -> Self::Output {
        self.map(|l|l*r)
    }
}

/// `px * 1`
impl<T> MulAssign<T> for RGBA<T> where
    T: Copy + Mul<Output=T>
{
    #[inline(always)]
    fn mul_assign(&mut self, r: T) {
        *self = self.map(|l| l*r);
    }
}

#[cfg(test)]
mod test {
    use super::*;
    const WHITE_RGB: RGB<u8> = RGB::new(255, 255, 255);
    const BLACK_RGB: RGB<u8> = RGB::new(0, 0, 0);
    const RED_RGB: RGB<u8> = RGB::new(255, 0, 0);
    const GREEN_RGB: RGB<u8> = RGB::new(0, 255, 0);
    const BLUE_RGB: RGB<u8> = RGB::new(0, 0, 255);

    const WHITE_RGBA: RGBA<u8> = RGBA::new(255, 255, 255, 255);
    const BLACK_RGBA: RGBA<u8> = RGBA::new(0, 0, 0, 0);
    const RED_RGBA: RGBA<u8> = RGBA::new(255, 0, 0, 255);
    const GREEN_RGBA: RGBA<u8> = RGBA::new(0, 255, 0, 0);
    const BLUE_RGBA: RGBA<u8> = RGBA::new(0, 0, 255, 255);

    #[test]
    fn test_add() {
        assert_eq!(RGB::new(2,4,6), RGB::new(1,2,3) + RGB{r:1,g:2,b:3});
        assert_eq!(RGB::new(2.,4.,6.), RGB::new(1.,3.,5.) + 1.);

        assert_eq!(RGBA::new_alpha(2u8,4,6,8u16), RGBA::new_alpha(1u8,2,3,4u16) + RGBA{r:1u8,g:2,b:3,a:4u16});
        assert_eq!(RGBA::new(2i16,4,6,8), RGBA::new(1,3,5,7) + 1);

        assert_eq!(RGB::new(255, 255, 0), RED_RGB+GREEN_RGB);
        assert_eq!(RGB::new(255, 0, 0), RED_RGB+RGB::new(0, 0, 0));
        assert_eq!(WHITE_RGB, BLACK_RGB + 255);

        assert_eq!(RGBA::new(255, 255, 0, 255), RED_RGBA+GREEN_RGBA);
        assert_eq!(RGBA::new(255, 0, 0, 255), RED_RGBA+RGBA::new(0, 0, 0, 0));
        assert_eq!(WHITE_RGBA, BLACK_RGBA + 255);
    }

    #[test]
    #[should_panic]
    fn test_add_overflow() {
        assert_ne!(RGBA::new(255u8, 255, 0, 0), RED_RGBA+BLUE_RGBA);;
    }

    #[test]
    fn test_sub() {
        assert_eq!(RED_RGB, (WHITE_RGB - GREEN_RGB) - BLUE_RGB);
        assert_eq!(BLACK_RGB, WHITE_RGB - 255);

        assert_eq!(RGBA::new(255, 255, 0, 0), WHITE_RGBA - BLUE_RGBA);
        assert_eq!(BLACK_RGBA, WHITE_RGBA - 255);
    }

    #[test]
    fn test_add_assign() {
        let mut green_rgb = RGB::new(0, 255, 0);
        green_rgb += RGB::new(255, 0, 255);
        assert_eq!(WHITE_RGB, green_rgb);

        let mut black_rgb = RGB::new(0, 0, 0);
        black_rgb += 255;
        assert_eq!(WHITE_RGB, black_rgb);

        let mut green_rgba = RGBA::new(0, 255, 0, 0);
        green_rgba += RGBA::new(255, 0, 255, 255);
        assert_eq!(WHITE_RGBA, green_rgba);

        let mut black_rgba = RGBA::new(0, 0, 0, 0);
        black_rgba += 255;
        assert_eq!(WHITE_RGBA, black_rgba);
    }

    #[test]
    fn test_sub_assign() {
        let mut green_rgb = RGB::new(0, 255, 0);
        green_rgb -= RGB::new(0, 255, 0);
        assert_eq!(BLACK_RGB, green_rgb);

        let mut white_rgb = RGB::new(255, 255, 255);
        white_rgb -= 255;
        assert_eq!(BLACK_RGB, white_rgb);

        let mut green_rgba = RGBA::new(0, 255, 0, 0);
        green_rgba -= RGBA::new(0, 255, 0, 0);
        assert_eq!(BLACK_RGBA, green_rgba);

        let mut white_rgba = RGBA::new(255, 255, 255, 255);
        white_rgba -= 255;
        assert_eq!(BLACK_RGBA, white_rgba);
    }

    #[test]
    fn test_mult() {
        assert_eq!(RGB::new(0.5,1.5,2.5), RGB::new(1.,3.,5.) * 0.5);
        assert_eq!(RGBA::new(2,4,6,8), RGBA::new(1,2,3,4) * 2);
    }

    #[test]
    fn test_mult_assign() {
        let mut green_rgb = RGB::new(0u16, 255, 0);
        green_rgb *= 1;
        assert_eq!(RGB::new(0, 255, 0), green_rgb);
        green_rgb *= 2;
        assert_eq!(RGB::new(0, 255*2, 0), green_rgb);

        let mut green_rgba = RGBA::new(0u16, 255, 0, 0);
        green_rgba *= 1;
        assert_eq!(RGBA::new(0, 255, 0, 0), green_rgba);
        green_rgba *= 2;
        assert_eq!(RGBA::new(0, 255*2, 0, 0), green_rgba);
    }
}
