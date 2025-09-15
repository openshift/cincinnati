use crate::constants::TWISTED_D;
use crate::curve::twedwards::{extended::ExtendedPoint, extensible::ExtensiblePoint};
use crate::field::FieldElement;
use subtle::{Choice, ConditionallySelectable};

/// This point representation is not a part of the API.
/// AffinePoint is mainly used as a convenience struct.
/// XXX: Initially, I wanted to leave some of these in the library to help
/// others learn. So if you are scrubbing the commit history. Hopefully they were helpful.

/// Represents an AffinePoint on the Twisted Edwards Curve
/// with Equation y^2 - x^2 = 1 - (TWISTED_D) * x^2 * y^2
#[derive(PartialEq, Eq)]
pub struct AffinePoint {
    pub(crate) x: FieldElement,
    pub(crate) y: FieldElement,
}

impl Default for AffinePoint {
    fn default() -> AffinePoint {
        AffinePoint::identity()
    }
}

impl AffinePoint {
    /// Identity element
    pub(crate) fn identity() -> AffinePoint {
        AffinePoint {
            x: FieldElement::zero(),
            y: FieldElement::one(),
        }
    }
    /// Checks if the AffinePoint is on the TwistedEdwards curve
    fn is_on_curve(&self) -> bool {
        let xx = self.x.square();
        let yy = self.y.square();

        yy - xx == FieldElement::one() + (TWISTED_D * xx * yy)
    }
    // Negates an AffinePoint
    pub(crate) fn negate(&self) -> AffinePoint {
        AffinePoint {
            x: self.x.negate(),
            y: self.y,
        }
    }
    /// Adds an AffinePoint onto an AffinePoint
    pub(crate) fn add(&self, other: &AffinePoint) -> AffinePoint {
        let y_numerator = self.y * other.y + self.x * other.x;
        let y_denominator = FieldElement::one() - TWISTED_D * self.x * other.x * self.y * other.y;

        let x_numerator = self.x * other.y + self.y * other.x;
        let x_denominator = FieldElement::one() + TWISTED_D * self.x * other.x * self.y * other.y;

        let x = x_numerator * x_denominator.invert();
        let y = y_numerator * y_denominator.invert();
        AffinePoint { x, y }
    }
    /// Converts an AffinePoint to an ExtensiblePoint
    pub(crate) fn to_extensible(&self) -> ExtensiblePoint {
        ExtensiblePoint {
            X: self.x,
            Y: self.y,
            Z: FieldElement::one(),
            T1: self.x,
            T2: self.y,
        }
    }
    /// Converts an AffinePoint to an AffineNielsPoint
    pub(crate) fn to_affine_niels(&self) -> AffineNielsPoint {
        AffineNielsPoint {
            y_plus_x: self.y + self.x,
            y_minus_x: self.y - self.x,
            td: self.x * self.y * TWISTED_D,
        }
    }
    /// Converts an An AffinePoint to an ExtendedPoint
    pub(crate) fn to_extended(&self) -> ExtendedPoint {
        self.to_extensible().to_extended()
    }
}

/// Represents a PreComputed or Cached AffinePoint
///  ((y+x)/2, (y-x)/2, dxy)
#[derive(Copy, Clone)]
pub struct AffineNielsPoint {
    pub(crate) y_plus_x: FieldElement,
    pub(crate) y_minus_x: FieldElement,
    pub(crate) td: FieldElement,
}

impl ConditionallySelectable for AffineNielsPoint {
    fn conditional_select(a: &Self, b: &Self, choice: Choice) -> Self {
        AffineNielsPoint {
            y_plus_x: FieldElement::conditional_select(&a.y_plus_x, &b.y_plus_x, choice),
            y_minus_x: FieldElement::conditional_select(&a.y_minus_x, &b.y_minus_x, choice),
            td: FieldElement::conditional_select(&a.td, &b.td, choice),
        }
    }
}

impl AffineNielsPoint {
    /// Checks if two AffineNielsPoints are equal
    /// Returns true if they are
    pub(crate) fn equals(&self, other: &AffineNielsPoint) -> bool {
        (self.y_minus_x == other.y_minus_x)
            && (self.y_plus_x == other.y_plus_x)
            && (self.td == other.td)
    }

    /// Returns the identity element for an AffineNielsPoint
    pub(crate) fn identity() -> AffineNielsPoint {
        AffineNielsPoint {
            y_plus_x: FieldElement::one(),
            y_minus_x: FieldElement::one(),
            td: FieldElement::zero(),
        }
    }
    /// Converts an AffineNielsPoint to an ExtendedPoint
    pub(crate) fn to_extended(&self) -> ExtendedPoint {
        ExtendedPoint {
            X: self.y_plus_x - self.y_minus_x,
            Y: self.y_minus_x + self.y_plus_x,
            Z: FieldElement::one(),
            T: self.y_plus_x * self.y_minus_x,
        }
    }
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn test_negation() {
        use crate::constants::TWISTED_EDWARDS_BASE_POINT;
        let a = TWISTED_EDWARDS_BASE_POINT.to_affine();
        assert!(a.is_on_curve());

        let neg_a = a.negate();
        let got = neg_a.add(&a);
        assert!(got == AffinePoint::identity());
    }
}
