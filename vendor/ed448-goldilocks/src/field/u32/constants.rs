// Move all curve specific constants into this file
// as they will have different representations depending on the backend
use crate::curve::edwards::ExtendedPoint;
use crate::curve::twedwards::extended::ExtendedPoint as TwExtendedPoint;
use crate::field::u32::FieldElement28;

/// -4 * Twisted_D = -4 * (EDWARDS_D-1)
pub const NEG_FOUR_TIMES_TWISTED_D: FieldElement28 = FieldElement28([
    156327, 268435456, 268435455, 268435455, 268435455, 268435455, 268435455, 268435455, 268435454,
    268435455, 268435455, 268435455, 268435455, 268435455, 268435455, 268435455,
]);

// TODO: Differentiate between Twisted edwards d(a=-1) which is -39082 and edwards d(a=1) which is -39081

/// Edwards `d`, equals to -39081
pub const EDWARDS_D: FieldElement28 = FieldElement28([
    268396374, 268435455, 268435455, 268435455, 268435455, 268435455, 268435455, 268435455,
    268435454, 268435455, 268435455, 268435455, 268435455, 268435455, 268435455, 268435455,
]);

/// Neg_Edwards `-d`, equals to 39081
pub const NEG_EDWARDS_D: FieldElement28 =
    FieldElement28([39081, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]);

/// Twisted Edwards D equals `d-1`, equals to -39082
pub const TWISTED_D: FieldElement28 = FieldElement28([
    268396373, 268435455, 268435455, 268435455, 268435455, 268435455, 268435455, 268435455,
    268435454, 268435455, 268435455, 268435455, 268435455, 268435455, 268435455, 268435455,
]);

/// Twice the Twisted Edwards d which equals to -78164
pub const TWO_TIMES_TWISTED_D: FieldElement28 = FieldElement28([
    268357291, 268435455, 268435455, 268435455, 268435455, 268435455, 268435455, 268435455,
    268435454, 268435455, 268435455, 268435455, 268435455, 268435455, 268435455, 268435455,
]);

/// INVSQRT(a*d_2-1) where d_2 = 39082/39081
pub const DECAF_FACTOR: FieldElement28 = FieldElement28([
    0x05572736, 0x042ef0f4, 0x00ce5296, 0x07bf6aa2, 0x0ed26033, 0x0f4fd6ed, 0x0a839a66, 0x0968c14b,
    0x04a2d780, 0x0b8d54b6, 0x01a7b8a5, 0x06aa0a1f, 0x0d722fa2, 0x0683bf68, 0x0beb24f7, 0x022d962f,
]);
/// 39082 used in the doubling procedure in montgomery ladder
pub const A_PLUS_TWO_OVER_FOUR: FieldElement28 =
    FieldElement28([39082, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]);

// The basepoint of Ed448-Goldilocks
pub const GOLDILOCKS_BASE_POINT: ExtendedPoint = ExtendedPoint {
    X: FieldElement28([
        118276190, 40534716, 9670182, 135141552, 85017403, 259173222, 68333082, 171784774,
        174973732, 15824510, 73756743, 57518561, 94773951, 248652241, 107736333, 82941708,
    ]),
    Y: FieldElement28([
        36764180, 8885695, 130592152, 20104429, 163904957, 30304195, 121295871, 5901357, 125344798,
        171541512, 175338348, 209069246, 3626697, 38307682, 24032956, 110359655,
    ]),
    Z: FieldElement28([1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]),
    T: FieldElement28([
        45061619, 6694120, 103620075, 168286294, 228718479, 151739175, 150043102, 237197013,
        14095975, 138747174, 90839103, 152869968, 221073549, 114093113, 183378460, 209054552,
    ]),
};

// The basepoint of the Twisted Edwards curve which is 2-isogenous to Ed448-Goldilocks
pub const TWISTED_EDWARDS_BASE_POINT: TwExtendedPoint = TwExtendedPoint {
    X: FieldElement28([
        0, 268435456, 268435455, 268435455, 268435455, 268435455, 268435455, 134217727, 268435454,
        268435455, 268435455, 268435455, 268435455, 268435455, 268435455, 134217727,
    ]),
    Y: FieldElement28([
        266160740, 101161805, 74490312, 12706731, 149232027, 72184820, 68425752, 84169329,
        64300076, 80170041, 105082960, 37781586, 19953866, 222875756, 82854534, 139496929,
    ]),
    Z: FieldElement28([1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]),
    T: FieldElement28([
        202998021, 238846317, 66379923, 102789507, 54662147, 81652110, 85576069, 171023191,
        104342404, 127188629, 141403663, 236837931, 109226495, 84812757, 24364708, 114517662,
    ]),
};
