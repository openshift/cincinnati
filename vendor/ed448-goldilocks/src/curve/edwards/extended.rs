use crate::constants::EDWARDS_D;
use crate::curve::edwards::affine::AffinePoint;
use crate::curve::montgomery::montgomery::MontgomeryPoint; // XXX: need to fix this path
use crate::curve::scalar_mul::variable_base;
use crate::curve::twedwards::extended::ExtendedPoint as TwistedExtendedPoint;
use crate::field::{FieldElement, Scalar};
use subtle::{Choice, ConditionallyNegatable, ConditionallySelectable, ConstantTimeEq};

/// Represent points on the (untwisted) edwards curve using Extended Homogenous Projective Co-ordinates
/// (x, y) -> (X/Z, Y/Z, Z, T)
/// a = 1, d = -39081
/// XXX: Make this more descriptive
/// Should this be renamed to EdwardsPoint so that we are consistent with Dalek crypto? Necessary as ExtendedPoint is not regular lingo?
#[derive(Copy, Clone, Debug)]
pub struct ExtendedPoint {
    pub(crate) X: FieldElement,
    pub(crate) Y: FieldElement,
    pub(crate) Z: FieldElement,
    pub(crate) T: FieldElement,
}

impl ConditionallySelectable for ExtendedPoint {
    fn conditional_select(a: &Self, b: &Self, choice: Choice) -> Self {
        ExtendedPoint {
            X: FieldElement::conditional_select(&a.X, &b.X, choice),
            Y: FieldElement::conditional_select(&a.Y, &b.Y, choice),
            Z: FieldElement::conditional_select(&a.Z, &b.Z, choice),
            T: FieldElement::conditional_select(&a.T, &b.T, choice),
        }
    }
}

pub struct CompressedEdwardsY(pub [u8; 57]);

impl CompressedEdwardsY {
    pub fn decompress(&self) -> Option<ExtendedPoint> {
        // Safe to unwrap here as the underlying data structure is a slice
        let (sign, b) = self.0.split_last().unwrap();

        let mut y_bytes: [u8; 56] = [0; 56];
        y_bytes.copy_from_slice(&b);

        // Recover x using y
        let y = FieldElement::from_bytes(&y_bytes);
        let yy = y.square();
        let dyy = EDWARDS_D * yy;
        let numerator = FieldElement::one() - yy;
        let denominator = FieldElement::one() - dyy;

        let (mut x, is_res) = FieldElement::sqrt_ratio(&numerator, &denominator);
        if !is_res {
            return None;
        }

        // Compute correct sign of x
        let compressed_sign_bit = Choice::from(sign >> 7);
        x.conditional_negate(compressed_sign_bit);

        return Some(AffinePoint { x, y }.to_extended());
    }
}

impl ConstantTimeEq for ExtendedPoint {
    fn ct_eq(&self, other: &Self) -> Choice {
        let XZ = self.X * other.Z;
        let ZX = self.Z * other.X;

        let YZ = self.Y * other.Z;
        let ZY = self.Z * other.Y;

        (XZ.ct_eq(&ZX)) & (YZ.ct_eq(&ZY))
    }
}

impl PartialEq for ExtendedPoint {
    fn eq(&self, other: &ExtendedPoint) -> bool {
        self.ct_eq(other).into()
    }
}
impl Eq for ExtendedPoint {}

impl Default for ExtendedPoint {
    fn default() -> ExtendedPoint {
        ExtendedPoint::identity()
    }
}

impl ExtendedPoint {
    /// Identity point
    pub fn identity() -> ExtendedPoint {
        ExtendedPoint {
            X: FieldElement::zero(),
            Y: FieldElement::one(),
            Z: FieldElement::one(),
            T: FieldElement::zero(),
        }
    }
    /// Generator for the prime subgroup
    pub const fn generator() -> ExtendedPoint {
        crate::constants::GOLDILOCKS_BASE_POINT
    }

    pub fn to_montgomery(&self) -> MontgomeryPoint {
        // u = y^2 * [(1-dy^2)/(1-y^2)]

        let affine = self.to_affine();

        let yy = affine.y.square();
        let dyy = EDWARDS_D * yy;

        let u = yy * (FieldElement::one() - dyy) * (FieldElement::one() - yy).invert();

        MontgomeryPoint(u.to_bytes())
    }
    /// Generic scalar multiplication to compute s*P
    pub fn scalar_mul(&self, scalar: &Scalar) -> ExtendedPoint {
        // Compute floor(s/4)
        let mut scalar_div_four = scalar.clone();
        scalar_div_four.div_by_four();

        // Use isogeny and dual isogeny to compute phi^-1((s/4) * phi(P))
        let partial_result = variable_base(&self.to_twisted(), &scalar_div_four).to_untwisted();
        // Add partial result to (scalar mod 4) * P
        partial_result.add(&self.scalar_mod_four(&scalar))
    }

    /// Returns (scalar mod 4) * P in constant time
    pub fn scalar_mod_four(&self, scalar: &Scalar) -> ExtendedPoint {
        // Compute compute (scalar mod 4)
        let s_mod_four = scalar[0] & 3;

        // Compute all possible values of (scalar mod 4) * P
        let zero_p = ExtendedPoint::identity();
        let one_p = self.clone();
        let two_p = one_p.double();
        let three_p = two_p.add(self);

        // Under the reasonable assumption that `==` is constant time
        // Then the whole function is constant time.
        // This should be cheaper than calling double_and_add or a scalar mul operation
        // as the number of possibilities are so small.
        // XXX: This claim has not been tested (although it sounds intuitive to me)
        let mut result = ExtendedPoint::identity();
        result.conditional_assign(&zero_p, Choice::from((s_mod_four == 0) as u8));
        result.conditional_assign(&one_p, Choice::from((s_mod_four == 1) as u8));
        result.conditional_assign(&two_p, Choice::from((s_mod_four == 2) as u8));
        result.conditional_assign(&three_p, Choice::from((s_mod_four == 3) as u8));

        result
    }

    // Standard compression; store Y and sign of X
    // XXX: This needs more docs and is `compress` the conventional function name? I think to_bytes/encode is?
    pub fn compress(&self) -> CompressedEdwardsY {
        let affine = self.to_affine();

        let affine_x = affine.x;
        let affine_y = affine.y;

        let mut compressed_bytes = [0u8; 57];

        let sign = affine_x.is_negative().unwrap_u8();

        let y_bytes = affine_y.to_bytes();
        for i in 0..y_bytes.len() {
            compressed_bytes[i] = y_bytes[i]
        }
        *compressed_bytes.last_mut().unwrap() = sign as u8;
        CompressedEdwardsY(compressed_bytes)
    }

    //https://iacr.org/archive/asiacrypt2008/53500329/53500329.pdf (3.1)
    // These formulas are unified, so for now we can use it for doubling. Will refactor later for speed
    pub fn add(&self, other: &ExtendedPoint) -> ExtendedPoint {
        let aXX = self.X * other.X; // aX1X2
        let dTT = EDWARDS_D * self.T * other.T; // dT1T2
        let ZZ = self.Z * other.Z; // Z1Z2
        let YY = self.Y * other.Y;

        let X = {
            let x_1 = (self.X * other.Y) + (self.Y * other.X);
            let x_2 = ZZ - dTT;
            x_1 * x_2
        };
        let Y = {
            let y_1 = YY - aXX;
            let y_2 = ZZ + dTT;
            y_1 * y_2
        };

        let T = {
            let t_1 = YY - aXX;
            let t_2 = (self.X * other.Y) + (self.Y * other.X);
            t_1 * t_2
        };

        let Z = { (ZZ - dTT) * (ZZ + dTT) };

        ExtendedPoint { X, Y, Z, T }
    }

    // XXX: See comment on addition, the formula is unified, so this will do for now
    //https://iacr.org/archive/asiacrypt2008/53500329/53500329.pdf (3.1)
    pub fn double(&self) -> ExtendedPoint {
        self.add(&self)
    }

    pub(crate) fn is_on_curve(&self) -> bool {
        let XY = self.X * self.Y;
        let ZT = self.Z * self.T;

        // Y^2 + X^2 == Z^2 - T^2 * D

        let YY = self.Y.square();
        let XX = self.X.square();
        let ZZ = self.Z.square();
        let TT = self.T.square();
        let lhs = YY + XX;
        let rhs = ZZ + TT * EDWARDS_D;

        (XY == ZT) && (lhs == rhs)
    }

    pub fn to_affine(&self) -> AffinePoint {
        let INV_Z = self.Z.invert();

        let mut x = self.X * INV_Z;
        x.strong_reduce();

        let mut y = self.Y * INV_Z;
        y.strong_reduce();

        AffinePoint { x, y }
    }
    /// Edwards_Isogeny is derived from the doubling formula
    /// XXX: There is a duplicate method in the twisted edwards module to compute the dual isogeny
    /// XXX: Not much point trying to make it generic I think. So what we can do is optimise each respective isogeny method for a=1 or a = -1 (currently, I just made it really slow and simple)
    fn edwards_isogeny(&self, a: FieldElement) -> TwistedExtendedPoint {
        // Convert to affine now, then derive extended version later
        let affine = self.to_affine();
        let x = affine.x;
        let y = affine.y;

        // Compute x
        let xy = x * y;
        let x_numerator = xy + xy;
        let x_denom = y.square() - (a * x.square());
        let new_x = x_numerator * x_denom.invert();

        // Compute y
        let y_numerator = y.square() + (a * x.square());
        let y_denom = (FieldElement::one() + FieldElement::one()) - y.square() - (a * x.square());
        let new_y = y_numerator * y_denom.invert();

        TwistedExtendedPoint {
            X: new_x,
            Y: new_y,
            Z: FieldElement::one(),
            T: new_x * new_y,
        }
    }
    pub fn to_twisted(&self) -> TwistedExtendedPoint {
        self.edwards_isogeny(FieldElement::one())
    }

    pub fn negate(&self) -> ExtendedPoint {
        ExtendedPoint {
            X: self.X.negate(),
            Y: self.Y,
            Z: self.Z,
            T: self.T.negate(),
        }
    }

    pub fn torque(&self) -> ExtendedPoint {
        ExtendedPoint {
            X: self.X.negate(),
            Y: self.Y.negate(),
            Z: self.Z,
            T: self.T,
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use hex::decode as hex_decode;
    fn slice_to_fixed_array(b: &[u8]) -> [u8; 56] {
        let mut a: [u8; 56] = [0; 56];
        a.copy_from_slice(&b);
        a
    }

    fn hex_to_field(data: &str) -> FieldElement {
        let mut bytes = hex_decode(data).unwrap();
        bytes.reverse();
        FieldElement::from_bytes(&slice_to_fixed_array(&bytes))
    }

    #[test]
    fn test_isogeny() {
        let x = hex_to_field("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa955555555555555555555555555555555555555555555555555555555");
        let y = hex_to_field("ae05e9634ad7048db359d6205086c2b0036ed7a035884dd7b7e36d728ad8c4b80d6565833a2a3098bbbcb2bed1cda06bdaeafbcdea9386ed");
        let a = AffinePoint { x, y }.to_extended();
        let twist_a = a.to_twisted().to_untwisted();
        assert!(twist_a == a.double().double())
    }

    // XXX: Move this to constants folder to test all global constants
    #[test]
    fn derive_base_points() {
        use crate::constants::{GOLDILOCKS_BASE_POINT, TWISTED_EDWARDS_BASE_POINT};

        // This was the original basepoint which had order 2q;
        let old_x = hex_to_field("4F1970C66BED0DED221D15A622BF36DA9E146570470F1767EA6DE324A3D3A46412AE1AF72AB66511433B80E18B00938E2626A82BC70CC05E");
        let old_y = hex_to_field("693F46716EB6BC248876203756C9C7624BEA73736CA3984087789C1E05A0C2D73AD3FF1CE67C39C4FDBD132C4ED7C8AD9808795BF230FA14");
        let old_bp = AffinePoint { x: old_x, y: old_y }.to_extended();

        // This is the new basepoint, that is in the ed448 paper
        let new_x = hex_to_field("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa955555555555555555555555555555555555555555555555555555555");
        let new_y = hex_to_field("ae05e9634ad7048db359d6205086c2b0036ed7a035884dd7b7e36d728ad8c4b80d6565833a2a3098bbbcb2bed1cda06bdaeafbcdea9386ed");
        let new_bp = AffinePoint { x: new_x, y: new_y }.to_extended();

        // Doubling the old basepoint, should give us the new basepoint
        assert_eq!(old_bp.double(), new_bp);

        // XXX: Unfortunately, the test vectors in libdecaf currently use the old basepoint.
        // We need to update this. But for now, I use the old basepoint so that I can check against libdecaf

        assert_eq!(GOLDILOCKS_BASE_POINT, old_bp);

        // The Twisted basepoint can be derived by using the isogeny
        assert_eq!(old_bp.to_twisted(), TWISTED_EDWARDS_BASE_POINT)
    }

    #[test]
    fn test_is_on_curve() {
        let x  = hex_to_field("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa955555555555555555555555555555555555555555555555555555555");
        let y  = hex_to_field("ae05e9634ad7048db359d6205086c2b0036ed7a035884dd7b7e36d728ad8c4b80d6565833a2a3098bbbcb2bed1cda06bdaeafbcdea9386ed");
        let gen = AffinePoint { x, y }.to_extended();
        assert!(gen.is_on_curve());
    }
    #[test]
    fn test_compress_decompress() {
        let x  = hex_to_field("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa955555555555555555555555555555555555555555555555555555555");
        let y  = hex_to_field("ae05e9634ad7048db359d6205086c2b0036ed7a035884dd7b7e36d728ad8c4b80d6565833a2a3098bbbcb2bed1cda06bdaeafbcdea9386ed");
        let gen = AffinePoint { x, y }.to_extended();

        let decompressed_point = gen.compress().decompress();
        assert!(decompressed_point.is_some());

        assert!(gen == decompressed_point.unwrap());
    }
}
