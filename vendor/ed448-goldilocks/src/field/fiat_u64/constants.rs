// Move all curve specific constants into this file
// as they will have different representations depending on the backend
use crate::curve::edwards::ExtendedPoint;
use crate::curve::twedwards::extended::ExtendedPoint as TwExtendedPoint;
use crate::field::fiat_u64::FieldElement56;

/// -4 * Twisted_D = -4 * (EDWARDS_D-1)
pub const NEG_FOUR_TIMES_TWISTED_D: FieldElement56 = FieldElement56([
    156327,
    0,
    0,
    0,
    72057594037927935,
    72057594037927935,
    72057594037927935,
    72057594037927935,
]);

/// Edwards `d`, equals to -39081
pub const EDWARDS_D: FieldElement56 = FieldElement56([
    144115188075816789,
    144115188075855870,
    144115188075855870,
    144115188075855870,
    144115188075855868,
    144115188075855870,
    144115188075855870,
    144115188075855870,
]);

/// Neg_Edwards `-d`, equals to 39081
pub const NEG_EDWARDS_D: FieldElement56 = FieldElement56([39081, 0, 0, 0, 0, 0, 0, 0]);

/// Twisted Edwards D equals `d-1`, equals to -39082
pub const TWISTED_D: FieldElement56 = FieldElement56([
    144115188075816788,
    144115188075855870,
    144115188075855870,
    144115188075855870,
    144115188075855868,
    144115188075855870,
    144115188075855870,
    144115188075855870,
]);

/// Twice the Twisted Edwards d which equals to -78164
pub const TWO_TIMES_TWISTED_D: FieldElement56 = FieldElement56([
    144115188075777706,
    144115188075855870,
    144115188075855870,
    144115188075855870,
    144115188075855868,
    144115188075855870,
    144115188075855870,
    144115188075855870,
]);

/// INVSQRT(a*d_2-1) where d_2 = 39082/39081
pub const DECAF_FACTOR: FieldElement56 = FieldElement56([
    0x42ef0f45572736,
    0x7bf6aa20ce5296,
    0xf4fd6eded26033,
    0x968c14ba839a66,
    0xb8d54b64a2d780,
    0x6aa0a1f1a7b8a5,
    0x683bf68d722fa2,
    0x22d962fbeb24f7,
]);

/// 39082 used in the doubling procedure in montgomery ladder
pub const A_PLUS_TWO_OVER_FOUR: FieldElement56 = FieldElement56([39082, 0, 0, 0, 0, 0, 0, 0]);

/// The basepoint of Ed448-Goldilocks
pub const GOLDILOCKS_BASE_POINT: ExtendedPoint = ExtendedPoint {
    X: FieldElement56([
        10880955091566686,
        36276784145337894,
        69571282115576635,
        46113124210880026,
        4247859732800292,
        15440021224255559,
        66747077793030847,
        22264495316135181,
    ]),
    Y: FieldElement56([
        2385235625966100,
        5396741696826776,
        8134720567442877,
        1584133578609663,
        46047824121994270,
        56121598560924524,
        10283140089599689,
        29624444337960636,
    ]),
    Z: FieldElement56([1, 0, 0, 0, 0, 0, 0, 0]),
    T: FieldElement56([
        1796939199780339,
        45174008172060139,
        40732174862907279,
        63672088496536030,
        37244660935497319,
        41035719659624511,
        30626637035688077,
        56117654178374172,
    ]),
};

/// The basepoint of the Twisted Edwards curve which is 2-isogenous to Ed448-Goldilocks
pub const TWISTED_EDWARDS_BASE_POINT: TwExtendedPoint = TwExtendedPoint {
    X: FieldElement56([
        0,
        72057594037927936,
        72057594037927935,
        36028797018963967,
        72057594037927934,
        72057594037927935,
        72057594037927935,
        36028797018963967,
    ]),
    Y: FieldElement56([
        27155415521118820,
        3410937204744648,
        19376965222209947,
        22594032279754776,
        21520481577673772,
        10141917371396176,
        59827755213158602,
        37445921829569158,
    ]),
    Z: FieldElement56([1, 0, 0, 0, 0, 0, 0, 0]),
    T: FieldElement56([
        64114820220813573,
        27592348249940115,
        21918321435874307,
        45908688348236165,
        34141937727972228,
        63575698147485199,
        22766751209138687,
        30740600843388580,
    ]),
};
