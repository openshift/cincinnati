use crate::curve::twedwards::extended::ExtendedPoint;
use std::fmt;
use subtle::{Choice, ConstantTimeEq};

pub struct RistrettoPoint(ExtendedPoint);

#[derive(Copy, Clone)]
pub struct CompressedRistretto([u8; 56]);

impl fmt::Debug for CompressedRistretto {
    fn fmt(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        self.0[..].fmt(formatter)
    }
}

impl ConstantTimeEq for CompressedRistretto {
    fn ct_eq(&self, other: &CompressedRistretto) -> Choice {
        self.as_bytes().ct_eq(other.as_bytes())
    }
}

impl PartialEq for CompressedRistretto {
    fn eq(&self, other: &CompressedRistretto) -> bool {
        self.ct_eq(other).into()
    }
}
impl Eq for CompressedRistretto {}

impl CompressedRistretto {
    pub fn as_bytes(&self) -> &[u8] {
        &self.0
    }
}

impl RistrettoPoint {
    pub fn identity() -> RistrettoPoint {
        RistrettoPoint(ExtendedPoint::identity())
    }

    pub fn equals(&self, other: &RistrettoPoint) -> bool {
        let XY = self.0.X * other.0.Y;
        let YX = self.0.Y * other.0.X;
        XY == YX
    }

    pub fn encode(&self) -> CompressedRistretto {
        todo!()
    }
}

impl CompressedRistretto {
    pub fn identity() -> CompressedRistretto {
        CompressedRistretto([0; 56])
    }

    pub fn decode(&self) -> Option<RistrettoPoint> {
        todo!()
    }
}
