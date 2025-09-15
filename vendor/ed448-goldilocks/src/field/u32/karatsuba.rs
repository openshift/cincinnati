// -*- mode: rust; coding: utf-8; -*-
use super::FieldElement28;

// Code originally from Mike Hamburg.Referenced from otrv4/ed448
/// Karatsuba algorithm:
/// Given two field elements `a` and `b` in base `r` where `a` and `b` are length `n`
/// Divide each field element into halves `a_hi`, `a_lo`,`b_hi`,`b_lo` where `a_hi`denotes high bits of `a`
/// a = a_hi * r^n/2 + a_lo
/// b = b_hi * r^n/2 + b_lo
/// a*b = (a_hi * r^n/2 + a_lo)(b_hi * r^n/2 + b_lo) = (a_hi * b_hi * r^n) + (a_hi * b_lo + a_lo * b_hi) * r^n/2 + a_lo * b_lo
/// If we let r^n/2 = \phi = 2^224 ie we split the number in half, then we can also perform a reduction in the multiplication since:
/// \phi^2 - \phi - 1 mod p = 0 => \phi^2 = \phi + 1

const MASK: u32 = (1 << 28) - 1;

// XXX: This has been changed to use i128 to avoid using wrapping arithmetic; need to check bounds
pub(crate) fn mul(a: &FieldElement28, b: &FieldElement28) -> FieldElement28 {
    const MASK: i128 = (1 << 28) - 1;
    let mut c = FieldElement28::zero();

    let aa = [
        a[0].wrapping_add(a[8]),
        a[1].wrapping_add(a[1 + 8]),
        a[2].wrapping_add(a[2 + 8]),
        a[3].wrapping_add(a[3 + 8]),
        a[4].wrapping_add(a[4 + 8]),
        a[5].wrapping_add(a[5 + 8]),
        a[6].wrapping_add(a[6 + 8]),
        a[7].wrapping_add(a[7 + 8]),
    ];
    let bb = [
        b[0].wrapping_add(b[8]),
        b[1].wrapping_add(b[1 + 8]),
        b[2].wrapping_add(b[2 + 8]),
        b[3].wrapping_add(b[3 + 8]),
        b[4].wrapping_add(b[4 + 8]),
        b[5].wrapping_add(b[5 + 8]),
        b[6].wrapping_add(b[6 + 8]),
        b[7].wrapping_add(b[7 + 8]),
    ];

    let m = |a: u32, b: u32| -> i128 { (a as i128) * (b as i128) };

    let (mut z0, mut z1, mut z2) = (0i128, 0i128, 0i128);

    // j=0
    z2 = m(a[0], b[0]);
    z1 = m(aa[0], bb[0]) - z2;
    z0 += m(a[8], b[8]) + z2;

    z2 = m(aa[7], bb[1])
        + m(aa[6], bb[2])
        + m(aa[5], bb[3])
        + m(aa[4], bb[4])
        + m(aa[3], bb[5])
        + m(aa[2], bb[6])
        + m(aa[1], bb[7]);
    z1 += m(a[15], b[9])
        + m(a[14], b[10])
        + m(a[13], b[11])
        + m(a[12], b[12])
        + m(a[11], b[13])
        + m(a[10], b[14])
        + m(a[9], b[15])
        + z2;
    z0 -= m(a[7], b[1])
        + m(a[6], b[2])
        + m(a[5], b[3])
        + m(a[4], b[4])
        + m(a[3], b[5])
        + m(a[2], b[6])
        + m(a[1], b[7])
        - z2;

    c[0] = (z0 & MASK) as u32;
    c[8] = (z1 & MASK) as u32;

    z0 >>= 28;
    z1 >>= 28;

    //j=1
    z2 = m(a[1], b[0]) + m(a[0], b[1]);
    z1 += m(aa[1], bb[0]) + m(aa[0], bb[1]) - z2;
    z0 += m(a[9], b[8]) + m(a[8], b[9]) + z2;

    z2 = m(aa[7], bb[2])
        + m(aa[6], bb[3])
        + m(aa[5], bb[4])
        + m(aa[4], bb[5])
        + m(aa[3], bb[6])
        + m(aa[2], bb[7]);
    z1 += m(a[15], b[10])
        + m(a[14], b[11])
        + m(a[13], b[12])
        + m(a[12], b[13])
        + m(a[11], b[14])
        + m(a[10], b[15])
        + z2;
    z0 -= m(a[7], b[2])
        + m(a[6], b[3])
        + m(a[5], b[4])
        + m(a[4], b[5])
        + m(a[3], b[6])
        + m(a[2], b[7])
        - z2;

    c[1] = (z0 & MASK) as u32;
    c[9] = (z1 & MASK) as u32;

    z0 >>= 28;
    z1 >>= 28;

    //j = 2
    z2 = m(a[2], b[0]) + m(a[1], b[1]) + m(a[0], b[2]);
    z1 += m(aa[0], bb[2]) + m(aa[1], bb[1]) + m(aa[2], bb[0]) - z2;
    z0 += m(a[10], b[8]) + m(a[9], b[9]) + m(a[8], b[10]) + z2;

    z2 = m(aa[7], bb[3]) + m(aa[6], bb[4]) + m(aa[5], bb[5]) + m(aa[4], bb[6]) + m(aa[3], bb[7]);
    z1 += m(a[15], b[11])
        + m(a[14], b[12])
        + m(a[13], b[13])
        + m(a[12], b[14])
        + m(a[11], b[15])
        + z2;
    z0 -= m(a[7], b[3]) + m(a[6], b[4]) + m(a[5], b[5]) + m(a[4], b[6]) + m(a[3], b[7]) - z2;

    c[2] = (z0 & MASK) as u32;
    c[10] = (z1 & MASK) as u32;

    z0 >>= 28;
    z1 >>= 28;

    //j = 3
    z2 = m(a[3], b[0]) + m(a[2], b[1]) + m(a[1], b[2]) + m(a[0], b[3]);
    z1 += m(aa[3], bb[0]) + m(aa[2], bb[1]) + m(aa[1], bb[2]) + m(aa[0], bb[3]) - z2;
    z0 += m(a[11], b[8]) + m(a[10], b[9]) + m(a[9], b[10]) + m(a[8], b[11]) + z2;

    z2 = m(aa[7], bb[4]) + m(aa[6], bb[5]) + m(aa[5], bb[6]) + m(aa[4], bb[7]);
    z0 -= m(a[7], b[4]) + m(a[6], b[5]) + m(a[5], b[6]) + m(a[4], b[7]) - z2;
    z1 += m(a[15], b[12]) + m(a[14], b[13]) + m(a[13], b[14]) + m(a[12], b[15]) + z2;

    c[3] = (z0 & MASK) as u32;
    c[11] = (z1 & MASK) as u32;

    z0 >>= 28;
    z1 >>= 28;

    //j = 4
    z2 = m(a[4], b[0]) + m(a[3], b[1]) + m(a[2], b[2]) + m(a[1], b[3]) + m(a[0], b[4]);
    z1 += m(aa[4], bb[0]) + m(aa[3], bb[1]) + m(aa[2], bb[2]) + m(aa[1], bb[3]) + m(aa[0], bb[4])
        - z2;
    z0 += m(a[12], b[8]) + m(a[11], b[9]) + m(a[10], b[10]) + m(a[9], b[11]) + m(a[8], b[12]) + z2;

    z2 = m(aa[7], bb[5]) + m(aa[6], bb[6]) + m(aa[5], bb[7]);
    z1 += m(a[15], b[13]) + m(a[14], b[14]) + m(a[13], b[15]) + z2;
    z0 -= m(a[7], b[5]) + m(a[6], b[6]) + m(a[5], b[7]) - z2;

    c[4] = (z0 & MASK) as u32;
    c[12] = (z1 & MASK) as u32;

    z0 >>= 28;
    z1 >>= 28;

    //j = 5
    z2 = m(a[5], b[0])
        + m(a[4], b[1])
        + m(a[3], b[2])
        + m(a[2], b[3])
        + m(a[1], b[4])
        + m(a[0], b[5]);
    z1 += m(aa[5], bb[0])
        + m(aa[4], bb[1])
        + m(aa[3], bb[2])
        + m(aa[2], bb[3])
        + m(aa[1], bb[4])
        + m(aa[0], bb[5])
        - z2;
    z0 += m(a[13], b[8])
        + m(a[12], b[9])
        + m(a[11], b[10])
        + m(a[10], b[11])
        + m(a[9], b[12])
        + m(a[8], b[13])
        + z2;
    z2 = m(aa[7], bb[6]) + m(aa[6], bb[7]);
    z1 += m(a[15], b[14]) + m(a[14], b[15]) + z2;
    z0 -= m(a[7], b[6]) + m(a[6], b[7]) - z2;

    c[5] = (z0 & MASK) as u32;
    c[13] = (z1 & MASK) as u32;

    z0 >>= 28;
    z1 >>= 28;

    //j = 6

    z2 = m(a[6], b[0])
        + m(a[5], b[1])
        + m(a[4], b[2])
        + m(a[3], b[3])
        + m(a[2], b[4])
        + m(a[1], b[5])
        + m(a[0], b[6]);
    z1 += m(aa[6], bb[0])
        + m(aa[5], bb[1])
        + m(aa[4], bb[2])
        + m(aa[3], bb[3])
        + m(aa[2], bb[4])
        + m(aa[1], bb[5])
        + m(aa[0], bb[6])
        - z2;
    z0 += m(a[14], b[8])
        + m(a[13], b[9])
        + m(a[12], b[10])
        + m(a[11], b[11])
        + m(a[10], b[12])
        + m(a[9], b[13])
        + m(a[8], b[14])
        + z2;

    z2 = m(aa[7], bb[7]);
    z1 += m(a[15], b[15]) + z2;
    z0 -= m(a[7], b[7]) - z2;

    c[6] = (z0 & MASK) as u32;
    c[14] = (z1 & MASK) as u32;

    z0 >>= 28;
    z1 >>= 28;

    //j = 7
    z2 = m(a[7], b[0])
        + m(a[6], b[1])
        + m(a[5], b[2])
        + m(a[4], b[3])
        + m(a[3], b[4])
        + m(a[2], b[5])
        + m(a[1], b[6])
        + m(a[0], b[7]);
    z1 += m(aa[7], bb[0])
        + m(aa[6], bb[1])
        + m(aa[5], bb[2])
        + m(aa[4], bb[3])
        + m(aa[3], bb[4])
        + m(aa[2], bb[5])
        + m(aa[1], bb[6])
        + m(aa[0], bb[7])
        - z2;
    z0 += m(a[15], b[8])
        + m(a[14], b[9])
        + m(a[13], b[10])
        + m(a[12], b[11])
        + m(a[11], b[12])
        + m(a[10], b[13])
        + m(a[9], b[14])
        + m(a[8], b[15])
        + z2;

    z2 = 0;
    z1 += z2;
    z0 += z2;

    c[7] = (z0 & MASK) as u32;
    c[15] = (z1 & MASK) as u32;

    z0 >>= 28;
    z1 >>= 28;

    // Final step

    z0 += z1 + (c[8] as i128);
    z1 += c[0] as i128;

    c[8] = (z0 & MASK) as u32;
    c[0] = (z1 & MASK) as u32;

    z0 >>= 28;
    z1 >>= 28;

    c[9] += (z0 & MASK) as u32;
    c[1] += (z1 & MASK) as u32;

    c
}

// XXX: unroll and re-write
pub(crate) fn square(a: &FieldElement28) -> FieldElement28 {
    let (mut accum0, mut accum1, mut accum2) = (0u64, 0u64, 0u64);

    // result
    let mut c = FieldElement28::zero();

    let widemul = |a: u32, b: u32| -> u64 { (a as u64) * (b as u64) };

    let mut aa = [0u64; 8];
    for i in 0..8 {
        aa[i] = (a[i] as u64) + (a[i + 8] as u64);
    }

    // start j = 0
    accum2 = 0;
    accum2 = accum2.wrapping_add(widemul(a[0], a[0]));
    accum1 = accum1.wrapping_add(aa[0].wrapping_mul(aa[0]));
    accum0 = accum0.wrapping_add(widemul(a[8], a[8]));
    accum1 = accum1.wrapping_sub(accum2);
    accum0 = accum0.wrapping_add(accum2);
    accum2 = 0;
    for i in 1..4 {
        accum2 = accum2.wrapping_add(aa[i].wrapping_mul(aa[8 - i]) << 1);
        accum1 = accum1.wrapping_add(widemul(a[8 + i], a[16 - i]) << 1);
        accum0 = accum0.wrapping_sub(widemul(a[i], a[8 - i]) << 1);
    }
    accum2 = accum2.wrapping_add(aa[4].wrapping_mul(aa[4]));
    accum1 = accum1
        .wrapping_add(widemul(a[12], a[12]))
        .wrapping_add(accum2);
    accum0 = accum0
        .wrapping_sub(widemul(a[4], a[4]))
        .wrapping_add(accum2);
    c[0] = (accum0 as u32) & MASK;
    c[8] = (accum1 as u32) & MASK;
    accum0 >>= 28;
    accum1 >>= 28;
    // end j = 0

    // start j = 1..7
    for j in 1..7 {
        // first round
        accum2 = 0;
        for i in 0..((j / 2) + 1) {
            if i != j - i {
                accum2 = accum2.wrapping_add(widemul(a[j - i], a[i]) << 1);
                accum1 = accum1.wrapping_add(aa[j - i].wrapping_mul(aa[i]) << 1);
                accum0 = accum0.wrapping_add(widemul(a[8 + (j - i)], a[8 + i]) << 1);
            } else {
                accum2 = accum2.wrapping_add(widemul(a[j / 2], a[j / 2]));
                accum1 = accum1.wrapping_add(aa[j / 2].wrapping_mul(aa[j / 2]));
                accum0 = accum0.wrapping_add(widemul(a[8 + (j / 2)], a[8 + (j / 2)]));
            }
        }
        accum1 = accum1.wrapping_sub(accum2);
        accum0 = accum0.wrapping_add(accum2);

        // second round
        accum2 = 0;
        for i in 1..(((8 - j) / 2) + 1) {
            if j + i != 8 - i {
                accum2 = accum2.wrapping_add(aa[8 - i].wrapping_mul(aa[j + i]) << 1);
                accum1 = accum1.wrapping_add(widemul(a[16 - i], a[8 + j + i]) << 1);
                accum0 = accum0.wrapping_sub(widemul(a[8 - i], a[j + i]) << 1);
            } else {
                accum2 = accum2.wrapping_add(aa[j + i].wrapping_mul(aa[j + i]));
                accum1 = accum1.wrapping_add(widemul(a[8 + j + i], a[8 + j + i]));
                accum0 = accum0.wrapping_sub(widemul(a[j + i], a[j + i]));
            }
        }
        accum1 = accum1.wrapping_add(accum2);
        accum0 = accum0.wrapping_add(accum2);

        c[j] = (accum0 as u32) & MASK;
        c[j + 8] = (accum1 as u32) & MASK;
        accum0 >>= 28;
        accum1 >>= 28;
    }
    // end j = 1..7

    // start j = 7
    accum2 = 0;
    for i in 0..4 {
        accum2 = accum2.wrapping_add(widemul(a[7 - i], a[i]) << 1);
        accum1 = accum1.wrapping_add(aa[7 - i].wrapping_mul(aa[i]) << 1);
        accum0 = accum0.wrapping_add(widemul(a[15 - i], a[8 + i]) << 1);
    }
    accum1 = accum1.wrapping_sub(accum2);
    accum0 = accum0.wrapping_add(accum2);
    c[7] = (accum0 as u32) & MASK;
    c[15] = (accum1 as u32) & MASK;
    accum0 >>= 28;
    accum1 >>= 28;
    // end j = 7

    // final
    accum0 += accum1;
    accum0 += c[8] as u64;
    accum1 += c[0] as u64;
    c[8] = (accum0 as u32) & MASK;
    c[0] = (accum1 as u32) & MASK;
    accum0 >>= 28;
    accum1 >>= 28;
    c[9] += accum0 as u32;
    c[1] += accum1 as u32;

    c
}

#[test]
fn test_basic_mul() {
    let a = FieldElement28([5, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]);
    let b = FieldElement28([210, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]);
    let c = FieldElement28([1050, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]);

    let expected_c = mul(&a, &b);
    assert_eq!(expected_c.to_bytes()[..], c.to_bytes()[..]);
}

#[test]
fn test_mul() {
    let x = FieldElement28::from_bytes(&[
        0xf5, 0x81, 0x74, 0xd5, 0x7a, 0x33, 0x72, 0x36, 0x3c, 0x0d, 0x9f, 0xcf, 0xaa, 0x3d, 0xc1,
        0x8b, 0x1e, 0xff, 0x7e, 0x89, 0xbf, 0x76, 0x78, 0x63, 0x65, 0x80, 0xd1, 0x7d, 0xd8, 0x4a,
        0x87, 0x3b, 0x14, 0xb9, 0xc0, 0xe1, 0x68, 0x0b, 0xbd, 0xc8, 0x76, 0x47, 0xf3, 0xc3, 0x82,
        0x90, 0x2d, 0x2f, 0x58, 0xd2, 0x75, 0x4b, 0x39, 0xbc, 0xa8, 0x74,
    ]);

    let y = FieldElement28::from_bytes(&[
        0x74, 0xa8, 0xbc, 0x39, 0x4b, 0x75, 0xd2, 0x58, 0x2f, 0x2d, 0x90, 0x82, 0xc3, 0xf3, 0x47,
        0x76, 0xc8, 0xbd, 0x0b, 0x68, 0xe1, 0xc0, 0xb9, 0x14, 0x3b, 0x87, 0x4a, 0xd8, 0x7d, 0xd1,
        0x80, 0x65, 0x63, 0x78, 0x76, 0xbf, 0x89, 0x7e, 0xff, 0x1e, 0x8b, 0xc1, 0x3d, 0xaa, 0xcf,
        0x9f, 0x0d, 0x3c, 0x36, 0x72, 0x33, 0x7a, 0xd5, 0x74, 0x81, 0xf5,
    ]);

    let expected = FieldElement28::from_bytes(&[
        0x11, 0x95, 0x9c, 0x2e, 0x91, 0x78, 0x6f, 0xec, 0xff, 0x37, 0xe5, 0x8e, 0x2b, 0x50, 0x9e,
        0xf8, 0xfb, 0x41, 0x08, 0xc4, 0xa7, 0x02, 0x1c, 0xbf, 0x5a, 0x9f, 0x18, 0xa7, 0xec, 0x32,
        0x65, 0x7e, 0xed, 0xdc, 0x81, 0x81, 0x80, 0xa8, 0x4c, 0xdd, 0x95, 0x14, 0xe6, 0x67, 0x26,
        0xd3, 0xa1, 0x22, 0xdb, 0xb3, 0x9f, 0x17, 0x7a, 0x85, 0x16, 0x6c,
    ]);

    let result = mul(&x, &y);
    assert_eq!(result.to_bytes()[..], expected.to_bytes()[..])
}
#[test]
fn test_naive_square() {
    let a = FieldElement28([5, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]);
    let b = FieldElement28([5, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]);
    let c = FieldElement28([25, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]);

    let expected_c = mul(&a, &b);
    assert_eq!(expected_c.to_bytes()[..], c.to_bytes()[..]);
}
#[test]
fn test_karatsuba_square() {
    let a = FieldElement28([5, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]);
    let expected_c = FieldElement28([25, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]);
    let c = square(&a);
    assert_eq!(expected_c.to_bytes()[..], c.to_bytes()[..]);

    let d = FieldElement28([
        268396374, 268435455, 268435455, 268435455, 268435455, 268435455, 268435455, 268435455,
        268435454, 268435455, 268435455, 268435455, 268435455, 268435455, 268435455, 268435455,
    ]);
    let expected_d2 = mul(&d, &d);
    let d2 = square(&d);
    assert_eq!(expected_d2.to_bytes()[..], d2.to_bytes()[..]);
}
