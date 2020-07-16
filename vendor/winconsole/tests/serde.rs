#![cfg(test)]
#![cfg(feature = "serde")]

extern crate serde_cbor;
extern crate serde_json;
extern crate winconsole;

use winconsole::console;

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
fn code_page() {
	use winconsole::console::CodePage;
	serde_test!(console::get_input_code_page(), CodePage);
}
#[test]
fn code_page_info() {
	use winconsole::console::CodePageInfo;
	serde_test!(console::get_input_code_page().get_info().unwrap(), CodePageInfo);
}
#[test]
fn console_color() {
	use winconsole::console::ConsoleColor;
	serde_test!(ConsoleColor::Magenta, ConsoleColor);
}
#[test]
fn console_font() {
	use winconsole::console::ConsoleFont;
	serde_test!(console::get_font().unwrap(), ConsoleFont);
}
#[test]
fn console_state() {
	use winconsole::console::ConsoleState;
	serde_test!(console::get_state(true, false).unwrap(), ConsoleState, false);
}
#[test]
fn history_info() {
	use winconsole::console::HistoryInfo;
	serde_test!(console::get_history_info().unwrap(), HistoryInfo);
}
#[test]
fn input_settings() {
	use winconsole::console::InputSettings;
	serde_test!(console::get_input_mode().unwrap(), InputSettings);
}
#[test]
fn output_settings() {
	use winconsole::console::OutputSettings;
	serde_test!(console::get_output_mode().unwrap(), OutputSettings);
}
#[test]
fn rect() {
	use winconsole::console::Rect;
	serde_test!(Rect::new(1, 2, 3, 4), Rect);
}
#[test]
fn selection_info() {
	use winconsole::console::SelectionInfo;
	serde_test!(console::get_selection_info().unwrap(), SelectionInfo);
}
