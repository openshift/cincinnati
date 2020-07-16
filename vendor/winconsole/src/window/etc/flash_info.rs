/// Information used to flash the window.
#[derive(Clone, Copy, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct FlashInfo {
	/// The number of times a flash should occur.
	pub count: u32,
	/// Should the window caption flash?
	pub flash_caption: bool,
	/// Should the tray icon flash?
	pub flash_tray: bool,
	/// Should the flash continue indefinitely?
	pub indefinite: bool,
	/// The rate at which the window should flash, in milliseconds.
	/// If this is zero, the default blink rate is used.
	pub rate: u32,
	/// Should the flash continue until the window comes to the foreground?
	pub until_foreground: bool
}

impl FlashInfo {
	/**
	 Returns an empty FlashInfo object.
	 */
	pub fn new() -> FlashInfo {
		FlashInfo {
			count: 0,
			flash_caption: false,
			flash_tray: false,
			indefinite: false,
			rate: 0,
			until_foreground: false
		}
	}
}
