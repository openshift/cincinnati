use cgmath::Vector2;
use rgb::RGB8;

use std::{io, mem, ptr};

use winapi::ctypes::c_void as VOID;
use winapi::shared::minwindef::{DWORD, MAX_PATH, UINT, WORD};
use winapi::um::{consoleapi, processenv, utilapiset, wincon, winnls};
use winapi::um::winbase::{
	STD_OUTPUT_HANDLE as STDOUT,
	STD_INPUT_HANDLE as STDIN
};
use winapi::um::wincon::{
	CHAR_INFO,
	CHAR_INFO_Char,
	COORD,
	CONSOLE_CURSOR_INFO,
	CONSOLE_HISTORY_INFO,
	CONSOLE_FONT_INFOEX,
	CONSOLE_READCONSOLE_CONTROL,
	CONSOLE_SCREEN_BUFFER_INFO,
	CONSOLE_SCREEN_BUFFER_INFOEX,
	CONSOLE_SELECTION_INFO,
	SMALL_RECT
};
use winapi::um::winnls::CPINFOEXA;
use winapi::um::winnt::{CHAR, WCHAR};

use super::errors::*;

type HandlerRoutine = unsafe extern "system" fn(_: u32) -> i32;

mod console_main;
mod etc;
#[cfg(feature = "input")]
mod console_input;

pub use self::console_main::*;
pub use self::etc::*;
#[cfg(feature = "input")]
pub(crate) use self::console_input::*;
