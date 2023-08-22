//! Crate providing a CRC-24 hasher based on the IETF RFC2440 specification.

use std::default::Default;
use std::hash::Hasher;

const INIT: u32 = 0xB7_04_CE;

include! { concat!(env!("OUT_DIR"), "/table.inc") }

/// CRC-24 hasher based on IETF RFC2440 specification.
#[derive(Copy,Clone,PartialEq,Eq)]
pub struct Crc24Hasher {
	state: u32
}

impl Crc24Hasher {
	/// Creates a new CRC-24 hasher initialized with the given state.
	pub fn init(v: u32) -> Crc24Hasher {
		Crc24Hasher { state: v & 0xFF_FF_FF }
	}
	/// Creates a new CRC-24 hasher initialized with a nonzero state
	/// specified in RFC2440.
	pub fn new() -> Crc24Hasher {
		Crc24Hasher { state: INIT }
	}
}

impl Default for Crc24Hasher {
	/// Creates a new CRC-24 hasher initialized with a nonzero state
	/// specified in RFC2440.
	fn default() -> Crc24Hasher {
		Crc24Hasher::new()
	}
}

impl Hasher for Crc24Hasher {
	fn finish(&self) -> u64 { self.state as u64 }

	fn write(&mut self, msg: &[u8]) {
		let mut s = self.state;
		for &octet in msg.iter() {
			let index = ((octet as u32) ^ (s >> 16)) & 0xFF;
			s = (s << 8) ^ CRC24_TABLE[index as usize];
		}
		self.state = s & 0xFF_FF_FF;
	}
}

/// Computes hash of the raw bytes using CRC-24
/// (without including the length as part of the data)
pub fn hash_raw(octets: &[u8]) -> u32 {
	let mut h: Crc24Hasher = Default::default();
	h.write(octets);
	h.finish() as u32
}

#[cfg(test)]
mod test {

const CRC24_INIT: u32 = 0x__b7_04_ce;
const CRC24_POLY: u32 = 0x1_86_4c_fb; // including x^24

// directly translated from RFC2440 section 6.1.
fn crc_octets(octets: &[u8]) -> u32 {
	let mut crc = CRC24_INIT;
	for &octet in octets.iter() {
		crc ^= (octet as u32) << 16;
		for _ in 0..8 {
			crc <<= 1;
			if (crc & 0x1_00_00_00) != 0 {
				crc ^= CRC24_POLY;
			}
		}
	}
	crc & 0xFF_FF_FF
}

fn test_compare_impls(octets: &[u8]) -> bool {
	let h1 = crc_octets(octets);
	let h2 = super::hash_raw(octets);
	h1 == h2
}

#[test]
fn test() {
	assert!(test_compare_impls(b""));
	assert!(test_compare_impls(b"x"));
	assert!(test_compare_impls(b"sg"));
	assert!(test_compare_impls(b"crc"));
	assert!(test_compare_impls(b"test"));
}

} // mod test

