flags! {
	/**
	 Settings related to console input.  

	 # See
	 [GetConsoleMode](https://docs.microsoft.com/en-us/windows/console/getconsolemode).
	 */
	InputSettings<u32> {
		/// Should characters read be echoed to the console output?
		EchoInput = 0x4,
		/// Should insert mode be enabled?
		InsertMode = 0x20,
		/**
		 Should read functions return only when a newline is read?
		 Disabling this requires EchoInput to be disabled as well.
		 */
		LineInput = 0x2,
		/// Should mouse events be placed into the input buffer?
		MouseInput = 0x10,
		/// Should certain input be processed by the system?
		ProcessedInput = 0x1,
		/// Should quick-edit mode be enabled?
		QuickEditMode = 0xC0,
		/// Should buffer resize events and focus events be placed into the input buffer?
		WindowInput = 0x8,
		/// Should user input be converted to virtual terminal sequences?
		VirtualTerminalInput = 0x200,
	}
}
