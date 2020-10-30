use cgmath::Vector2;
use super::*;

/**
 Brings the window to the foreground and activates it, optionally displaying it.

 # Arguments
 * `display` - Should the window be displayed?
 
 # Examples
 ```
 # extern crate winconsole;
 # use winconsole::window;
 # fn main() {
 window::activate(true).unwrap();
 # }
 ```

 # Errors
 * [`IoError`]: Returned if an OS error occurs.

 [`IoError`]: ../errors/enum.WinError.html#Io.v
 */
pub fn activate(display: bool) -> WinResult<()> {
	os_err!(unsafe { winuser::SetForegroundWindow(window_handle!()) });
	if display {
		show(true);
		if is_minimized()? {
			restore();
		}
	}

	Ok(())
}
/**
 Flashes the console window caption and/or tray icon.

 # Examples
 ```
 # extern crate winconsole;
 # use winconsole::window;
 # use winconsole::window::FlashInfo;
 # fn main() {
 let mut info = FlashInfo::new();
 info.count = 3;
 info.flash_caption = true;
 window::flash(info);
 # }
 ```
 */
pub fn flash(info: FlashInfo) {
	let mut flags: u32 = 0;
	let mut count = info.count;
	let rate = info.rate;

	if info.flash_caption { flags |= 0x1; }
	if info.flash_tray { flags |= 0x2; }
	if info.indefinite {
		flags |= 0x4;
		count = 0;
	}
	if info.until_foreground {
		flags |= 0xC;
		count = 0;
	}

	unsafe {
		let handle = window_handle!();
		let mut info: FLASHWINFO = mem::zeroed();
		info.cbSize = mem::size_of::<FLASHWINFO>() as u32;
		info.dwFlags = flags;
		info.dwTimeout = rate;
		info.hwnd = handle;
		info.uCount = count;

		let info_p = &mut info as *mut FLASHWINFO;
		winuser::FlashWindowEx(info_p);
	}
}
/**
 Returns the screen position of the cursor.

 # Examples
 ```
 # extern crate winconsole;
 # use winconsole::window;
 # fn main() {
 let position = window::get_cursor_position().unwrap();
 println!("Cursor position: {:?}", position);
 # }
 ```

 # Errors
 * [`IoError`]: Returned if an OS error occurs.

 [`IoError`]: ../errors/enum.WinError.html#Io.v
 */
pub fn get_cursor_position() -> WinResult<Vector2<i32>> {
	let mut point: POINT;

	os_err!(unsafe {
		point = mem::zeroed();
		let point_p = &mut point as *mut POINT;
		winuser::GetCursorPos(point_p)
	});

	Ok(Vector2::new(point.x, point.y))
}
/**
 Returns the display state of the console window.

 # Examples
 ```
 # extern crate winconsole;
 # use winconsole::window;
 # fn main() {
 println!("State: {:?}", window::get_display_state().unwrap());
 # }
 ```

 # Errors
 * [`IoError`]: Returned if an OS error occurs.

 [`IoError`]: ../errors/enum.WinError.html#Io.v
 */
pub fn get_display_state() -> WinResult<DisplayState> {
	if !is_visible() {
		Ok(DisplayState::Hidden)
	} else {
		Ok(DisplayState::from(get_window_show()? as u8))
	}
}
/**
 Returns the position of the console window in pixels.

 # Examples
 ```
 # extern crate winconsole;
 # use winconsole::window;
 # fn main() {
 let position = window::get_position().unwrap();
 println!("Window position: {:?}", position);
 # }
 ```

 # Errors
 * [`IoError`]: Returned if an OS error occurs.

 [`IoError`]: ../errors/enum.WinError.html#Io.v
 */
pub fn get_position() -> WinResult<Vector2<i32>> {
	let rect = get_window_rect()?;
	Ok(Vector2::new(rect.left, rect.top))
}
/**
 Returns the size of the console window in pixels.

 # Examples
 ```
 # extern crate winconsole;
 # use winconsole::window;
 # fn main() {
 let size = window::get_size().unwrap();
 println!("Window size: {:?}", size);
 # }
 ```

 # Errors
 * [`IoError`]: Returned if an OS error occurs.

 [`IoError`]: ../errors/enum.WinError.html#Io.v
 */
pub fn get_size() -> WinResult<Vector2<i32>> {
	let rect = get_window_rect()?;
	Ok(Vector2::new(rect.right - rect.left - 1, rect.bottom - rect.top - 1))
}
/**
 Hides the console window.

 # Examples
 ```
 # extern crate winconsole;
 # use winconsole::window;
 # fn main() {
 window::hide();
 # }
 ```
 */
pub fn hide() {
	set_window_show(0)
}
/**
 Is the window the active window?

 # Examples
 ```
 # extern crate winconsole;
 # use winconsole::window;
 # fn main() {
 println!("Active? {}", window::is_active());
 # }
 ```
 */
pub fn is_active() -> bool {
	unsafe { winuser::GetForegroundWindow() == window_handle!() }
}
/**
 Is the window maximized?

 # Examples
 ```
 # extern crate winconsole;
 # use winconsole::window;
 # fn main() {
 println!("Maximized? {}", window::is_maximized().unwrap());
 # }
 ```

 # Errors
 * [`IoError`]: Returned if an OS error occurs.

 [`IoError`]: ../errors/enum.WinError.html#Io.v
 */
pub fn is_maximized() -> WinResult<bool> {
	Ok(get_window_show()? == 3)
}
/**
 Is the window minimized?

 # Examples
 ```
 # extern crate winconsole;
 # use winconsole::window;
 # fn main() {
 println!("Minimized? {}", window::is_minimized().unwrap());
 # }
 ```

 # Errors
 * [`IoError`]: Returned if an OS error occurs.

 [`IoError`]: ../errors/enum.WinError.html#Io.v
 */
pub fn is_minimized() -> WinResult<bool> {
	Ok(get_window_show()? == 2)
}
/**
 Is the window visible?

 Note that this only refers to the state of the WS_VISIBLE style bit; the window may be obscured by other windows.

 # Examples
 ```
 # extern crate winconsole;
 # use winconsole::window;
 # fn main() {
 println!("Visible? {}", window::is_visible());
 # }
 ```
 */
pub fn is_visible() -> bool {
	unsafe {
		let handle = window_handle!();
		winuser::IsWindowVisible(handle) != 0
	}
}
/**
 Maximizes and activates the console window.

 # Examples
 ```
 # extern crate winconsole;
 # use winconsole::window;
 # fn main() {
 window::maximize();
 # }
 ```
 */
pub fn maximize() {
	set_window_show(3)
}
/**
 Minimizes the console window.

 # Arguments
 * `activate` - Should the window become the active window?
 * `activate_next` - If `activate` is false and this is true, the next top-level window in the Z order is activated.

 # Examples
 ```
 # extern crate winconsole;
 # use winconsole::window;
 # fn main() {
 window::minimize(false, true);
 # }
 ```
 */
pub fn minimize(activate: bool, activate_next: bool) {
	let cmd = if activate {
		2
	} else {
		if activate_next { 6 } else { 7 }
	};
	set_window_show(cmd)
}
/**
 Restores and activates the console window.

 # Examples
 ```
 # extern crate winconsole;
 # use winconsole::window;
 # fn main() {
 window::restore();
 # }
 ```
 */
pub fn restore() {
	set_window_show(9)
}
/**
 Sets the screen position of the cursor.

 # Examples
 ```
 # extern crate winconsole;
 # use winconsole::window;
 # fn main() {
 let position = window::get_cursor_position().unwrap();
 window::set_cursor_position(position.x + 1, position.y + 1).unwrap();
 # }
 ```

 # Errors
 * [`IoError`]: Returned if an OS error occurs.

 [`IoError`]: ../errors/enum.WinError.html#Io.v
 */
pub fn set_cursor_position(x: i32, y: i32) -> WinResult<()> {
	os_err!(unsafe { winuser::SetCursorPos(x, y) });
	Ok(())
}
/**
 Sets the display state of the console window.

 # Arguments
 * `state` - The display state to set.

 # Examples
 ```
 # extern crate winconsole;
 # use winconsole::window;
 # use winconsole::window::DisplayState;
 # fn main() {
 window::set_display_state(DisplayState::Minimized);
 # }
 ```
 */
pub fn set_display_state(state: DisplayState) {
	match state {
		DisplayState::Normal => {
			show(true);
			restore();
		},
		DisplayState::Hidden => hide(),
		DisplayState::Minimized => {
			show(false);
			minimize(false, true);
		},
		DisplayState::Maximized => {
			show(true);
			maximize();
		}
	}
}
/**
 Sets the position of the console window.

 # Examples
 ```
 # extern crate winconsole;
 # use winconsole::window;
 # fn main() {
 let position = window::get_position().unwrap();
 window::set_position(position.x + 1, position.y + 1).unwrap();
 # }
 ```

 # Errors
 * [`IoError`]: Returned if an OS error occurs.

 [`IoError`]: ../errors/enum.WinError.html#Io.v
 */
pub fn set_position(x: i32, y: i32) -> WinResult<()> {
	set_window_info(x, y, 0, 0, None, 1)
}
/**
 Sets the size of the console window.

 # Examples
 ```
 # extern crate winconsole;
 # use winconsole::window;
 # fn main() {
 let size = window::get_size().unwrap();
 window::set_size(size.x + 1, size.y + 1).unwrap();
 # }
 ```

 # Errors
 * [`IoError`]: Returned if an OS error occurs.

 [`IoError`]: ../errors/enum.WinError.html#Io.v
 */
pub fn set_size(width: i32, height: i32) -> WinResult<()> {
	set_window_info(0, 0, width + 1, height + 1, None, 2)
}
/**
 Shows the console window.

 # Arguments
 * `activate` - Should the window become the active window?

 # Examples
 ```
 # extern crate winconsole;
 # use winconsole::window;
 # fn main() {
 window::show(true);
 # }
 ```
 */
pub fn show(activate: bool) {
	let cmd = if activate { 5 } else { 8 };
	set_window_show(cmd)
}
/**
 Stops flashing the console window caption and/or tray icon.

 # Examples
 ```
 # extern crate winconsole;
 # use winconsole::window;
 # fn main() {
 window::stop_flash();
 # }
 ```
 */
pub fn stop_flash() {
	flash(FlashInfo::new())
}

fn get_window_rect() -> WinResult<RECT> {
	let mut rect: RECT;
	os_err!(unsafe {
		let handle = window_handle!();
		rect = mem::zeroed();
		let rect_p = &mut rect as *mut RECT;

		winuser::GetWindowRect(handle, rect_p)
	});
	Ok(rect)
}
fn get_window_show() -> WinResult<u32> {
	let state: u32;
	os_err!(unsafe {
		let handle = window_handle!();
		let mut placement: WINDOWPLACEMENT = mem::zeroed();
		placement.length = mem::size_of::<WINDOWPLACEMENT>() as u32;
		let placement_p = &mut placement as *mut WINDOWPLACEMENT;

		let result = winuser::GetWindowPlacement(handle, placement_p);
		state = placement.showCmd;
		result
	});
	Ok(state)
}
fn set_window_info(x: i32, y: i32, width: i32, height: i32, z: Option<HWND>, flags: u32) -> WinResult<()> {
	let z = match z {
		Some(z) => z,
		None => ptr::null_mut()
	};
	os_err!(unsafe {
		let handle = window_handle!();
		winuser::SetWindowPos(handle, z, x, y, width, height, flags)
	});
	Ok(())
}
fn set_window_show(value: i32) {
	unsafe {
		let handle = window_handle!();
		winuser::ShowWindow(handle, value);
	}
}
