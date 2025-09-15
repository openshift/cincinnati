use std::ops::{Add, Index, IndexMut, Mul, Sub};

use subtle::{Choice, ConstantTimeEq};

/// This is the scalar field
/// size = 4q = 2^446 - 0x8335dc163bb124b65129c96fde933d8d723a70aadc873d6d54a7bb0d
/// We can therefore use 14 saturated 32-bit limbs
#[derive(Debug, Copy, Clone)]
pub struct Scalar(pub(crate) [u32; 14]);

const MODULUS: Scalar = Scalar([
    0xab5844f3, 0x2378c292, 0x8dc58f55, 0x216cc272, 0xaed63690, 0xc44edb49, 0x7cca23e9, 0xffffffff,
    0xffffffff, 0xffffffff, 0xffffffff, 0xffffffff, 0xffffffff, 0x3fffffff,
]);
// Montgomomery R^2
const R2: Scalar = Scalar([
    0x049b9b60, 0xe3539257, 0xc1b195d9, 0x7af32c4b, 0x88ea1859, 0x0d66de23, 0x5ee4d838, 0xae17cf72,
    0xa3c47c44, 0x1a9cc14b, 0xe4d070af, 0x2052bcb7, 0xf823b729, 0x3402a939,
]);

impl ConstantTimeEq for Scalar {
    fn ct_eq(&self, other: &Self) -> Choice {
        self.to_bytes().ct_eq(&other.to_bytes())
    }
}

impl PartialEq for Scalar {
    fn eq(&self, other: &Scalar) -> bool {
        self.ct_eq(&other).into()
    }
}
impl Eq for Scalar {}

impl From<u32> for Scalar {
    fn from(a: u32) -> Scalar {
        Scalar([a, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0])
    }
}

impl Index<usize> for Scalar {
    type Output = u32;
    fn index(&self, index: usize) -> &Self::Output {
        &self.0[index]
    }
}
impl IndexMut<usize> for Scalar {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        &mut self.0[index]
    }
}

// Trait implementations

impl Add<Scalar> for Scalar {
    type Output = Scalar;
    fn add(self, rhs: Scalar) -> Self::Output {
        add(&self, &rhs)
    }
}
impl Mul<Scalar> for Scalar {
    type Output = Scalar;
    fn mul(self, rhs: Scalar) -> Self::Output {
        let unreduced = montgomery_multiply(&self, &rhs);
        montgomery_multiply(&unreduced, &R2)
    }
}
impl Sub<Scalar> for Scalar {
    type Output = Scalar;
    fn sub(self, rhs: Scalar) -> Self::Output {
        sub_extra(&self, &rhs, 0)
    }
}
impl Default for Scalar {
    fn default() -> Scalar {
        Scalar::zero()
    }
}

impl Scalar {
    pub const fn one() -> Scalar {
        Scalar([1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0])
    }
    pub const fn zero() -> Scalar {
        Scalar([0; 14])
    }
    /// Divides a scalar by four without reducing mod p
    /// This is used in the 2-isogeny when mapping points from Ed448-Goldilocks
    /// to Twisted-Goldilocks
    pub(crate) fn div_by_four(&mut self) {
        for i in 0..=12 {
            self.0[i] = (self.0[i + 1] << 30) | (self.0[i] >> 2);
        }
        self.0[13] >>= 2
    }
    // This method was modified from Curve25519-Dalek codebase. [scalar.rs]
    // We start with 14 u32s and convert them to 56 u8s.
    // We then use the code copied from Dalek to convert the 56 u8s to radix-16 and re-center the coefficients to be between [-16,16)
    // XXX: We can recode the scalar without converting it to bytes, will refactor this method to use this and check which is faster.
    pub(crate) fn to_radix_16(&self) -> [i8; 113] {
        let bytes = self.to_bytes();
        let mut output = [0i8; 113];

        // Step 1: change radix.
        // Convert from radix 256 (bytes) to radix 16 (nibbles)
        #[inline(always)]
        fn bot_half(x: u8) -> u8 {
            (x >> 0) & 15
        }
        #[inline(always)]
        fn top_half(x: u8) -> u8 {
            (x >> 4) & 15
        }

        // radix-16
        for i in 0..56 {
            output[2 * i] = bot_half(bytes[i]) as i8;
            output[2 * i + 1] = top_half(bytes[i]) as i8;
        }
        // re-center co-efficients to be between [-8, 8)
        for i in 0..112 {
            let carry = (output[i] + 8) >> 4;
            output[i] -= carry << 4;
            output[i + 1] += carry;
        }

        output
    }
    // XXX: Better if this method returns an array of 448 items
    pub fn bits(&self) -> Vec<bool> {
        let mut bits: Vec<bool> = Vec::with_capacity(14 * 32);
        // We have 14 limbs, each 32 bits
        // First we iterate each limb
        for limb in self.0.iter() {
            // Then we iterate each bit in the limb
            for j in 0..32 {
                bits.push(limb & (1 << j) != 0)
            }
        }

        // XXX :We are doing LSB first
        bits
    }
    pub fn from_bytes(bytes: [u8; 56]) -> Scalar {
        let load7 = |input: &[u8]| -> u64 {
            (input[0] as u64)
                | ((input[1] as u64) << 8)
                | ((input[2] as u64) << 16)
                | ((input[3] as u64) << 24)
        };

        let mut res = Scalar::zero();
        for i in 0..14 {
            // Load i'th 32 bytes
            let out = load7(&bytes[i * 4..]);
            res[i] = out as u32;
        }

        res
    }
    pub fn to_bytes(&self) -> [u8; 56] {
        let mut res = [0u8; 56];

        for i in 0..14 {
            let mut l = self.0[i];
            for j in 0..4 {
                res[4 * i + j] = l as u8;
                l >>= 8;
            }
        }
        res
    }
    fn square(&self) -> Scalar {
        montgomery_multiply(&self, &self)
    }
    pub fn invert(&self) -> Self {
        let mut pre_comp: Vec<Scalar> = vec![Scalar::zero(); 8];
        let mut result = Scalar::zero();

        let scalar_window_bits = 3;
        let last = (1 << scalar_window_bits) - 1;

        // precompute [a^1, a^3,,..]
        pre_comp[0] = montgomery_multiply(self, &R2);

        if last > 0 {
            pre_comp[last] = montgomery_multiply(&pre_comp[0], &pre_comp[0]);
        }

        for i in 1..=last {
            pre_comp[i] = montgomery_multiply(&pre_comp[i - 1], &pre_comp[last])
        }

        // Sliding window
        let mut residue: usize = 0;
        let mut trailing: usize = 0;
        let mut started: usize = 0;

        // XXX: This can definitely be refactored to be readable
        let loop_start = -scalar_window_bits as isize;
        let loop_end = 446 - 1;
        for i in (loop_start..=loop_end).rev() {
            if started != 0 {
                result = result.square()
            }

            let mut w: u32;
            if i >= 0 {
                w = MODULUS[(i / 32) as usize];
            } else {
                w = 0;
            }

            if i >= 0 && i < 32 {
                w -= 2
            }

            residue = (((residue as u32) << 1) | ((w >> ((i as u32) % 32)) & 1)) as usize;
            if residue >> scalar_window_bits != 0 {
                trailing = residue;
                residue = 0
            }

            if trailing > 0 && (trailing & ((1 << scalar_window_bits) - 1)) == 0 {
                if started != 0 {
                    result = montgomery_multiply(
                        &result,
                        &pre_comp[trailing >> (scalar_window_bits + 1)],
                    )
                } else {
                    result = pre_comp[trailing >> (scalar_window_bits + 1)];
                    started = 1
                }
                trailing = 0
            }
            trailing <<= 1
        }

        // de-montgomerize and return result

        montgomery_multiply(&result, &Scalar::one())
    }

    /// Halves a Scalar modulo the prime
    pub fn halve(&self) -> Self {
        let mut result = Scalar::zero();

        let mask = 0u32.wrapping_sub(self[0] & 1);
        let mut chain = 0u64;

        for i in 0..14 {
            chain += (self[i] as u64) + ((MODULUS[i] & mask) as u64);
            result[i] = chain as u32;
            chain >>= 32
        }

        for i in 0..13 {
            result[i] = (result[i] >> 1) | (result[i + 1] << 31);
        }
        result[13] = (result[13] >> 1) | ((chain << 31) as u32);

        result
    }
}
/// Computes a + b mod p
pub fn add(a: &Scalar, b: &Scalar) -> Scalar {
    // First add the two Scalars together
    // Since our limbs are saturated, the result of each
    // limb being added can be a 33-bit integer so we propagate the carry bit
    let mut result = Scalar::zero();

    // a + b
    let mut chain = 0u64;
    //XXX: Can unroll all of these for loops. They are mainly just ripple carry/borrow adders.
    for i in 0..14 {
        chain += (a[i] as u64) + (b[i] as u64);
        // Low 32 bits are the results
        result[i] = chain as u32;
        // 33rd bit is the carry
        chain >>= 32;
    }

    // Now reduce the results
    sub_extra(&result, &MODULUS, chain as u32)
}

/// Compute a - b mod p
/// Computes a - b and conditionally computes the modulus if the result was negative
fn sub_extra(a: &Scalar, b: &Scalar, carry: u32) -> Scalar {
    let mut result = Scalar::zero();

    // a - b
    let mut chain = 0i64;
    for i in 0..14 {
        chain += a[i] as i64 - b[i] as i64;
        // Low 32 bits are the results
        result[i] = chain as u32;
        // 33rd bit is the borrow
        chain >>= 32
    }

    // if the result of a-b was negative and carry was zero
    // then borrow will be 0xfff..fff and the modulus will be added conditionally to the result
    // If the carry was 1 and a-b was not negative, then the borrow will be 0x00000...001 ( this should not happen)
    // Since the borrow should never be more than 0, the carry should never be more than 1;
    // XXX: Explain why the case of borrow == 1 should never happen
    let borrow = chain + (carry as i64);
    assert!(borrow == -1 || borrow == 0);

    chain = 0i64;
    for i in 0..14 {
        chain += (result[i] as i64) + ((MODULUS[i] as i64) & borrow);
        // Low 32 bits are the results
        result[i] = chain as u32;
        // 33rd bit is the carry
        chain >>= 32;
    }

    result
}

fn montgomery_multiply(x: &Scalar, y: &Scalar) -> Scalar {
    const MONTGOMERY_FACTOR: u32 = 0xae918bc5;

    let mut result = Scalar::zero();
    let mut carry = 0u32;

    // (a * b ) + c
    let mul_add = |a: u32, b: u32, c: u32| -> u64 { ((a as u64) * (b as u64)) + (c as u64) };

    for i in 0..14 {
        let mut chain = 0u64;
        for j in 0..14 {
            chain += mul_add(x[i], y[j], result[j]);
            result[j] = chain as u32;
            chain >>= 32;
        }

        let saved = chain as u32;
        let multiplicand = result[0].wrapping_mul(MONTGOMERY_FACTOR);
        chain = 0u64;

        for j in 0..14 {
            chain += mul_add(multiplicand, MODULUS[j], result[j]);
            if j > 0 {
                result[j - 1] = chain as u32;
            }
            chain >>= 32;
        }
        chain += (saved as u64) + (carry as u64);
        result[14 - 1] = chain as u32;
        carry = (chain >> 32) as u32;
    }

    sub_extra(&result, &MODULUS, carry)
}
#[cfg(test)]
mod test {
    use super::*;
    #[test]
    fn test_basic_add() {
        let five = Scalar::from(5);
        let six = Scalar::from(6);

        assert_eq!(five + six, Scalar::from(11))
    }

    #[test]
    fn test_basic_sub() {
        let ten = Scalar::from(10);
        let five = Scalar::from(5);
        assert_eq!(ten - five, Scalar::from(5))
    }

    #[test]
    fn test_basic_mul() {
        let ten = Scalar::from(10);
        let five = Scalar::from(5);

        assert_eq!(ten * five, Scalar::from(50))
    }

    #[test]
    fn test_mul() {
        let a = Scalar([
            0xffb823a3, 0xc96a3c35, 0x7f8ed27d, 0x087b8fb9, 0x1d9ac30a, 0x74d65764, 0xc0be082e,
            0xa8cb0ae8, 0xa8fa552b, 0x2aae8688, 0x2c3dc273, 0x47cf8cac, 0x3b089f07, 0x1e63e807,
        ]);

        let b = Scalar([
            0xd8bedc42, 0x686eb329, 0xe416b899, 0x17aa6d9b, 0x1e30b38b, 0x188c6b1a, 0xd099595b,
            0xbc343bcb, 0x1adaa0e7, 0x24e8d499, 0x8e59b308, 0x0a92de2d, 0xcae1cb68, 0x16c5450a,
        ]);

        let exp = Scalar([
            0xa18d010a, 0x1f5b3197, 0x994c9c2b, 0x6abd26f5, 0x08a3a0e4, 0x36a14920, 0x74e9335f,
            0x07bcd931, 0xf2d89c1e, 0xb9036ff6, 0x203d424b, 0xfccd61b3, 0x4ca389ed, 0x31e055c1,
        ]);

        assert_eq!(a * b, exp)
    }
    #[test]
    fn test_basic_square() {
        let a = Scalar([
            0xcf5fac3d, 0x7e56a34b, 0xf640922b, 0x3fa50692, 0x1370f8b8, 0x6f08f331, 0x8dccc486,
            0x4bb395e0, 0xf22c6951, 0x21cc3078, 0xd2391f9d, 0x930392e5, 0x04b3273b, 0x31620816,
        ]);
        let expected_a_squared = Scalar([
            0x15598f62, 0xb9b1ed71, 0x52fcd042, 0x862a9f10, 0x1e8a309f, 0x9988f8e0, 0xa22347d7,
            0xe9ab2c22, 0x38363f74, 0xfd7c58aa, 0xc49a1433, 0xd9a6c4c3, 0x75d3395e, 0x0d79f6e3,
        ]);

        assert_eq!(a.square(), expected_a_squared)
    }

    #[test]
    fn test_sanity_check_index_mut() {
        let mut x = Scalar::one();
        x[0] = 2u32;
        assert_eq!(x, Scalar::from(2))
    }
    #[test]
    fn test_basic_halving() {
        let eight = Scalar::from(8);
        let four = Scalar::from(4);
        let two = Scalar::from(2);
        assert_eq!(eight.halve(), four);
        assert_eq!(four.halve(), two);
        assert_eq!(two.halve(), Scalar::one());
    }

    #[test]
    fn test_equals() {
        let a = Scalar::from(5);
        let b = Scalar::from(5);
        let c = Scalar::from(10);
        assert!(a == b);
        assert!(!(a == c))
    }

    #[test]
    fn test_basic_inversion() {
        // Test inversion from 2 to 100
        for i in 1..=100 {
            let x = Scalar::from(i);
            let x_inv = x.invert();
            assert_eq!(x_inv * x, Scalar::one())
        }

        // Inversion of zero is zero
        let zero = Scalar::zero();
        let expected_zero = zero.invert();
        assert_eq!(expected_zero, zero)
    }
    #[test]
    fn test_serialise() {
        let scalar = Scalar([
            0x15598f62, 0xb9b1ed71, 0x52fcd042, 0x862a9f10, 0x1e8a309f, 0x9988f8e0, 0xa22347d7,
            0xe9ab2c22, 0x38363f74, 0xfd7c58aa, 0xc49a1433, 0xd9a6c4c3, 0x75d3395e, 0x0d79f6e3,
        ]);
        let got = Scalar::from_bytes(scalar.to_bytes());
        assert_eq!(scalar, got)
    }
    #[test]
    fn test_debug() {
        let k = Scalar([
            200, 210, 250, 145, 130, 180, 147, 122, 222, 230, 214, 247, 203, 32,
        ]);
        let s = k;
        dbg!(&s.to_radix_16()[..]);
    }
}
