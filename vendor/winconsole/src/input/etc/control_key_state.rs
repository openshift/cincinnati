flags! {
	/**
	 Flags which represent the state of control keys.

	 # See
	 [KEY_EVENT_RECORD](https://docs.microsoft.com/en-us/windows/console/key-event-record-str).
	 */
	ControlKeyState<u16> {
		/// Is caps-lock enabled?
		CapsLockOn = 0x80,
		/// Is the key an enhanced key?
		EnhancedKey = 0x100,
		/// Is left alt in a pressed state?
		LeftAltPressed = 0x2,
		/// Is left ctrl in a pressed state?
		LeftCtrlPressed = 0x8,
		/// Is num-lock enabled?
		NumLockOn = 0x20,
		/// Is right alt in a pressed state?
		RightAltPressed = 0x1,
		/// Is right ctrl in a pressed state?
		RightCtrlPressed = 0x4,
		/// Is scroll-lock enabled?
		ScrollLockOn = 0x40,
		/// Is the shift key in a pressed state?
		ShiftPressed = 0x10,
	}
}
