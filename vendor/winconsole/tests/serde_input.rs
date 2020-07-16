#![cfg(test)]
#![cfg(feature = "input")]
#![cfg(feature = "serde")]

extern crate serde_cbor;
extern crate serde_json;
extern crate winconsole;

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
fn control_key_state() {
	use winconsole::input::ControlKeyState;
	serde_test!(ControlKeyState::from(5), ControlKeyState);
}
#[test]
fn focus_event() {
	use winconsole::input::FocusEvent;
	serde_test!(FocusEvent::new(), FocusEvent);
}
#[test]
fn input_event() {
	use winconsole::input::{FocusEvent, InputEvent};
	serde_test!(InputEvent::FocusLost(FocusEvent::new()), InputEvent);
}
#[test]
fn input_filter() {
	use winconsole::input::InputFilter;
	serde_test!(InputFilter::from(9), InputFilter);
}
#[test]
fn key_code() {
	use winconsole::input::KeyCode;
	serde_test!(KeyCode::Return, KeyCode);
}
#[test]
fn key_event() {
	use winconsole::input::KeyEvent;
	serde_test!(KeyEvent::new(), KeyEvent);
}
#[test]
fn mouse_event() {
	use winconsole::input::MouseEvent;
	serde_test!(MouseEvent::new(), MouseEvent);
}
#[test]
fn mouse_move_event() {
	use winconsole::input::MouseMoveEvent;
	serde_test!(MouseMoveEvent::new(), MouseMoveEvent);
}
#[test]
fn mouse_wheel_event() {
	use winconsole::input::MouseWheelEvent;
	serde_test!(MouseWheelEvent::new(), MouseWheelEvent);
}
#[test]
fn resize_event() {
	use winconsole::input::ResizeEvent;
	serde_test!(ResizeEvent::new(), ResizeEvent);
}
