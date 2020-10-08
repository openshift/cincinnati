#![cfg(test)]
#![cfg(feature = "window")]
#![cfg(feature = "serde")]

extern crate serde_cbor;
extern crate serde_json;
extern crate winconsole;

use winconsole::window;

macro_rules! serde_test {
	($obj:expr, $type:ty, $output:expr) => {
		let obj = $obj;

		let cbor = serde_cbor::to_vec(&obj).unwrap();
		let json = serde_json::to_string(&obj).unwrap();
		if $output {
			println!("{}", json);
			println!("{:?}", cbor);
		}

		let cbor_de: $type = serde_cbor::from_slice(&cbor).unwrap();
		let json_de: $type = serde_json::from_str(&json).unwrap();

		assert_eq!(obj, cbor_de);
		assert_eq!(obj, json_de);
	};
	($obj:expr, $type:ty) => (serde_test!($obj, $type, true));
}

#[test]
fn display_state() {
	use winconsole::window::DisplayState;
	serde_test!(window::get_display_state().unwrap(), DisplayState);
}
#[test]
fn flash_info() {
	use winconsole::window::FlashInfo;
	serde_test!(FlashInfo::new(), FlashInfo);
}
