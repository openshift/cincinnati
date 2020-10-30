use super::console;
use super::console::*;
use super::errors::*;

use cgmath::Vector2;

use std::{fmt, fmt::{Display, Formatter}, mem};

use winapi::um::wincon::{
	COORD,
	FOCUS_EVENT,
	FOCUS_EVENT_RECORD,
	INPUT_RECORD,
	KEY_EVENT,
	KEY_EVENT_RECORD,
	KEY_EVENT_RECORD_uChar,
	MOUSE_EVENT,
	MOUSE_EVENT_RECORD,
	MOUSE_MOVED,
	MOUSE_WHEELED,
	MOUSE_HWHEELED,
	WINDOW_BUFFER_SIZE_EVENT,
	WINDOW_BUFFER_SIZE_RECORD,
};

mod input_main;
mod etc;

pub use self::input_main::*;
pub use self::etc::*;

const BUTTON_VIRTUAL: [u8; 5] = [1, 2, 4, 5, 6];
