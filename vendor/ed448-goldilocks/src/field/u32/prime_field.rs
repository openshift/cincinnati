use super::karatsuba;
use std::ops::{Add, Index, IndexMut, Mul, Sub};
use subtle::{Choice, ConditionallyNegatable, ConditionallySelectable};
/// FieldElement28 represents an element in the field
/// q = 2^448 - 2^224 -1
///
/// FieldElement28 is represented using radix 2^28 as 16 u32s. We therefore represent
/// a field element `x` as x_0 + x_1 * 2^{28 * 1} + .... + x_15 * 2^{28 * 15}

#[derive(Copy, Clone, Debug)]
pub struct FieldElement28(pub(crate) [u32; 16]);

////
/// Trait Implementations
///

impl Index<usize> for FieldElement28 {
    type Output = u32;
    fn index(&self, a: usize) -> &Self::Output {
        &self.0[a]
    }
}

impl IndexMut<usize> for FieldElement28 {
    fn index_mut(&mut self, a: usize) -> &mut Self::Output {
        &mut self.0[a]
    }
}
impl Mul<FieldElement28> for FieldElement28 {
    type Output = FieldElement28;
    fn mul(self, rhs: FieldElement28) -> Self::Output {
        karatsuba::mul(&self, &rhs)
    }
}
impl Mul<&FieldElement28> for FieldElement28 {
    type Output = FieldElement28;
    fn mul(self, rhs: &FieldElement28) -> Self::Output {
        karatsuba::mul(&self, rhs)
    }
}

impl Add<FieldElement28> for FieldElement28 {
    type Output = FieldElement28;
    fn add(self, rhs: FieldElement28) -> Self::Output {
        let mut inter_res = self.add_no_reduce(&rhs);
        inter_res.weak_reduce();
        inter_res
    }
}
impl Sub<FieldElement28> for FieldElement28 {
    type Output = FieldElement28;
    fn sub(mut self, rhs: FieldElement28) -> Self::Output {
        self = FieldElement28::bias(&self, 2);
        let mut inter_res = self.sub_no_reduce(&rhs);
        inter_res.weak_reduce();
        inter_res
    }
}

impl ConditionallyNegatable for FieldElement28 {
    fn conditional_negate(&mut self, choice: Choice) {
        let self_neg = self.clone().negate();
        self.conditional_assign(&self_neg, choice);
    }
}

impl ConditionallySelectable for FieldElement28 {
    fn conditional_select(
        a: &FieldElement28,
        b: &FieldElement28,
        choice: Choice,
    ) -> FieldElement28 {
        FieldElement28([
            u32::conditional_select(&a.0[0], &b.0[0], choice),
            u32::conditional_select(&a.0[1], &b.0[1], choice),
            u32::conditional_select(&a.0[2], &b.0[2], choice),
            u32::conditional_select(&a.0[3], &b.0[3], choice),
            u32::conditional_select(&a.0[4], &b.0[4], choice),
            u32::conditional_select(&a.0[5], &b.0[5], choice),
            u32::conditional_select(&a.0[6], &b.0[6], choice),
            u32::conditional_select(&a.0[7], &b.0[7], choice),
            u32::conditional_select(&a.0[8], &b.0[8], choice),
            u32::conditional_select(&a.0[9], &b.0[9], choice),
            u32::conditional_select(&a.0[10], &b.0[10], choice),
            u32::conditional_select(&a.0[11], &b.0[11], choice),
            u32::conditional_select(&a.0[12], &b.0[12], choice),
            u32::conditional_select(&a.0[13], &b.0[13], choice),
            u32::conditional_select(&a.0[14], &b.0[14], choice),
            u32::conditional_select(&a.0[15], &b.0[15], choice),
        ])
    }
}

impl Default for FieldElement28 {
    fn default() -> FieldElement28 {
        FieldElement28::zero()
    }
}

///
/// Constants
///

impl FieldElement28 {
    pub const fn zero() -> FieldElement28 {
        FieldElement28([0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0])
    }
    pub const fn one() -> FieldElement28 {
        FieldElement28([1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0])
    }
    pub fn minus_one() -> FieldElement28 {
        FieldElement28([
            268435454, 268435455, 268435455, 268435455, 268435455, 268435455, 268435455, 268435455,
            268435454, 268435455, 268435455, 268435455, 268435455, 268435455, 268435455, 268435455,
        ])
    }
}

///
/// Checks
///
impl FieldElement28 {
    pub fn is_negative(&self) -> Choice {
        let bytes = self.to_bytes();
        (bytes[0] & 1).into()
    }
}

///
/// Serialisation
///
impl FieldElement28 {
    /// Helper function for internally constructing a field element
    pub(crate) const fn from_raw_slice(slice: [u32; 16]) -> FieldElement28 {
        FieldElement28(slice)
    }

    /// This does not check if the encoding is canonical (ie if the input is reduced)
    /// We parse in chunks of 56 bytes, the first 28 bytes will contain the i'th limb
    /// and the second 28 bytes will contain the (2i+1)'th limb
    pub(crate) fn from_bytes(bytes: &[u8; 56]) -> FieldElement28 {
        let load7 = |input: &[u8]| -> u64 {
            (input[0] as u64)
                | ((input[1] as u64) << 8)
                | ((input[2] as u64) << 16)
                | ((input[3] as u64) << 24)
                | ((input[4] as u64) << 32)
                | ((input[5] as u64) << 40)
                | ((input[6] as u64) << 48)
        };

        const MASK: u32 = (1 << 28) - 1;
        let mut res = FieldElement28::zero();
        for i in 0..8 {
            // Load i'th 56 bytes
            let out = load7(&bytes[i * 7..]);
            // Process two 28-bit limbs
            res[2 * i] = (out as u32) & MASK;
            res[2 * i + 1] = (out >> 28) as u32;
        }

        res
    }

    // We encode the Field element by storing each consecutive into a u64
    pub(crate) fn to_bytes(&self) -> [u8; 56] {
        // Reduce element to a canonical representation.
        let mut limbs = self.clone();
        limbs.strong_reduce();

        let mut res = [0u8; 56];

        for i in 0..8 {
            let mut l = (limbs[2 * i] as u64) + ((limbs[2 * i + 1] as u64) << 28);

            for j in 0..7 {
                res[7 * i + j] = l as u8;
                l >>= 8;
            }
        }
        res
    }
}

impl FieldElement28 {
    /// Sqaures a field element
    pub(crate) fn square(&self) -> FieldElement28 {
        karatsuba::square(self)
    }

    /// Negates a field element
    pub(crate) fn negate(&self) -> FieldElement28 {
        FieldElement28::zero() - *self
    }

    /// Bias adds 'b' multiples of `p` to self
    pub(crate) fn bias(a: &FieldElement28, b: u32) -> FieldElement28 {
        const MASK: u32 = (1 << 28) - 1;

        let co1 = b * MASK;
        let co2 = co1 - b;

        let lo = [co1; 4];
        let hi = [co2, co1, co1, co1];

        FieldElement28([
            a[0].wrapping_add(lo[0]),
            a[1].wrapping_add(lo[1]),
            a[2].wrapping_add(lo[2]),
            a[3].wrapping_add(lo[3]),
            a[4].wrapping_add(lo[0]),
            a[5].wrapping_add(lo[1]),
            a[6].wrapping_add(lo[2]),
            a[7].wrapping_add(lo[3]),
            a[8].wrapping_add(hi[0]),
            a[9].wrapping_add(hi[1]),
            a[10].wrapping_add(hi[2]),
            a[11].wrapping_add(hi[3]),
            a[12].wrapping_add(lo[0]),
            a[13].wrapping_add(lo[1]),
            a[14].wrapping_add(lo[2]),
            a[15].wrapping_add(lo[3]),
        ])
    }

    pub(crate) fn weak_reduce(&mut self) {
        const MASK: u32 = (1 << 28) - 1;

        let limb16_mod = self[16 - 1] >> 28;
        self[16 / 2] += limb16_mod;

        self[15] = (self[15] & MASK) + (self[14] >> 28);
        self[14] = (self[14] & MASK) + (self[13] >> 28);
        self[13] = (self[13] & MASK) + (self[12] >> 28);
        self[12] = (self[12] & MASK) + (self[11] >> 28);
        self[11] = (self[11] & MASK) + (self[10] >> 28);
        self[10] = (self[10] & MASK) + (self[9] >> 28);
        self[9] = (self[9] & MASK) + (self[8] >> 28);
        self[8] = (self[8] & MASK) + (self[7] >> 28);
        self[7] = (self[7] & MASK) + (self[6] >> 28);
        self[6] = (self[6] & MASK) + (self[5] >> 28);
        self[5] = (self[5] & MASK) + (self[4] >> 28);
        self[4] = (self[4] & MASK) + (self[3] >> 28);
        self[3] = (self[3] & MASK) + (self[2] >> 28);
        self[2] = (self[2] & MASK) + (self[1] >> 28);
        self[1] = (self[1] & MASK) + (self[0] >> 28);

        self[0] = (self[0] & MASK) + limb16_mod;
    }

    /// Reduces the field element to a canonical representation
    /// This is used when checking equality between two field elements and
    /// when encoding a field element
    pub(crate) fn strong_reduce(&mut self) {
        const MASK: u32 = (1 << 28) - 1;

        // After weak reducing, we can make the guarantee that
        // 0 < self < 2p
        self.weak_reduce();

        fn sub_carry(a: u32, b: i64) -> i64 {
            let x = a as i64;
            x - b
        }
        fn add_carry(a: u32, b: u64) -> u64 {
            let x = a as u64;
            x + b
        }

        // We know that  0 <= self < 2p
        // This next section computes self - p
        // Our range for self would become -p <= self < p

        let mut scarry = 0i64;
        scarry += sub_carry(self[0], 0xfffffff);
        self[0] = (scarry as u32) & MASK;
        scarry >>= 28;
        scarry += sub_carry(self[1], 0xfffffff);
        self[1] = (scarry as u32) & MASK;
        scarry >>= 28;
        scarry += sub_carry(self[2], 0xfffffff);
        self[2] = (scarry as u32) & MASK;
        scarry >>= 28;
        scarry += sub_carry(self[3], 0xfffffff);
        self[3] = (scarry as u32) & MASK;
        scarry >>= 28;
        scarry += sub_carry(self[4], 0xfffffff);
        self[4] = (scarry as u32) & MASK;
        scarry >>= 28;
        scarry += sub_carry(self[5], 0xfffffff);
        self[5] = (scarry as u32) & MASK;
        scarry >>= 28;
        scarry += sub_carry(self[6], 0xfffffff);
        self[6] = (scarry as u32) & MASK;
        scarry >>= 28;
        scarry += sub_carry(self[7], 0xfffffff);
        self[7] = (scarry as u32) & MASK;
        scarry >>= 28;
        scarry += sub_carry(self[8], 0xffffffe);
        self[8] = (scarry as u32) & MASK;
        scarry >>= 28;
        scarry += sub_carry(self[9], 0xfffffff);
        self[9] = (scarry as u32) & MASK;
        scarry >>= 28;
        scarry += sub_carry(self[10], 0xfffffff);
        self[10] = (scarry as u32) & MASK;
        scarry >>= 28;
        scarry += sub_carry(self[11], 0xfffffff);
        self[11] = (scarry as u32) & MASK;
        scarry >>= 28;
        scarry += sub_carry(self[12], 0xfffffff);
        self[12] = (scarry as u32) & MASK;
        scarry >>= 28;
        scarry += sub_carry(self[13], 0xfffffff);
        self[13] = (scarry as u32) & MASK;
        scarry >>= 28;
        scarry += sub_carry(self[14], 0xfffffff);
        self[14] = (scarry as u32) & MASK;
        scarry >>= 28;
        scarry += sub_carry(self[15], 0xfffffff);
        self[15] = (scarry as u32) & MASK;
        scarry >>= 28;

        // There are two cases to consider; either the value was >= p or it was <less than p
        // Case 1:
        // If the value was more than p, then the final borrow will be zero. This is scarry.
        // Case 2:
        // If the  value was less than p, the final borrow will be -1.

        // The only two possibilities for the borrow bit is -1 or 0.
        assert!(scarry == 0 || scarry + 1 == 0);

        let scarry_mask = (scarry as u32) & MASK;
        let mut carry = 0u64;
        let m = scarry_mask as u64;

        // Carry propagation

        carry += add_carry(self[0], m);
        self[0] = (carry as u32) & MASK;
        carry >>= 28;
        carry += add_carry(self[1], m);
        self[1] = (carry as u32) & MASK;
        carry >>= 28;
        carry += add_carry(self[2], m);
        self[2] = (carry as u32) & MASK;
        carry >>= 28;
        carry += add_carry(self[3], m);
        self[3] = (carry as u32) & MASK;
        carry >>= 28;
        carry += add_carry(self[4], m);
        self[4] = (carry as u32) & MASK;
        carry >>= 28;
        carry += add_carry(self[5], m);
        self[5] = (carry as u32) & MASK;
        carry >>= 28;
        carry += add_carry(self[6], m);
        self[6] = (carry as u32) & MASK;
        carry >>= 28;
        carry += add_carry(self[7], m);
        self[7] = (carry as u32) & MASK;
        carry >>= 28;
        carry += add_carry(self[8], m & 0xfffffffffffffffe);
        self[8] = (carry as u32) & MASK;
        carry >>= 28;
        carry += add_carry(self[9], m);
        self[9] = (carry as u32) & MASK;
        carry >>= 28;
        carry += add_carry(self[10], m);
        self[10] = (carry as u32) & MASK;
        carry >>= 28;
        carry += add_carry(self[11], m);
        self[11] = (carry as u32) & MASK;
        carry >>= 28;
        carry += add_carry(self[12], m);
        self[12] = (carry as u32) & MASK;
        carry >>= 28;
        carry += add_carry(self[13], m);
        self[13] = (carry as u32) & MASK;
        carry >>= 28;
        carry += add_carry(self[14], m);
        self[14] = (carry as u32) & MASK;
        carry >>= 28;
        carry += add_carry(self[15], m);
        self[15] = (carry as u32) & MASK;

        // This last shift is only needed for the asserts that follow
        carry >>= 28;
        // carry can only be 1 or 0 depending on the previous borrow bit
        assert!(carry < 2);
        // carry + borrow should cancel out
        assert!((carry as i64) + (scarry as i64) == 0);
    }

    /// Adds two field elements together
    /// Result is not reduced
    pub(crate) fn add_no_reduce(&self, rhs: &FieldElement28) -> FieldElement28 {
        FieldElement28([
            self[0] + rhs[0],
            self[1] + rhs[1],
            self[2] + rhs[2],
            self[3] + rhs[3],
            self[4] + rhs[4],
            self[5] + rhs[5],
            self[6] + rhs[6],
            self[7] + rhs[7],
            self[8] + rhs[8],
            self[9] + rhs[9],
            self[10] + rhs[10],
            self[11] + rhs[11],
            self[12] + rhs[12],
            self[13] + rhs[13],
            self[14] + rhs[14],
            self[15] + rhs[15],
        ])
    }

    /// Subtracts the two field elements from each other
    /// Result is not reduced
    pub(crate) fn sub_no_reduce(&self, rhs: &FieldElement28) -> FieldElement28 {
        FieldElement28([
            self[0].wrapping_sub(rhs[0]),
            self[1].wrapping_sub(rhs[1]),
            self[2].wrapping_sub(rhs[2]),
            self[3].wrapping_sub(rhs[3]),
            self[4].wrapping_sub(rhs[4]),
            self[5].wrapping_sub(rhs[5]),
            self[6].wrapping_sub(rhs[6]),
            self[7].wrapping_sub(rhs[7]),
            self[8].wrapping_sub(rhs[8]),
            self[9].wrapping_sub(rhs[9]),
            self[10].wrapping_sub(rhs[10]),
            self[11].wrapping_sub(rhs[11]),
            self[12].wrapping_sub(rhs[12]),
            self[13].wrapping_sub(rhs[13]),
            self[14].wrapping_sub(rhs[14]),
            self[15].wrapping_sub(rhs[15]),
        ])
    }
}
