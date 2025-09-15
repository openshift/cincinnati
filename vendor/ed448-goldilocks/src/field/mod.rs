#[cfg(feature = "fiat_u64_backend")]
pub mod fiat_u64;

#[cfg(feature = "u32_backend")]
pub mod u32;

// XXX: Currently we only have one implementation for Scalar
mod scalar;
pub use crate::field::scalar::Scalar;

#[cfg(feature = "u32_backend")]
pub type FieldElement = crate::field::u32::FieldElement28;

#[cfg(feature = "fiat_u64_backend")]
pub type FieldElement = crate::field::fiat_u64::FieldElement56;

use subtle::{Choice, ConstantTimeEq};
impl ConstantTimeEq for FieldElement {
    fn ct_eq(&self, other: &Self) -> Choice {
        self.to_bytes().ct_eq(&other.to_bytes())
    }
}

impl PartialEq for FieldElement {
    fn eq(&self, other: &FieldElement) -> bool {
        self.ct_eq(&other).into()
    }
}
impl Eq for FieldElement {}

impl FieldElement {
    /// Checks if a field element is zero
    pub(crate) fn is_zero(&self) -> Choice {
        self.ct_eq(&FieldElement::zero())
    }
    /// Inverts a field element
    /// Previous chain length: 462, new length 460
    pub fn invert(&self) -> FieldElement {
        // Addition chain taken from https://github.com/mmcloughlin/addchain
        let _1 = self;
        let _10 = _1.square();
        let _11 = *_1 * _10;
        let _110 = _11.square();
        let _111 = *_1 * _110;
        let _111000 = _111.square_n(3);
        let _111111 = _111 * _111000;

        let x12 = _111111.square_n(6) * _111111;
        let x24 = x12.square_n(12) * x12;
        let i34 = x24.square_n(6);
        let x30 = _111111 * i34;
        let x48 = i34.square_n(18) * x24;
        let x96 = x48.square_n(48) * x48;
        let x192 = x96.square_n(96) * x96;
        let x222 = x192.square_n(30) * x30;
        let x223 = x222.square() * _1;

        (x223.square_n(223) * x222).square_n(2) * _1
    }
    /// Squares a field element  `n` times
    fn square_n(&self, mut n: u32) -> FieldElement {
        let mut result = self.square();

        // Decrease value by 1 since we just did a squaring
        n = n - 1;

        for _ in 0..n {
            result = result.square();
        }

        result
    }

    /// Computes the inverse square root of a field element
    /// Returns the result and a boolean to indicate whether self
    /// was a Quadratic residue
    pub(crate) fn inverse_square_root(&self) -> (FieldElement, bool) {
        let (mut l0, mut l1, mut l2) = (
            FieldElement::zero(),
            FieldElement::zero(),
            FieldElement::zero(),
        );

        l1 = self.square();
        l2 = l1 * self;
        l1 = l2.square();
        l2 = l1 * self;
        l1 = l2.square_n(3);
        l0 = l2 * l1;
        l1 = l0.square_n(3);
        l0 = l2 * l1;
        l2 = l0.square_n(9);
        l1 = l0 * l2;
        l0 = l1 * l1;
        l2 = l0 * self;
        l0 = l2.square_n(18);
        l2 = l1 * l0;
        l0 = l2.square_n(37);
        l1 = l2 * l0;
        l0 = l1.square_n(37);
        l1 = l2 * l0;
        l0 = l1.square_n(111);
        l2 = l1 * l0;
        l0 = l2.square();
        l1 = l0 * self;
        l0 = l1.square_n(223);
        l1 = l2 * l0;
        l2 = l1.square();
        l0 = l2 * self;

        let is_residue = l0 == FieldElement::one();
        (l1, is_residue)
    }

    /// Computes the square root ratio of two elements
    pub(crate) fn sqrt_ratio(u: &FieldElement, v: &FieldElement) -> (FieldElement, bool) {
        let x = *u * v;
        let (inv_sqrt_x, is_res) = x.inverse_square_root();
        (inv_sqrt_x * u, is_res)
    }
}
