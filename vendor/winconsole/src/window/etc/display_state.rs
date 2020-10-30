enumeration! {
	/// The display state of the console window.
	#[repr(u8)]
	DisplayState<u8> {
		__DEFAULT__ = 1,
		/// The window is hidden.
		Hidden = 0,
		/// The window is visible, and not maximized nor minimized.
		Normal = 1,
		/// The window is visible and minimized.
		Minimized = 2,
		/// The window is visible and maximized.
		Maximized = 3,
	}
}
