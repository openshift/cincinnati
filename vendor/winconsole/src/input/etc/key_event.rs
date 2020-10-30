use super::*;

/**
 Represents an input event which occurred on the keyboard.
 */
#[derive(Clone, Copy, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct KeyEvent {
	/// The character associated with the pressed key.
	pub character: char,
	/// The KeyCode of the key which the event occurred on.
	pub key_code: KeyCode,
	/// A ControlKeyState object describing the state of control keys.
	pub modifiers: ControlKeyState,
	/// Is the key pressed?
	pub pressed: bool,
	/// The amount of times the event was repeated in the input buffer.
	pub repeat_count: u16,
	/// The scan code of the key.
	pub scan_code: u16
}

impl KeyEvent {
	/**
	 Returns an empty KeyEvent.
	 */
	pub fn new() -> KeyEvent {
		KeyEvent {
			character: '\0',
			modifiers: ControlKeyState::new(),
			key_code: KeyCode::None,
			pressed: false,
			repeat_count: 0,
			scan_code: 0
		}
	}
}

impl Into<InputEvent> for KeyEvent {
	fn into(self) -> InputEvent {
		if self.pressed {
			InputEvent::KeyDown(self)
		} else {
			InputEvent::KeyUp(self)
		}
	}
}
