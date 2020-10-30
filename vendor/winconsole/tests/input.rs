#![cfg(test)]
#![cfg(feature = "input")]
extern crate cgmath;
extern crate winconsole;

use cgmath::Vector2;
use winconsole::{
	console,
	input::{self, KeyCode, KeyEvent, MouseWheelEvent}
};

#[test]
fn drop_restore() {
	let old_mode = console::get_input_mode().unwrap();
	{
		let _ctx = input::start().unwrap();
		let mut mode = old_mode.clone();
		mode.MouseInput = !mode.MouseInput;
		mode.WindowInput = !mode.WindowInput;

		console::set_input_mode(mode).unwrap();
		assert_eq!(console::get_input_mode().unwrap(), mode);
	}
	assert_eq!(console::get_input_mode().unwrap(), old_mode);
}
#[test]
fn drop_no_restore() {
	let old_mode = console::get_input_mode().unwrap();
	let mut mode = old_mode.clone();
	{
		let mut ctx = input::start().unwrap();
		ctx.restore_on_drop = false;

		mode.MouseInput = !mode.MouseInput;
		mode.WindowInput = !mode.WindowInput;
		console::set_input_mode(mode).unwrap();
		assert_eq!(console::get_input_mode().unwrap(), mode);
	}
	assert_eq!(console::get_input_mode().unwrap(), mode);
	
	console::set_input_mode(old_mode).unwrap();
	assert_eq!(console::get_input_mode().unwrap(), old_mode);
}
#[test]
fn simulate() {
	let mut ctx = input::start().unwrap();

	let key_event = {
		let mut ev = KeyEvent::new();
		ev.key_code = KeyCode::A;
		ev.character = 'a';
		ev.pressed = true;
		ev
	};
	let wheel_event = {
		let mut ev = MouseWheelEvent::new();
		ev.delta = 120;
		ev.position = Vector2::new(10, 15);
		ev
	};

	input::flush().unwrap();
	ctx.flush();
	ctx.simulate(key_event);
	ctx.simulate(wheel_event);

	let events = ctx.get().unwrap();
	assert!(events.len() >= 2);
	assert_eq!(events[0], key_event.into());
	assert_eq!(events[1], wheel_event.into());
}
