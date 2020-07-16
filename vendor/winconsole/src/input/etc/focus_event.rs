use super::InputEvent;

/// Represents an input event which occurred as a result of window focus changing.
#[derive(Clone, Copy, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct FocusEvent {
	/// Is the window focused?
	pub focused: bool
}

impl FocusEvent {
	/**
	 Returns an empty FocusEvent.
	 */
	pub fn new() -> FocusEvent {
		FocusEvent {
			focused: false
		}
	}
}

impl Into<InputEvent> for FocusEvent {
	fn into(self) -> InputEvent {
		if self.focused {
			InputEvent::Focused(self)
		} else {
			InputEvent::FocusLost(self)
		}
	}
}
