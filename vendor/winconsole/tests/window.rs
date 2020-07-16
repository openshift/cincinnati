#![cfg(test)]
#![cfg(feature = "window")]
extern crate cgmath;
extern crate winconsole;

use cgmath::Vector2;
use winconsole::window;

#[test]
fn cursor() {
	let old_position = window::get_cursor_position().unwrap();
	window::set_cursor_position(0, 0).unwrap();
	assert_eq!(window::get_cursor_position().unwrap(), Vector2::new(0, 0));

	window::set_cursor_position(old_position.x, old_position.y).unwrap();
}
#[test]
fn maximize() {
	let state = window::get_display_state().unwrap();
	window::maximize();
	assert!(window::is_maximized().unwrap());
	window::set_display_state(state);
}
#[test]
fn minimize() {
	let state = window::get_display_state().unwrap();
	window::minimize(false, true);
	assert!(window::is_minimized().unwrap());
	window::set_display_state(state);
}
