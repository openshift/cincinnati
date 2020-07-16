use super::CodePage;

/// Information about a code page.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct CodePageInfo {
	/// The maximum length, in bytes, of a character in the code page.
	pub max_char_size: u8,
	/// The default character used when translating to the code page.
	pub default: String,
	/// An array of lead byte ranges.
	pub lead_byte: [u8; 12],
	/// The default unicode character used when translating to the code page.
	pub unicode_default: String,
	/// The code page associated with the information.
	pub code_page: CodePage,
	/// The full name of the code page.
	pub name: String
}

impl CodePageInfo {
	/**
	 Returns an empty CodePageInfo object.
	 */
	pub fn new() -> CodePageInfo {
		CodePageInfo {
			max_char_size: 0,
			default: String::new(),
			lead_byte: [0; 12],
			unicode_default: String::new(),
			code_page: CodePage::None,
			name: String::new()
		}
	}
}
