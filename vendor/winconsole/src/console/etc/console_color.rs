enumeration! {
	/// Represents supported console colors.
	#[repr(u8)]
	ConsoleColor<u8, u16> {
		__DEFAULT__ = 0xF,
		/// The color black; defaults to #000000.
		Black = 0x0,
		/// The color dark blue; defaults to #000080.
		DarkBlue = 0x1,
		/// The color dark green; defaults to #008000.
		DarkGreen = 0x2,
		/// The color teal; defaults to #008080.
		Teal = 0x3,
		/// The color dark red; defaults to #800000.
		DarkRed = 0x4,
		/// The color magenta; defaults to #800080.
		Magenta = 0x5,
		/// The color dark yellow; defaults to #808000.
		DarkYellow = 0x6,
		/// The color gray; defaults to #C0C0C0.
		Gray = 0x7,
		/// The color dark gray; defaults to #808080.
		DarkGray = 0x8,
		/// The color blue; defaults to #0000FF.
		Blue = 0x9,
		/// The color green; defaults to #00FF00.
		Green = 0xA,
		/// The color aqua; defaults to #00FFFF.
		Aqua = 0xB,
		/// The color red; defaults to #FF0000.
		Red = 0xC,
		/// The color pink; defaults to #FF00FF.
		Pink = 0xD,
		/// The color yellow; defaults to #FFFF00.
		Yellow = 0xE,
		/// The color white; defaults to #FFFFFF.
		White = 0xF,
	}
}
