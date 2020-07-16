flags! {
	/**
	 Settings related to console output.  

	 # See
	 [GetConsoleMode](https://docs.microsoft.com/en-us/windows/console/getconsolemode).
	 */
	OutputSettings<u32> {
		/// Should output be processed for escape sequences?
		ProcessedOutput = 0x1,
		/// Should the console cursor wrap at the end of the line?
		WrapAtEol = 0x2,
		/// Should virtual terminal sequences be processed?
		VirtualTerminalProcessing = 0x4,
		/// Should the console cursor return to the the first column on newline?
		DisableNewlineAutoReturn = 0x8,
		/// Should LVB attribute flags be enabled?
		LVBGridWorldwide = 0x10,
	}
}
