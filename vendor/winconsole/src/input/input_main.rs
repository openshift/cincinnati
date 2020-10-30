use super::*;

/**
 Flushes the console input buffer.

 # Examples
 ```
 # extern crate winconsole;
 # use winconsole::input;
 # fn main() {
 input::flush().unwrap();
 # }
 ```

 # Errors
 * [`InvalidHandleError`]: Returned if an invalid handle to the console input is retrieved or used.
 * [`IoError`]: Returned if an OS error occurs.

 [`InvalidHandleError`]: ../errors/enum.WinError.html#InvalidHandle.v
 [`IoError`]: ../errors/enum.WinError.html#Io.v
 */
pub fn flush() -> WinResult<()> {
	console::flush_input()
}
/**
 Returns the number of input events which are available in the console
 input buffer.

 # Examples
 ```
 # extern crate winconsole;
 # use winconsole::input;
 # fn main() {
 let num = input::get_num_input_events().unwrap();
 println!("input events available: {}", num);
 # }
 ```

 # Errors
 * [`InvalidHandleError`]: Returned if an invalid handle to the console input is retrieved or used.
 * [`IoError`]: Returned if an OS error occurs.

 [`InvalidHandleError`]: ../errors/enum.WinError.html#InvalidHandle.v
 [`IoError`]: ../errors/enum.WinError.html#Io.v
 */
pub fn get_num_input_events() -> WinResult<u32> {
	console::num_input_events()
}
/**
 Returns the number of mouse buttons available for the console.

 # Examples
 ```
 # extern crate winconsole;
 # use winconsole::input;
 # fn main() {
 let num = input::get_num_mouse_buttons().unwrap();
 println!("Mouse buttons available: {}", num);
 # }
 ```

 # Errors
 * [`IoError`]: Returned if an OS error occurs.

 [`IoError`]: ../errors/enum.WinError.html#Io.v
 */
pub fn get_num_mouse_buttons() -> WinResult<u32> {
	console::num_mouse_buttons()
}
/**
 Returns a list of keys that are currently in the pressed state.

 # Examples
 ```
 # extern crate winconsole;
 # use winconsole::input;
 # fn main() {
 let pressed = input::get_pressed_keys().unwrap();
 for key in pressed {
 	println!("{} is down", key);
 }
 # }
 ```

 # Errors
 * [`IoError`]: Returned if an OS error occurs.

 [`IoError`]: ../errors/enum.WinError.html#Io.v
 */
pub fn get_pressed_keys() -> WinResult<Vec<KeyCode>> {
	let mut ret = Vec::new();
	let state = console::get_keyboard_state()?;
	for (i, n) in state.iter().enumerate() {
		let key = KeyCode::from(i as u8);
		if key == KeyCode::None || key == KeyCode::NoMapping { continue; }
		if n & 0x80 != 0 { ret.push(key); }
	}
	Ok(ret)
}
/**
 Returns a boolean representing whether or not the key is currently pressed.

 # Arguments
 * `key_code` - The KeyCode to retrieve the status of.

 # Examples
 ```
 # extern crate winconsole;
 # use winconsole::input;
 # use winconsole::input::KeyCode;
 # fn main() {
 let pressed = input::is_key_down(KeyCode::Return);
 println!("Is [ENTER] pressed? {}", pressed);
 # }
 ```
 */
pub fn is_key_down(key_code: KeyCode) -> bool {
	if key_code == KeyCode::None || key_code == KeyCode::NoMapping {
		return false;
	}
	console::get_key_state(key_code as u8 as u32)
}
/**
 Returns a boolean representing whether or not a key such as
 CapsLock or NumLock is toggled. This is insignificant for
 non-toggle keys.

 # Arguments
 * `key_code` - The KeyCode to retrieve the status of.

 # Examples
 ```
 # extern crate winconsole;
 # use winconsole::input;
 # use winconsole::input::KeyCode;
 # fn main() {
 let pressed = input::is_key_toggled(KeyCode::Capital);
 println!("Is CapsLock on? {}", pressed);
 # }
 ```
 */
pub fn is_key_toggled(key_code: KeyCode) -> bool {
	if key_code == KeyCode::None || key_code == KeyCode::NoMapping {
		return false;
	}
	console::get_key_toggle(key_code as u8 as u32)
}
/**
 Creates and returns an InputContext, and initialises input.

 # Examples
 ```
 # extern crate winconsole;
 # use winconsole::input;
 # fn main() {
 let mut ctx = input::start().unwrap();
 let event = ctx.wait().unwrap();
 println!("{}", event);
 # }
 ```

 # Errors
 * [`InvalidHandleError`]: Returned if an invalid handle to the console input is retrieved or used.
 * [`IoError`]: Returned if an OS error occurs.

 [`InvalidHandleError`]: ../errors/enum.WinError.html#InvalidHandle.v
 [`IoError`]: ../errors/enum.WinError.html#Io.v
 */
pub fn start() -> WinResult<InputContext> {
	let mut ctx = InputContext::new(console::get_input_mode()?);
	ctx.reset();
	console::flush_input()?;
	Ok(ctx)
}
/**
 Adds an input event directly to the input buffer.

 # Arguments
 * `event` - The InputEvent to add.
 * `button_status` - The current status of mouse buttons 1 through 5. If None,
 the current button states are used.

 # Examples
 ```
 # extern crate winconsole;
 # use winconsole::input;
 # use winconsole::input::FocusEvent;
 # fn main() {
 let mut event = FocusEvent::new();
 event.focused = true;
 input::write(event, None).unwrap();
 # }
 ```

 # Errors
 * [`InvalidHandleError`]: Returned if an invalid handle to the console input is retrieved or used.
 * [`IoError`]: Returned if an OS error occurs.

 [`InvalidHandleError`]: ../errors/enum.WinError.html#InvalidHandle.v
 [`IoError`]: ../errors/enum.WinError.html#Io.v
 */
pub fn write(event: impl Into<InputEvent>, button_status: impl Into<Option<[bool; 5]>>) -> WinResult<()> {
	let event = event.into();
	if event == InputEvent::None { return Ok(()); }

	let button_status = match button_status.into() {
		None => {
			let mut status = [false; 5];
			for i in 0..5 {
				status[i] = console::get_key_state(BUTTON_VIRTUAL[i] as u32);
			}
			status
		},
		Some(status) => status
	};

	console::write_input(vec![self::convert_to_record(event, button_status)])
}

pub(crate) fn convert_events(records: &Vec<INPUT_RECORD>, ctx: &mut InputContext) -> Vec<InputEvent> {
	let mut ret = Vec::new();
	let button_status = &mut ctx.button_status;
	let held_keys = &mut ctx.held_keys;
	let repeat_enabled = ctx.repeat_enabled;
	for record in records {
		let ev = record.Event;

		match record.EventType {
			MOUSE_EVENT => {
				let mer = unsafe { ev.MouseEvent() };
				let flags = mer.dwEventFlags;
				let position = Vector2::new(mer.dwMousePosition.X as u16, mer.dwMousePosition.Y as u16);
				let modifiers = ControlKeyState::from(mer.dwControlKeyState as u16);

				if flags == MOUSE_MOVED {
					let mut mmev = MouseMoveEvent::new();
					mmev.modifiers = modifiers;
					mmev.position = position;
					ret.push(InputEvent::MouseMove(mmev));
				} else if flags & (MOUSE_WHEELED | MOUSE_HWHEELED) != 0 {
					let mut mwev = MouseWheelEvent::new();
					mwev.delta = ((mer.dwButtonState as i32) / 65536) as i16;
					mwev.horizontal = flags & MOUSE_HWHEELED != 0;
					mwev.modifiers = modifiers;
					mwev.position = position;
					ret.push(InputEvent::MouseWheel(mwev));
				} else {
					for i in 0..5 {
						let status = mer.dwButtonState & (0x1 << i) != 0;
						if status == button_status[i] { continue; }

						let mut mev = MouseEvent::new();
						mev.button = (i as u8) + 1;
						mev.modifiers = modifiers;
						mev.position = position;
						mev.pressed = status;
						mev.key_code = KeyCode::from(BUTTON_VIRTUAL[i]);

						button_status[i] = status;
						if status {
							ret.push(InputEvent::MouseDown(mev));
						} else {
							ret.push(InputEvent::MouseUp(mev));
						}
					}
				}
			},
			KEY_EVENT => {
				let ker = unsafe { ev.KeyEvent() };
				let virt = ker.wVirtualKeyCode as u8;
				if BUTTON_VIRTUAL.contains(&virt) { continue; }

				let key_code = KeyCode::from(virt);
				let character = unsafe { *(ker.uChar.AsciiChar()) };
				let status = ker.bKeyDown != 0;
				let mut kev = KeyEvent::new();
				kev.character = character as u8 as char;
				kev.key_code = key_code;
				kev.modifiers = ControlKeyState::from(ker.dwControlKeyState as u16);
				kev.pressed = status;
				kev.repeat_count = ker.wRepeatCount;
				kev.scan_code = ker.wVirtualScanCode;

				if status {
					if held_keys.contains(&key_code) {
						if repeat_enabled {
							ret.push(InputEvent::KeyHeld(kev));
						}
					} else {
						ret.push(InputEvent::KeyDown(kev));
						if key_code != KeyCode::None && key_code != KeyCode::NoMapping {
							held_keys.push(key_code);
						}
					}
				} else {
					if held_keys.contains(&key_code) {
						let index = held_keys.iter().position(|k| *k == key_code).unwrap();
						held_keys.remove(index);
					}
					ret.push(InputEvent::KeyUp(kev));
				}
			},
			FOCUS_EVENT => {
				let fer = unsafe { ev.FocusEvent() };
				let focused = fer.bSetFocus != 0;
				let mut fev = FocusEvent::new();
				fev.focused = focused;

				if focused {
					ret.push(InputEvent::Focused(fev));
				} else {
					ret.push(InputEvent::FocusLost(fev));
				}
			},
			WINDOW_BUFFER_SIZE_EVENT => {
				let wer = unsafe { ev.WindowBufferSizeEvent() };
				let mut rev = ResizeEvent::new();
				rev.size = Vector2::new(wer.dwSize.X as u16, wer.dwSize.Y as u16);
				ret.push(InputEvent::Resize(rev));
			},
			_ => ()
		}
	}
	ret
}

fn convert_to_record(event: InputEvent, button_status: [bool; 5]) -> INPUT_RECORD {
	let mut record: INPUT_RECORD = unsafe { mem::zeroed() };
	let mut ev = record.Event;
	match event {
		InputEvent::Focused(fev) | InputEvent::FocusLost(fev) => {
			record.EventType = FOCUS_EVENT;
			unsafe {
				*ev.FocusEvent_mut() = FOCUS_EVENT_RECORD {
					bSetFocus: bool_to_num!(fev.focused)
				};
			}
		},
		InputEvent::KeyHeld(kev) | InputEvent::KeyDown(kev) | InputEvent::KeyUp(kev) => {
			record.EventType = KEY_EVENT;
			let key_code: u8 = kev.key_code.into();
			let control_key_state: u16 = kev.modifiers.into();
			unsafe {
				let mut u_char: KEY_EVENT_RECORD_uChar = mem::zeroed();
				*u_char.AsciiChar_mut() = kev.character as i8;

				*ev.KeyEvent_mut() = KEY_EVENT_RECORD {
					bKeyDown: bool_to_num!(kev.pressed),
					wRepeatCount: kev.repeat_count,
					wVirtualKeyCode: key_code as u16,
					wVirtualScanCode: kev.scan_code,
					uChar: u_char,
					dwControlKeyState: control_key_state as u32
				};
			}
		},
		InputEvent::MouseDown(mev) | InputEvent::MouseUp(mev) => {
			record.EventType = MOUSE_EVENT;

			let control_key_state: u16 = mev.modifiers.into();
			let mut state = 0;
			for i in 0..5 {
				state |= bool_to_num!(button_status[i]) << i;
			}
			if mev.button > 0 {
				state |= bool_to_num!(mev.pressed) << (mev.button - 1);
			}

			unsafe {
				*ev.MouseEvent_mut() = MOUSE_EVENT_RECORD {
					dwMousePosition: COORD {
						X: mev.position.x as i16,
						Y: mev.position.y as i16
					},
					dwButtonState: state,
					dwControlKeyState: control_key_state as u32,
					dwEventFlags: 0
				};
			}
		},
		InputEvent::MouseMove(mmev) => {
			record.EventType = MOUSE_EVENT;

			let control_key_state: u16 = mmev.modifiers.into();
			unsafe {
				*ev.MouseEvent_mut() = MOUSE_EVENT_RECORD {
					dwMousePosition: COORD {
						X: mmev.position.x as i16,
						Y: mmev.position.y as i16
					},
					dwButtonState: 0,
					dwControlKeyState: control_key_state as u32,
					dwEventFlags: 1
				};
			}
		},
		InputEvent::MouseWheel(mwev) => {
			use std::num::Wrapping;
			record.EventType = MOUSE_EVENT;

			let control_key_state: u16 = mwev.modifiers.into();
			let flags = if mwev.horizontal {
				MOUSE_HWHEELED
			} else {
				MOUSE_WHEELED
			};
			let state = (Wrapping(mwev.delta as u32) * Wrapping(65536u32)).0;

			unsafe {
				*ev.MouseEvent_mut() = MOUSE_EVENT_RECORD {
					dwMousePosition: COORD {
						X: mwev.position.x as i16,
						Y: mwev.position.y as i16
					},
					dwButtonState: state,
					dwControlKeyState: control_key_state as u32,
					dwEventFlags: flags
				};
			}
		},
		InputEvent::Resize(rev) => {
			record.EventType = WINDOW_BUFFER_SIZE_EVENT;
			unsafe {
				*ev.WindowBufferSizeEvent_mut() = WINDOW_BUFFER_SIZE_RECORD {
					dwSize: COORD {
						X: rev.size.x as i16,
						Y: rev.size.y as i16
					}
				};
			}
		},
		InputEvent::None => unreachable!()
	};

	record.Event = ev;
	record
}
