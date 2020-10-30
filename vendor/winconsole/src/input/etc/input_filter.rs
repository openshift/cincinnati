flags! {
	/// Flags used to store the state of InputEvent filters.
	InputFilter<u16> {
		/// Should `InputEvent::None` be ignored?
		None = 0x1,
		/// Should `InputEvent::Focused` be ignored?
		Focused = 0x2,
		/// Should `InputEvent::FocusLost` be ignored?
		FocusLost = 0x4,
		/// Should `InputEvent::KeyHeld` be ignored?
		KeyHeld = 0x8,
		/// Should `InputEvent::KeyDown` be ignored?
		KeyDown = 0x10,
		/// Should `InputEvent::KeyUp` be ignored?
		KeyUp = 0x20,
		/// Should `InputEvent::MouseDown` be ignored?
		MouseDown = 0x40,
		/// Should `InputEvent::MouseMove` be ignored?
		MouseMove = 0x80,
		/// Should `InputEvent::MouseUp` be ignored?
		MouseUp = 0x100,
		/// Should `InputEvent::MouseWheel` be ignored?
		MouseWheel = 0x200,
		/// Should `InputEvent::Resize` be ignored?
		Resize = 0x400,
	}
}
