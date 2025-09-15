use crate::decaf::DecafPoint;

#[cfg(feature = "u32_backend")]
pub(crate) use crate::field::u32::constants::*;

#[cfg(feature = "fiat_u64_backend")]
pub(crate) use crate::field::fiat_u64::constants::*;

pub(crate) const DECAF_BASEPOINT: DecafPoint = DecafPoint(TWISTED_EDWARDS_BASE_POINT);
