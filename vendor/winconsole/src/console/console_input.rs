use super::*;
use winapi::um::{wincon::INPUT_RECORD, winuser};

pub(crate) fn get_key_state(key: u32) -> bool {
	let num = unsafe { winuser::GetAsyncKeyState(key as i32) as u16 };
	num & 0x8000 != 0
}
pub(crate) fn get_key_toggle(key: u32) -> bool {
	let num = unsafe { winuser::GetKeyState(key as i32) as u16 };
	num & 0x1 != 0
}
pub(crate) fn get_keyboard_state() -> WinResult<[u8; 256]> {
	let mut arr = [0; 256];
	os_err!(unsafe {
		let arr_p = &mut arr[0] as *mut u8;
		winuser::GetKeyState(0);
		winuser::GetKeyboardState(arr_p)
	});
	Ok(arr)
}
pub(crate) fn num_input_events() -> WinResult<u32> {
	let mut num: DWORD = 0;
	os_err!(unsafe {
		let handle = handle!(STDIN);
		let num_p = &mut num as *mut DWORD;
		consoleapi::GetNumberOfConsoleInputEvents(handle, num_p)
	});
	Ok(num)
}
pub(crate) fn num_mouse_buttons() -> WinResult<u32> {
	let mut num: DWORD = 0;
	os_err!(unsafe {
		let num_p = &mut num as *mut DWORD;
		wincon::GetNumberOfConsoleMouseButtons(num_p)
	});
	Ok(num)
}
pub(crate) fn peek_input(length: usize) -> WinResult<Vec<INPUT_RECORD>> {
	self::read_or_peek(length, true)
}
pub(crate) fn read_input(length: usize) -> WinResult<Vec<INPUT_RECORD>> {
	self::read_or_peek(length, false)
}
pub(crate) fn write_input(buffer: Vec<INPUT_RECORD>) -> WinResult<()> {
	os_err!(unsafe {
		let handle = handle!(STDIN);
		let length = buffer.len() as DWORD;
		if length == 0 {
			return Ok(());
		}
		let buffer = buffer.into_boxed_slice();

		let written_p = &mut 0u32 as *mut DWORD;
		let buffer_p = &buffer[0] as *const INPUT_RECORD;
		wincon::WriteConsoleInputA(handle, buffer_p, length, written_p)
	});
	Ok(())
}

fn read_or_peek(length: usize, peek: bool) -> WinResult<Vec<INPUT_RECORD>> {
	if length == 0 {
		return Ok(Vec::new());
	}

	let mut num: DWORD = 0;
	let mut buffer: Box<[INPUT_RECORD]>;
	os_err!(unsafe {
		let handle = handle!(STDIN);
		buffer = {
			let vec = vec![mem::zeroed(); length];
			vec.into_boxed_slice()
		};

		let length = length as DWORD;
		let buffer_p = &mut buffer[0] as *mut INPUT_RECORD;
		let num_p = &mut num as *mut DWORD;

		if peek {
			consoleapi::PeekConsoleInputA(handle, buffer_p, length, num_p)
		} else {
			consoleapi::ReadConsoleInputA(handle, buffer_p, length, num_p)
		}
	});
	Ok(buf_to_vec!(buffer, num))
}
