enumeration! {
	/// Represents a virtual key code.
	#[repr(u8)]
	KeyCode<u8> {
		__DEFAULT__ = 0xff,
		/// Default value.
		None = 0x0,
		/// Left mouse button.
		LButton = 0x01,
		/// Right mouse button.
		RButton = 0x02,
		/// Control-break processing.
		Cancel = 0x03,
		/// Middle mouse button (three-button mouse).
		MButton = 0x04,
		/// X1 mouse button.
		XButton1 = 0x05,
		/// X2 mouse button.
		XButton2 = 0x06,
		/// Backspace key.
		Backspace = 0x08,
		/// Tab key.
		Tab = 0x09,
		/// Clear key.
		Clear = 0x0c,
		/// Enter key.
		Return = 0x0d,
		/// Shift key.
		Shift = 0x10,
		/// Ctrl key.
		Control = 0x11,
		/// Alt key.
		Menu = 0x12,
		/// Pause key.
		Pause = 0x13,
		/// Caps Lock key.
		Capital = 0x14,
		/// Ime Kana/Hangul Mode.
		KanaHangul = 0x15,
		/// Ime Junja Mode.
		Junja = 0x17,
		/// Ime Final Mode.
		Final = 0x18,
		/// Ime Hanja/Kanji Mode.
		HanjaKanji = 0x19,
		/// Escape key.
		Escape = 0x1b,
		/// Ime Convert.
		Convert = 0x1c,
		/// Ime Nonconvert.
		NonConvert = 0x1d,
		/// Ime Accept.
		Accept = 0x1e,
		/// Ime Mode Change Request.
		ModeChange = 0x1f,
		/// Spacebar.
		Space = 0x20,
		/// Page Up key.
		PageUp = 0x21,
		/// Page Down key.
		PageDown = 0x22,
		/// End key.
		End = 0x23,
		/// Home key.
		Home = 0x24,
		/// Left Arrow key.
		Left = 0x25,
		/// Up Arrow key.
		Up = 0x26,
		/// Right Arrow key.
		Right = 0x27,
		/// Down Arrow key.
		Down = 0x28,
		/// Select key.
		Select = 0x29,
		/// Print key.
		Print = 0x2a,
		/// Execute key.
		Execute = 0x2b,
		/// Print Screen key.
		Snapshot = 0x2c,
		/// Ins key.
		Insert = 0x2d,
		/// Del key.
		Delete = 0x2e,
		/// Help key.
		Help = 0x2f,
		/// 0 key.
		Zero = 0x30,
		/// 1 key.
		One = 0x31,
		/// 2 key.
		Two = 0x32,
		/// 3 key.
		Three = 0x33,
		/// 4 key.
		Four = 0x34,
		/// 5 key.
		Five = 0x35,
		/// 6 key.
		Six = 0x36,
		/// 7 key.
		Seven = 0x37,
		/// 8 key.
		Eight = 0x38,
		/// 9 key.
		Nine = 0x39,
		/// A key.
		A = 0x41,
		/// B key.
		B = 0x42,
		/// C key.
		C = 0x43,
		/// D key.
		D = 0x44,
		/// E key.
		E = 0x45,
		/// F key.
		F = 0x46,
		/// G key.
		G = 0x47,
		/// H key.
		H = 0x48,
		/// I key.
		I = 0x49,
		/// J key.
		J = 0x4a,
		/// K key.
		K = 0x4b,
		/// L key.
		L = 0x4c,
		/// M key.
		M = 0x4d,
		/// N key.
		N = 0x4e,
		/// O key.
		O = 0x4f,
		/// P key.
		P = 0x50,
		/// Q key.
		Q = 0x51,
		/// R key.
		R = 0x52,
		/// S key.
		S = 0x53,
		/// T key.
		T = 0x54,
		/// U key.
		U = 0x55,
		/// V key.
		V = 0x56,
		/// W key.
		W = 0x57,
		/// X key.
		X = 0x58,
		/// Y key.
		Y = 0x59,
		/// Z key.
		Z = 0x5a,
		/// Left Windows Key (natural Keyboard).
		LWin = 0x5b,
		/// Right Windows Key (natural Keyboard).
		RWin = 0x5c,
		/// Applications Key (natural Keyboard).
		Apps = 0x5d,
		/// Computer Sleep key.
		Sleep = 0x5f,
		/// Numeric Keypad 0 key.
		Numpad0 = 0x60,
		/// Numeric Keypad 1 key.
		Numpad1 = 0x61,
		/// Numeric Keypad 2 key.
		Numpad2 = 0x62,
		/// Numeric Keypad 3 key.
		Numpad3 = 0x63,
		/// Numeric Keypad 4 key.
		Numpad4 = 0x64,
		/// Numeric Keypad 5 key.
		Numpad5 = 0x65,
		/// Numeric Keypad 6 key.
		Numpad6 = 0x66,
		/// Numeric Keypad 7 key.
		Numpad7 = 0x67,
		/// Numeric Keypad 8 key.
		Numpad8 = 0x68,
		/// Numeric Keypad 9 key.
		Numpad9 = 0x69,
		/// Multiply key.
		Multiply = 0x6a,
		/// Add key.
		Add = 0x6b,
		/// Separator key.
		Separator = 0x6c,
		/// Subtract key.
		Subtract = 0x6d,
		/// Decimal key.
		Decimal = 0x6e,
		/// Divide key.
		Divide = 0x6f,
		/// F1 key.
		F1 = 0x70,
		/// F2 key.
		F2 = 0x71,
		/// F3 key.
		F3 = 0x72,
		/// F4 key.
		F4 = 0x73,
		/// F5 key.
		F5 = 0x74,
		/// F6 key.
		F6 = 0x75,
		/// F7 key.
		F7 = 0x76,
		/// F8 key.
		F8 = 0x77,
		/// F9 key.
		F9 = 0x78,
		/// F10 key.
		F10 = 0x79,
		/// F11 key.
		F11 = 0x7a,
		/// F12 key.
		F12 = 0x7b,
		/// F13 key.
		F13 = 0x7c,
		/// F14 key.
		F14 = 0x7d,
		/// F15 key.
		F15 = 0x7e,
		/// F16 key.
		F16 = 0x7f,
		/// F17 key.
		F17 = 0x80,
		/// F18 key.
		F18 = 0x81,
		/// F19 key.
		F19 = 0x82,
		/// F20 key.
		F20 = 0x83,
		/// F21 key.
		F21 = 0x84,
		/// F22 key.
		F22 = 0x85,
		/// F23 key.
		F23 = 0x86,
		/// F24 key.
		F24 = 0x87,
		/// Num Lock key.
		NumLock = 0x90,
		/// Scroll Lock key.
		Scroll = 0x91,
		/// Left Shift key.
		LShift = 0xa0,
		/// Right Shift key.
		RShift = 0xa1,
		/// Left Control key.
		LControl = 0xa2,
		/// Right Control key.
		RControl = 0xa3,
		/// Left Menu key.
		LMenu = 0xa4,
		/// Right Menu key.
		RMenu = 0xa5,
		/// Browser Back key.
		BrowserBack = 0xa6,
		/// Browser Forward key.
		BrowserForward = 0xa7,
		/// Browser Refresh key.
		BrowserRefresh = 0xa8,
		/// Browser Stop key.
		BrowserStop = 0xa9,
		/// Browser Search key.
		BrowserSearch = 0xaa,
		/// Browser Favorites key.
		BrowserFavorites = 0xab,
		/// Browser Start And Home key.
		BrowserHome = 0xac,
		/// Volume Mute key.
		VolumeMute = 0xad,
		/// Volume Down key.
		VolumeDown = 0xae,
		/// Volume Up key.
		VolumeUp = 0xaf,
		/// Next Track key.
		MediaNextTrack = 0xb0,
		/// Previous Track key.
		MediaPrevTrack = 0xb1,
		/// Stop Media key.
		MediaStop = 0xb2,
		/// Play/pause Media key.
		MediaPlayPause = 0xb3,
		/// Start Mail key.
		LaunchMail = 0xb4,
		/// Select Media key.
		LaunchMediaSelect = 0xb5,
		/// Start Application 1 key.
		LaunchApp1 = 0xb6,
		/// Start Application 2 key.
		LaunchApp2 = 0xb7,
		/// Used for miscellaneous characters; it can vary by keyboard.
		Oem1 = 0xba,
		/// The '+' key.
		Plus = 0xbb,
		/// The ',' key.
		Comma = 0xbc,
		/// The '-' key.
		Minus = 0xbd,
		/// The '.' key.
		Period = 0xbe,
		/// Used for miscellaneous characters; it can vary by keyboard.
		Oem2 = 0xbf,
		/// Used for miscellaneous characters; it can vary by keyboard.
		Oem3 = 0xc0,
		/// For the U.S. standard keyboard, the '[{' key.
		Oem4 = 0xdb,
		/// For the U.S. standard keyboard, the '\|' key.
		Oem5 = 0xdc,
		/// For the U.S. standard keyboard, the ']}' key.
		Oem6 = 0xdd,
		/// For the U.S. standard keyboard, the 'single-quote/double-quote' key.
		Oem7 = 0xde,
		/// Used for miscellaneous characters; it can vary by keyboard.
		Oem8 = 0xdf,
		/// Either the angle bracket key or the backslash key on the RT 102-key keyboard.
		Oem102 = 0xe2,
		/// Ime Process key.
		ProcessKey = 0xe5,
		/// Attn key.
		Attn = 0xf6,
		/// Crsel key.
		CrSel = 0xf7,
		/// Exsel key.
		ExSel = 0xf8,
		/// Erase Eof key.
		ErEOF = 0xf9,
		/// Play key.
		Play = 0xfa,
		/// Zoom key.
		Zoom = 0xfb,
		/// Reserved.
		NoName = 0xfc,
		/// Pa1 key.
		PA1 = 0xfd,
		/// Clear key.
		OEMClear = 0xfe,
		/// Returned from keys with no mapping.
		NoMapping = 0xff,
	}
}
