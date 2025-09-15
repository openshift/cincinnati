use crate::curve::edwards::ExtendedPoint;
use crate::field::FieldElement;
// Affine point on untwisted curve
// XXX: This is only really needed for convenience in extended.rs . Will remove it sooner or later
pub struct AffinePoint {
    pub(crate) x: FieldElement,
    pub(crate) y: FieldElement,
}

impl AffinePoint {
    pub fn identity() -> AffinePoint {
        AffinePoint {
            x: FieldElement::zero(),
            y: FieldElement::one(),
        }
    }
    pub fn to_extended(&self) -> ExtendedPoint {
        ExtendedPoint {
            X: self.x,
            Y: self.y,
            Z: FieldElement::one(),
            T: self.x * self.y,
        }
    }
}
