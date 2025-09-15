use crate::{Scalar, constants::{DECAF_BASEPOINT, DECAF_FACTOR, NEG_EDWARDS_D, NEG_FOUR_TIMES_TWISTED_D}, curve::scalar_mul::double_and_add};
use crate::curve::twedwards::extended::ExtendedPoint;
use crate::field::FieldElement;
use std::{fmt, ops::Mul};
use subtle::{Choice, ConditionallyNegatable, ConstantTimeEq};

pub struct DecafPoint(pub(crate) ExtendedPoint);

impl<'s, 'p> Mul<&'s Scalar> for &'p DecafPoint {
    type Output = DecafPoint;
    fn mul(self, scalar: &'s Scalar) -> DecafPoint {
        // XXX: We can do better than double and add
        DecafPoint(double_and_add(&self.0, &scalar))
    }
}
impl<'p, 's> Mul<&'p DecafPoint> for &'s Scalar {
    type Output = DecafPoint;
    fn mul(self, point: &'p DecafPoint) -> DecafPoint {
        DecafPoint(double_and_add(&point.0,self))
    }
}
impl Mul<DecafPoint> for Scalar {
    type Output = DecafPoint;
    fn mul(self, point: DecafPoint) -> DecafPoint {
        DecafPoint(double_and_add(&point.0,&self))
    }
}
impl Mul<Scalar> for DecafPoint {
    type Output = DecafPoint;
    fn mul(self, scalar : Scalar) -> DecafPoint {
        DecafPoint(double_and_add(&self.0, &scalar))
    }
}

#[derive(Copy, Clone)]
pub struct CompressedDecaf(pub [u8; 56]);

impl fmt::Debug for CompressedDecaf {
    fn fmt(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        self.0[..].fmt(formatter)
    }
}

impl ConstantTimeEq for CompressedDecaf {
    fn ct_eq(&self, other: &CompressedDecaf) -> Choice {
        self.as_bytes().ct_eq(other.as_bytes())
    }
}

impl PartialEq for CompressedDecaf {
    fn eq(&self, other: &CompressedDecaf) -> bool {
        self.ct_eq(other).into()
    }
}
impl Eq for CompressedDecaf {}

impl CompressedDecaf {
    pub fn as_bytes(&self) -> &[u8] {
        &self.0
    }
}

impl DecafPoint {
    pub fn identity() -> DecafPoint {
        DecafPoint(ExtendedPoint::identity())
    }

    pub const fn generator() -> DecafPoint {
        DECAF_BASEPOINT
    }

    pub fn equals(&self, other: &DecafPoint) -> bool {
        self.0.X * other.0.Y == self.0.Y * other.0.X
    }

    pub fn add(&self, other: &DecafPoint) -> DecafPoint {
        DecafPoint(self.0.to_extensible().add_extended(&other.0).to_extended())
    }

    // This will be simpler than the curve2519 case, as there is no need to lift the points
    // XXX: Using the twisted edwards coordinates, for this a = -1 and d = EDWARDS_D-1, but we can simply use the EDWARDS constants when simplified
    // XXX: Should we call these functions compress and decompress?
    pub fn encode(&self) -> CompressedDecaf {
        let X = self.0.X;
        let Y = self.0.Y;
        let Z = self.0.Z;
        let T = self.0.T;

        let XX_TT = (X + T) * (X - T);

        let (isr, _) = (X.square() * XX_TT * NEG_EDWARDS_D).inverse_square_root();
        let mut ratio = isr * XX_TT;
        let altx = ratio * DECAF_FACTOR; // Sign choice
        ratio.conditional_negate(altx.is_negative());
        let k = ratio * Z - T;

        let mut s = k * NEG_EDWARDS_D * isr * X;
        s.conditional_negate(s.is_negative());

        CompressedDecaf(s.to_bytes())
    }
}

impl CompressedDecaf {
    pub fn identity() -> CompressedDecaf {
        CompressedDecaf([0; 56])
    }

    // XXX: We allow the identity point
    /// XXX: Clean this up to be more descriptive of what is happening
    pub fn decode(&self) -> Option<DecafPoint> {
        let s = FieldElement::from_bytes(&self.0);
        //XX: Check for canonical encoding and sign,
        // Copied this check from Dalek: The From_bytes function does not throw an error, if the bytes exceed the prime.
        // However, to_bytes reduces the Field element before serialising
        // So we can use to_bytes -> from_bytes and if the representations are the same, then the element was already in reduced form
        let s_bytes_check = s.to_bytes();
        let s_encoding_is_canonical = &s_bytes_check[..].ct_eq(&self.0);
        let s_is_negative = s.is_negative();
        if s_encoding_is_canonical.unwrap_u8() == 0u8 || s.is_negative().unwrap_u8() == 1u8 {
            return None;
        }

        let ss = s.square();
        let u1 = FieldElement::one() - ss;
        let u2 = FieldElement::one() + ss;
        let u1_sqr = u1.square();

        let v = ss * (NEG_FOUR_TIMES_TWISTED_D) + u1_sqr; // XXX: constantify please

        let (I, ok) = (v * u1_sqr).inverse_square_root();
        if !ok {
            return None;
        }

        let Dx = I * u1;
        let Dxs = (s + s) * Dx;

        let mut X = (Dxs * I) * v;
        let k = Dxs * DECAF_FACTOR;
        X.conditional_negate(k.is_negative());

        let Y = Dx * u2;
        let Z = FieldElement::one();
        let T = X * Y;

        Some(DecafPoint(ExtendedPoint { X: X, Y, Z, T }))
    }
}

mod test {
    use super::*;
    use crate::constants::TWISTED_EDWARDS_BASE_POINT;

    #[test]
    fn test_edwards_ristretto_operations() {
        // Basic test that if P1 + P2 = P3
        // Then Decaf(P1) + Decaf(P2) = Decaf(P3)

        let P = TWISTED_EDWARDS_BASE_POINT;

        let P2 = P.double();
        let P3 = P2.to_extensible().add_extended(&P).to_extended();

        // Encode and decode to make them Decaf points
        let Decaf_P = DecafPoint(P).encode().decode().unwrap();
        let Decaf_P2 = DecafPoint(P2).encode().decode().unwrap();
        let expected_Decaf_P3 = DecafPoint(P3).encode().decode().unwrap();

        // Adding the DecafPoint should be the same as adding the Edwards points and encoding the result as Decaf
        let Decaf_P3 = Decaf_P.add(&Decaf_P2);

        assert!(Decaf_P3.equals(&expected_Decaf_P3));
    }

    #[test]
    fn test_identity() {
        // Basic test to check the identity is being encoded properly
        let compress_identity = DecafPoint::identity().encode();
        assert!(compress_identity == CompressedDecaf::identity())
    }

    #[test]
    fn test_vectors_lib_decaf() {
        // Testing small multiples of basepoint. Taken from reference implementation.
        let compressed = [
            // Taken from libdecaf, where they were computed using SAGE script
            CompressedDecaf([
                0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            ]),
            CompressedDecaf([
                102, 102, 102, 102, 102, 102, 102, 102, 102, 102, 102, 102, 102, 102, 102, 102,
                102, 102, 102, 102, 102, 102, 102, 102, 102, 102, 102, 102, 51, 51, 51, 51, 51, 51,
                51, 51, 51, 51, 51, 51, 51, 51, 51, 51, 51, 51, 51, 51, 51, 51, 51, 51, 51, 51, 51,
                51,
            ]),
            CompressedDecaf([
                200, 152, 235, 79, 135, 249, 124, 86, 76, 111, 214, 31, 199, 228, 150, 137, 49, 74,
                31, 129, 142, 200, 94, 235, 59, 213, 81, 74, 200, 22, 211, 135, 120, 246, 158, 243,
                71, 168, 159, 202, 129, 126, 102, 222, 253, 237, 206, 23, 140, 124, 199, 9, 178,
                17, 110, 117,
            ]),
            CompressedDecaf([
                160, 192, 155, 242, 186, 114, 8, 253, 160, 244, 191, 227, 208, 245, 178, 154, 84,
                48, 18, 48, 109, 67, 131, 27, 90, 220, 111, 231, 248, 89, 111, 163, 8, 118, 61,
                177, 84, 104, 50, 59, 17, 207, 110, 74, 235, 140, 24, 254, 68, 103, 143, 68, 84,
                90, 105, 188,
            ]),
            CompressedDecaf([
                180, 111, 24, 54, 170, 40, 124, 10, 90, 86, 83, 240, 236, 94, 249, 233, 3, 244, 54,
                226, 28, 21, 112, 194, 154, 217, 229, 245, 150, 218, 151, 238, 175, 23, 21, 10,
                227, 11, 203, 49, 116, 208, 75, 194, 215, 18, 200, 199, 120, 157, 124, 180, 253,
                161, 56, 244,
            ]),
            CompressedDecaf([
                28, 91, 190, 207, 71, 65, 223, 170, 231, 157, 183, 45, 250, 206, 0, 234, 170, 197,
                2, 194, 6, 9, 52, 182, 234, 174, 202, 106, 32, 189, 61, 169, 224, 190, 135, 119,
                247, 208, 32, 51, 209, 177, 88, 132, 35, 34, 129, 164, 31, 199, 248, 14, 237, 4,
                175, 94,
            ]),
            CompressedDecaf([
                134, 255, 1, 130, 212, 15, 127, 158, 219, 120, 98, 81, 88, 33, 189, 103, 191, 214,
                22, 90, 60, 68, 222, 149, 215, 223, 121, 184, 119, 156, 207, 100, 96, 227, 198,
                139, 112, 193, 106, 170, 40, 15, 45, 123, 63, 34, 215, 69, 185, 122, 137, 144, 108,
                252, 71, 108,
            ]),
            CompressedDecaf([
                80, 43, 203, 104, 66, 235, 6, 240, 228, 144, 50, 186, 232, 124, 85, 76, 3, 29, 109,
                77, 45, 118, 148, 239, 191, 156, 70, 141, 72, 34, 12, 80, 248, 202, 40, 132, 51,
                100, 215, 12, 238, 146, 214, 254, 36, 110, 97, 68, 143, 157, 185, 128, 139, 59, 36,
                8,
            ]),
            CompressedDecaf([
                12, 152, 16, 241, 226, 235, 211, 137, 202, 167, 137, 55, 77, 120, 0, 121, 116, 239,
                77, 23, 34, 115, 22, 244, 14, 87, 139, 51, 104, 39, 218, 63, 107, 72, 42, 71, 148,
                235, 106, 57, 117, 185, 113, 181, 225, 56, 143, 82, 233, 30, 162, 241, 188, 176,
                249, 18,
            ]),
            CompressedDecaf([
                32, 212, 29, 133, 161, 141, 86, 87, 162, 150, 64, 50, 21, 99, 187, 208, 76, 47,
                251, 208, 163, 122, 123, 164, 58, 79, 125, 38, 60, 226, 111, 175, 78, 31, 116, 249,
                244, 181, 144, 198, 146, 41, 174, 87, 31, 227, 127, 166, 57, 181, 184, 235, 72,
                189, 154, 85,
            ]),
            CompressedDecaf([
                230, 180, 184, 244, 8, 199, 1, 13, 6, 1, 231, 237, 160, 195, 9, 161, 164, 39, 32,
                214, 208, 107, 87, 89, 253, 196, 225, 239, 226, 45, 7, 109, 108, 68, 212, 47, 80,
                141, 103, 190, 70, 41, 20, 210, 139, 142, 220, 227, 46, 112, 148, 48, 81, 100, 175,
                23,
            ]),
            CompressedDecaf([
                190, 136, 187, 184, 108, 89, 193, 61, 142, 157, 9, 171, 152, 16, 95, 105, 194, 209,
                221, 19, 77, 188, 211, 176, 134, 54, 88, 245, 49, 89, 219, 100, 192, 225, 57, 209,
                128, 243, 200, 155, 130, 150, 208, 174, 50, 68, 25, 192, 111, 168, 127, 199, 218,
                175, 52, 193,
            ]),
            CompressedDecaf([
                164, 86, 249, 54, 151, 105, 232, 240, 137, 2, 18, 74, 3, 20, 199, 160, 101, 55,
                160, 110, 50, 65, 31, 79, 147, 65, 89, 80, 161, 123, 173, 250, 116, 66, 182, 33,
                116, 52, 163, 160, 94, 244, 91, 229, 241, 11, 215, 178, 239, 142, 160, 12, 67, 30,
                222, 197,
            ]),
            CompressedDecaf([
                24, 110, 69, 44, 68, 102, 170, 67, 131, 180, 192, 2, 16, 213, 46, 121, 34, 219,
                249, 119, 30, 139, 71, 226, 41, 169, 183, 183, 60, 141, 16, 253, 126, 240, 182,
                228, 21, 48, 249, 31, 36, 163, 237, 154, 183, 31, 163, 139, 152, 178, 254, 71, 70,
                213, 29, 104,
            ]),
            CompressedDecaf([
                74, 231, 253, 202, 233, 69, 63, 25, 90, 142, 173, 92, 190, 26, 123, 150, 153, 103,
                59, 82, 196, 10, 178, 121, 39, 70, 72, 135, 190, 83, 35, 127, 127, 58, 33, 185, 56,
                212, 13, 14, 201, 225, 91, 29, 81, 48, 177, 63, 254, 216, 19, 115, 165, 62, 43, 67,
            ]),
            CompressedDecaf([
                132, 25, 129, 195, 191, 238, 195, 246, 12, 254, 202, 117, 217, 216, 220, 23, 244,
                108, 240, 16, 111, 36, 34, 181, 154, 236, 88, 10, 88, 243, 66, 39, 46, 58, 94, 87,
                90, 5, 93, 219, 5, 19, 144, 197, 76, 36, 198, 236, 177, 224, 172, 235, 7, 95, 96,
                86,
            ]),
        ];
        let mut point = DecafPoint::identity();
        let generator = DecafPoint::generator();
        for compressed_point in compressed.iter() {
            assert_eq!(&point.encode(), compressed_point);
            point = point.add(&generator);
        }
    }
}
