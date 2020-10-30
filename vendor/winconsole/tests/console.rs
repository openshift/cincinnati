#![cfg(test)]
extern crate cgmath;
extern crate winconsole;

use cgmath::Vector2;
use winconsole::console::{self, ConsoleColor};

#[test]
fn beep() {
    console::beep(440, 1000);
}

#[test]
fn color_mapping() {
	let old_black = console::get_color(ConsoleColor::Black).unwrap();
	let mut black = old_black.clone();
	black.r = 50;
	black.g = 210;
	black.b = 15;

	console::map_color(ConsoleColor::Black, black).unwrap();
	assert_eq!(console::get_color(ConsoleColor::Black).unwrap(), black);

	console::map_color(ConsoleColor::Black, old_black).unwrap();
}
#[test]
fn cursor_position() {
    let position = console::get_cursor_position().unwrap();
    console::set_cursor_position(10, 10).unwrap();
    assert_eq!(console::get_cursor_position().unwrap(), Vector2::new(10, 10));
    console::set_cursor_position(position.x, position.y).unwrap();
}
#[test]
fn cursor_size() {
    let old_size = console::get_cursor_size().unwrap();
    console::set_cursor_size(35).unwrap();
    assert_eq!(console::get_cursor_size().unwrap(), 35);
    console::set_cursor_size(old_size).unwrap();
}
#[test] #[should_panic]
fn cursor_size_fail() {
    console::set_cursor_size(101).unwrap();
}
#[test]
fn cursor_visible() {
    let visible = console::is_cursor_visible().unwrap();
    console::set_cursor_visible(false).unwrap();
    assert_eq!(console::is_cursor_visible().unwrap(), false);

    console::set_cursor_visible(visible).unwrap();
}

#[test]
fn background_color() {
    let old_color = console::get_background_color().unwrap();
    console::set_background_color(ConsoleColor::DarkRed).unwrap();
    println!("A message with a dark red background.");

    let background_color = console::get_background_color().unwrap();
    assert_eq!(background_color, ConsoleColor::DarkRed);
    console::set_background_color(old_color).unwrap();
}
#[test]
fn foreground_color() {
    let old_color = console::get_foreground_color().unwrap();
    console::set_foreground_color(ConsoleColor::DarkBlue).unwrap();
    println!("A dark blue message.");

    let foreground_color = console::get_foreground_color().unwrap();
    assert_eq!(foreground_color, ConsoleColor::DarkBlue);
    console::set_foreground_color(old_color).unwrap();
}

#[test]
fn input_mode() {
    let input_mode_orig = console::get_input_mode().unwrap();
    let mut input_mode = input_mode_orig.clone();

    input_mode.WindowInput = !input_mode.WindowInput;
    console::set_input_mode(input_mode).unwrap();
    assert_eq!(console::get_input_mode().unwrap(), input_mode);

    console::set_input_mode(input_mode_orig).unwrap();
}
#[test] #[should_panic]
fn input_mode_fail() {
	let mut input_mode = console::get_input_mode().unwrap();
	input_mode.LineInput = false;
	input_mode.EchoInput = true;
    console::set_input_mode(input_mode).unwrap();
}

#[test]
fn title() {
    let original_title = console::get_original_title().unwrap();

    console::set_title("Some New Console Title").unwrap();
    assert_eq!(console::get_title().unwrap(), "Some New Console Title");

    console::set_title(&original_title).unwrap();
    assert_eq!(console::get_title().unwrap(), original_title)
}
#[test]
fn title_empty() {
    let original_title = console::get_original_title().unwrap();

    console::set_title("").unwrap();
    assert_eq!(console::get_title().unwrap(), "");

    console::set_title(&original_title).unwrap();
    assert_eq!(console::get_title().unwrap(), original_title)
}

#[test]
fn window_size() {
	let max_size = console::get_largest_window_size().unwrap();
	let size = console::get_window_size().unwrap();
	if size.x + 1 < max_size.x && size.y + 1 < max_size.y {
		console::set_window_size(size.x + 1, size.y + 1).unwrap();
		assert_eq!(console::get_window_size().unwrap(), Vector2::new(size.x + 1, size.y + 1));
		console::set_window_size(size.x, size.y).unwrap();
	}
	assert_eq!(console::get_window_size().unwrap(), Vector2::new(size.x, size.y));
}
