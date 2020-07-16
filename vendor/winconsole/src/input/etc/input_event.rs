use super::*;

/// An input event.
#[derive(Clone, Copy, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum InputEvent {
	/// An empty input event.
	None,
	/// A window focus event.
	Focused(FocusEvent),
	/// A window focus lost event.
	FocusLost(FocusEvent),
	/// A key hold event.
	KeyHeld(KeyEvent),
	/// A key press event.
	KeyDown(KeyEvent),
	/// A key release event.
	KeyUp(KeyEvent),
	/// A mouse press event.
	MouseDown(MouseEvent),
	/// A mouse move event.
	MouseMove(MouseMoveEvent),
	/// A mouse release event.
	MouseUp(MouseEvent),
	/// A mouse wheel event.
	MouseWheel(MouseWheelEvent),
	/// A buffer resize event.
	Resize(ResizeEvent)
}

impl InputEvent {
	pub(crate) fn get_type(&self) -> u16 {
		match *self {
			InputEvent::None => 0x1,
			InputEvent::Focused(_) => 0x2,
			InputEvent::FocusLost(_) => 0x4,
			InputEvent::KeyHeld(_) => 0x8,
			InputEvent::KeyDown(_) => 0x10,
			InputEvent::KeyUp(_) => 0x20,
			InputEvent::MouseDown(_) => 0x40,
			InputEvent::MouseMove(_) => 0x80,
			InputEvent::MouseUp(_) => 0x100,
			InputEvent::MouseWheel(_) => 0x200,
			InputEvent::Resize(_) => 0x400
		}
	}
}

impl Display for InputEvent {
	fn fmt(&self, f: &mut Formatter) -> fmt::Result {
		let res = match *self {
			InputEvent::None => String::from("InputEvent::None"),
			InputEvent::Focused(_) => String::from("InputEvent::Focused"),
			InputEvent::FocusLost(_) => String::from("InputEvent::FocusLost"),
			InputEvent::KeyHeld(kev) => {
				format!("InputEvent::KeyHeld({})", kev.key_code)
			},
			InputEvent::KeyDown(kev) => {
				format!("InputEvent::KeyDown({})", kev.key_code)
			},
			InputEvent::KeyUp(kev) => {
				format!("InputEvent::KeyUp({})", kev.key_code)
			},
			InputEvent::MouseDown(mev) => {
				format!("InputEvent::MouseDown({})", mev.key_code)
			},
			InputEvent::MouseUp(mev) => {
				format!("InputEvent::MouseUp({})", mev.key_code)
			},
			InputEvent::MouseMove(mev) => {
				format!("InputEvent::MouseMove({}, {})", mev.position.x, mev.position.y)
			},
			InputEvent::MouseWheel(mev) => {
				format!("InputEvent::MouseWheel({})", mev.delta)
			},
			InputEvent::Resize(rev) => {
				format!("InputEvent::Resize({}, {})", rev.size.x, rev.size.y)
			},
		};
		write!(f, "{}", &res)
	}
}
