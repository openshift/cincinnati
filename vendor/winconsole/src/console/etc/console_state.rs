use cgmath::Vector2;
use rgb::RGB8;
use super::{CodePage, ConsoleColor, ConsoleFont, InputSettings, OutputSettings};

#[cfg(feature = "serde")]
use serde::ser::{Serialize, Serializer, SerializeStruct};
#[cfg(feature = "serde")]
use serde::de::{self, Deserialize, Deserializer, Visitor, SeqAccess, MapAccess};
#[cfg(feature = "serde")]
use std::fmt;

/// Represents the state of the console.
#[derive(Clone, Debug, PartialEq)]
pub struct ConsoleState {
	/// The background color of the console.
	pub background_color: ConsoleColor,
	/// The console's buffer size.
	pub buffer_size: Vector2<u16>,
	/// The color mapping of the console.
	pub color_mapping: [RGB8; 16],
	/// The console cursor position.
	pub cursor_position: Vector2<u16>,
	/// The console cursor size.
	pub cursor_size: u8,
	/// The visibility of the console cursor.
	pub cursor_visible: bool,
	/// The console font information.
	pub font: ConsoleFont,
	/// The foreground color of the console.
	pub foreground_color: ConsoleColor,
	/// The console input code page.
	pub input_code_page: CodePage,
	/// The console input mode.
	pub input_mode: InputSettings,
	/// The console's output contents.
	pub output: String,
	/// The console output code page.
	pub output_code_page: CodePage,
	/// The colors of the console's output contents.
	pub output_colors: Vec<(ConsoleColor, ConsoleColor)>,
	/// The console output mode.
	pub output_mode: OutputSettings,
	/// The console window title.
	pub title: String
}

impl ConsoleState {
	/**
	 Returns an empty ConsoleState object.
	 */
	pub fn new() -> ConsoleState {
		ConsoleState {
			background_color: ConsoleColor::Black,
			buffer_size: Vector2::new(0, 0),
			color_mapping: [RGB8 { r: 0, g: 0, b: 0 }; 16],
			cursor_position: Vector2::new(0, 0),
			cursor_size: 0,
			cursor_visible: false,
			font: ConsoleFont::new(),
			foreground_color: ConsoleColor::Black,
			input_mode: InputSettings::new(),
			input_code_page: CodePage::None,
			output: String::new(),
			output_code_page: CodePage::None,
			output_colors: Vec::new(),
			output_mode: OutputSettings::new(),
			title: String::new()
		}
	}
}

#[cfg(feature = "serde")]
impl Serialize for ConsoleState {
	fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
	where S: Serializer {
		let mapping = {
			let mut map = [0i32; 16];
			for (i, color) in self.color_mapping.iter().enumerate() {
				map[i] = make_colorref!(color) as i32;
			}
			map
		};
		let mut s = serializer.serialize_struct("ConsoleState", 17)?;
		s.serialize_field("background_color", &self.background_color)?;
		s.serialize_field("buffer_x", &self.buffer_size.x)?;
		s.serialize_field("buffer_y", &self.buffer_size.y)?;
		s.serialize_field("color_mapping", &mapping)?;
		s.serialize_field("cursor_x", &self.cursor_position.x)?;
		s.serialize_field("cursor_y", &self.cursor_position.y)?;
		s.serialize_field("cursor_size", &self.cursor_size)?;
		s.serialize_field("cursor_visible", &self.cursor_visible)?;
		s.serialize_field("font", &self.font)?;
		s.serialize_field("foreground_color", &self.foreground_color)?;
		s.serialize_field("input_code_page", &self.input_code_page)?;
		s.serialize_field("input_mode", &self.input_mode)?;
		s.serialize_field("output", &self.output)?;
		s.serialize_field("output_code_page", &self.output_code_page)?;
		s.serialize_field("output_colors", &self.output_colors)?;
		s.serialize_field("output_mode", &self.output_mode)?;
		s.serialize_field("title", &self.title)?;
		s.end() 
	}
}

#[cfg(feature = "serde")]
impl<'de> Deserialize<'de> for ConsoleState {
	fn deserialize<D>(deserializer: D) -> Result<ConsoleState, D::Error>
	where D: Deserializer<'de> {
		enum Field {
			BackgroundColor,
			BufferX,
			BufferY,
			ColorMapping,
			CursorX,
			CursorY,
			CursorSize,
			CursorVisible,
			Font,
			ForegroundColor,
			InputCodePage,
			InputMode,
			Output,
			OutputCodePage,
			OutputColors,
			OutputMode,
			Title
		};

		impl<'de> Deserialize<'de> for Field {
			fn deserialize<D>(deserializer: D) -> Result<Field, D::Error>
			where D: Deserializer<'de> {
				struct FieldVisitor;

				impl<'de> Visitor<'de> for FieldVisitor {
					type Value = Field;

					fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
						formatter.write_str(concat!(
							"`background_color`, `buffer_x`, `buffer_y`, color_mapping`, `cursor_x`",
							" `cursor_y`, `cursor_size`, `cursor_visible`, `font`,",
							" `foreground_color`, `input_code_page`, `input_mode`, `output`",
							" `output_code_page`, `output_colors`, `output_mode`, or `title`"
						))
					}

					fn visit_str<E>(self, value: &str) -> Result<Field, E>
					where E: de::Error {
						match value {
							"background_color" => Ok(Field::BackgroundColor),
							"buffer_x" => Ok(Field::BufferX),
							"buffer_y" => Ok(Field::BufferY),
							"color_mapping" => Ok(Field::ColorMapping),
							"cursor_x" => Ok(Field::CursorX),
							"cursor_y" => Ok(Field::CursorY),
							"cursor_size" => Ok(Field::CursorSize),
							"cursor_visible" => Ok(Field::CursorVisible),
							"font" => Ok(Field::Font),
							"foreground_color" => Ok(Field::ForegroundColor),
							"input_code_page" => Ok(Field::InputCodePage),
							"input_mode" => Ok(Field::InputMode),
							"output" => Ok(Field::Output),
							"output_code_page" => Ok(Field::OutputCodePage),
							"output_colors" => Ok(Field::OutputColors),
							"output_mode" => Ok(Field::OutputMode),
							"title" => Ok(Field::Title),
							_ => Err(de::Error::unknown_field(value, FIELDS)),
						}
					}
				}

				deserializer.deserialize_identifier(FieldVisitor)
			}
		}

		struct ConsoleStateVisitor;

		impl<'de> Visitor<'de> for ConsoleStateVisitor {
			type Value = ConsoleState;

			fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
				formatter.write_str("struct ConsoleState")
			}

			fn visit_seq<V>(self, mut seq: V) -> Result<ConsoleState, V::Error>
			where V: SeqAccess<'de> {
				macro_rules! element {
					($name:ident, $val:expr) => {
						let $name = seq.next_element()?
							.ok_or_else(|| de::Error::invalid_length($val, &self))?;
					};
				}

				element!(background_color, 0);
				element!(buffer_x, 1);
				element!(buffer_y, 2);
				let color_mapping: [i32; 16] = seq.next_element()?
					.ok_or_else(|| de::Error::invalid_length(3, &self))?;
				element!(cursor_x, 4);
				element!(cursor_y, 5);
				element!(cursor_size, 6);
				element!(cursor_visible, 7);
				element!(font, 8);
				element!(foreground_color, 9);
				element!(input_code_page, 10);
				element!(input_mode, 11);
				element!(output, 12);
				element!(output_code_page, 13);
				element!(output_colors, 14);
				element!(output_mode, 15);
				element!(title, 16);

				let mut ret = ConsoleState::new();
				ret.background_color = background_color;
				ret.buffer_size = Vector2::new(buffer_x, buffer_y);
				ret.cursor_position = Vector2::new(cursor_x, cursor_y);
				ret.cursor_size = cursor_size;
				ret.cursor_visible = cursor_visible;
				ret.font = font;
				ret.foreground_color = foreground_color;
				ret.input_code_page = input_code_page;
				ret.input_mode = input_mode;
				ret.output = output;
				ret.output_code_page = output_code_page;
				ret.output_colors = output_colors;
				ret.output_mode = output_mode;
				ret.title = title;
				for (i, color) in color_mapping.iter().enumerate() {
					ret.color_mapping[i] = make_rgb!(color);
				}

				Ok(ret)
			}

			fn visit_map<V>(self, mut map: V) -> Result<ConsoleState, V::Error>
			where V: MapAccess<'de> {
				macro_rules! field {
					($name:ident) => {
						{
							if $name.is_some() {
								return Err(de::Error::duplicate_field(stringify!($name)));
							}
							$name = Some(map.next_value()?);
						}
					};
				}
				macro_rules! set {
					($name:ident) => {
						let $name = $name.ok_or_else(|| de::Error::missing_field(stringify!($name)))?;
					};
					($name:ident, $x:expr) => {
						let mut $name = None;
					}
				}

				set!(background_color, true);
				set!(buffer_x, true);
				set!(buffer_y, true);
				let mut color_mapping: Option<[i32; 16]> = None;
				set!(cursor_x, true);
				set!(cursor_y, true);
				set!(cursor_size, true);
				set!(cursor_visible, true);
				set!(font, true);
				set!(foreground_color, true);
				set!(input_code_page, true);
				set!(input_mode, true);
				set!(output, true);
				set!(output_code_page, true);
				set!(output_colors, true);
				set!(output_mode, true);
				set!(title, true);

				while let Some(key) = map.next_key()? {
					match key {
						Field::BackgroundColor => field!(background_color),
						Field::BufferX => field!(buffer_x),
						Field::BufferY => field!(buffer_y),
						Field::ColorMapping => field!(color_mapping),
						Field::CursorX => field!(cursor_x),
						Field::CursorY => field!(cursor_y),
						Field::CursorSize => field!(cursor_size),
						Field::CursorVisible => field!(cursor_visible),
						Field::Font => field!(font),
						Field::ForegroundColor => field!(foreground_color),
						Field::InputCodePage => field!(input_code_page),
						Field::InputMode => field!(input_mode),
						Field::Output => field!(output),
						Field::OutputCodePage => field!(output_code_page),
						Field::OutputColors => field!(output_colors),
						Field::OutputMode => field!(output_mode),
						Field::Title => field!(title)
					}
				}

				set!(background_color);
				set!(buffer_x);
				set!(buffer_y);
				set!(color_mapping);
				set!(cursor_x);
				set!(cursor_y);
				set!(cursor_size);
				set!(cursor_visible);
				set!(font);
				set!(foreground_color);
				set!(input_code_page);
				set!(input_mode);
				set!(output);
				set!(output_code_page);
				set!(output_colors);
				set!(output_mode);
				set!(title);

				let mut ret = ConsoleState::new();
				ret.background_color = background_color;
				ret.buffer_size = Vector2::new(buffer_x, buffer_y);
				ret.cursor_position = Vector2::new(cursor_x, cursor_y);
				ret.cursor_size = cursor_size;
				ret.cursor_visible = cursor_visible;
				ret.font = font;
				ret.foreground_color = foreground_color;
				ret.input_code_page = input_code_page;
				ret.input_mode = input_mode;
				ret.output = output;
				ret.output_code_page = output_code_page;
				ret.output_colors = output_colors;
				ret.output_mode = output_mode;
				ret.title = title;
				for (i, color) in color_mapping.iter().enumerate() {
					ret.color_mapping[i] = make_rgb!(color);
				}

				Ok(ret)
			}
		}

		const FIELDS: &'static [&'static str] = &[
			"background_color", "buffer_x", "buffer_y", "color_mapping",
			"cursor_x", "cursor_y", "cursor_size", "cursor_visible",
			"font", "foreground_color", "input_code_page", "input_mode",
			"output", "output_code_page", "output_colors", "output_mode",
			"title",
		];
		deserializer.deserialize_struct("ConsoleState", FIELDS, ConsoleStateVisitor)
	}
}
