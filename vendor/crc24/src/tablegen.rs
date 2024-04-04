use std::env;
use std::fs;
use std::io::prelude::*;
use std::path;

const CRC24_POLY: u32 = 0x86_4C_FB; // CRC-24 (IETF RFC2440), used by OpenPGP
const INC_FILE: &'static str = "table.inc";

fn main() {
    let ods = env::var("OUT_DIR").unwrap();
	let odp = path::Path::new(&ods);
	let f = &mut fs::File::create(&odp.join(INC_FILE)).unwrap();
	write!(f, "{}", into_code(table_gen())).unwrap();
}

fn table_gen() -> Vec<u32> {
	let mut v = Vec::new();
	for hi in 0..256u32 {
		let mut temp = hi << 16;
		for _ in 0..8 {
			let x = if (temp & 0x80_00_00) == 0 { 0 } else { CRC24_POLY };
			temp = ((temp & 0x7F_FF_FF) << 1) ^ x;
		}
		v.push(temp);
	}
	v
}

fn into_code(tab: Vec<u32>) -> String {
	let mut out: Vec<u8> = Vec::new();
	writeln!(&mut out, "const CRC24_TABLE: [u32; 256] = [").unwrap();
	for row in tab.chunks(4) {
		writeln!(&mut out, "\t0x{:06x}, 0x{:06x}, 0x{:06x}, 0x{:06x},",
			row[0], row[1], row[2], row[3]).unwrap();
	}
	writeln!(&mut out, "];").unwrap();
	String::from_utf8(out).unwrap()
}

