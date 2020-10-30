use cgmath::Vector2;

#[cfg(feature = "serde")]
use serde::ser::{Serialize, Serializer, SerializeStruct};
#[cfg(feature = "serde")]
use serde::de::{self, Deserialize, Deserializer, Visitor, SeqAccess, MapAccess};
#[cfg(feature = "serde")]
use std::fmt;

/// Represents a usable console font.
#[derive(Clone, Debug, PartialEq)]
pub struct ConsoleFont {
	/**
	 An integer which describes the font family.
	 See the `tmPitchAndFamily` field
	 [here](https://msdn.microsoft.com/en-us/library/windows/desktop/dd145132(v=vs.85).aspx).
	 */
	pub family: u32,
	/// The index of the font in the console font table.
	pub index: u32,
	/// The name of the font.
	pub name: String,
	/// The font size.
	pub size: Vector2<u16>,
	/**
	 The font weight. Accepts values which are multiples of 100, with
	 400 representing normal weight and 700 representing bold.
	 */
	pub weight: u32
}
impl ConsoleFont {
	/**
	 Returns an empty ConsoleFont object.
	 */
	pub fn new() -> ConsoleFont {
		ConsoleFont {
			name: String::new(),
			size: Vector2::new(0, 0),
			weight: 0,
			family: 0,
			index: 0
		}
	}
}

#[cfg(feature = "serde")]
impl Serialize for ConsoleFont {
	fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
	where S: Serializer {
		let mut s = serializer.serialize_struct("ConsoleFont", 6)?;
		s.serialize_field("family", &self.family)?;
		s.serialize_field("index", &self.index)?;
		s.serialize_field("name", &self.name)?;
		s.serialize_field("width", &self.size.x)?;
		s.serialize_field("height", &self.size.y)?;
		s.serialize_field("weight", &self.weight)?;
		s.end()
	}
}

#[cfg(feature = "serde")]
impl<'de> Deserialize<'de> for ConsoleFont {
	fn deserialize<D>(deserializer: D) -> Result<ConsoleFont, D::Error>
	where D: Deserializer<'de> {
		enum Field {
			Family,
			Index,
			Name,
			Width,
			Height,
			Weight
		};

		impl<'de> Deserialize<'de> for Field {
			fn deserialize<D>(deserializer: D) -> Result<Field, D::Error>
			where D: Deserializer<'de> {
				struct FieldVisitor;

				impl<'de> Visitor<'de> for FieldVisitor {
					type Value = Field;

					fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
						formatter.write_str("`family`, `index`, `name`, `width`, `height`, `weight`")
					}

					fn visit_str<E>(self, value: &str) -> Result<Field, E>
					where E: de::Error {
						match value {
							"family" => Ok(Field::Family),
							"index" => Ok(Field::Index),
							"name" => Ok(Field::Name),
							"width" => Ok(Field::Width),
							"height" => Ok(Field::Height),
							"weight" => Ok(Field::Weight),
							_ => Err(de::Error::unknown_field(value, FIELDS)),
						}
					}
				}

				deserializer.deserialize_identifier(FieldVisitor)
			}
		}

		struct ConsoleFontVisitor;

		impl<'de> Visitor<'de> for ConsoleFontVisitor {
			type Value = ConsoleFont;

			fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
				formatter.write_str("struct ConsoleFont")
			}

			fn visit_seq<V>(self, mut seq: V) -> Result<ConsoleFont, V::Error>
			where V: SeqAccess<'de> {
				macro_rules! element {
					($name:ident, $val:expr) => {
						let $name = seq.next_element()?
							.ok_or_else(|| de::Error::invalid_length($val, &self))?;
					};
				}
				element!(family, 0);
				element!(index, 1);
				element!(name, 2);
				element!(width, 3);
				element!(height, 4);
				element!(weight, 5);

				let mut ret = ConsoleFont::new();
				ret.family = family;
				ret.index = index;
				ret.name = name;
				ret.size = Vector2::new(width, height);
				ret.weight = weight;

				Ok(ret)
			}

			fn visit_map<V>(self, mut map: V) -> Result<ConsoleFont, V::Error>
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

				set!(family, true);
				set!(index, true);
				set!(name, true);
				set!(width, true);
				set!(height, true);
				set!(weight, true);

				while let Some(key) = map.next_key()? {
					match key {
						Field::Family => field!(family),
						Field::Index => field!(index),
						Field::Name => field!(name),
						Field::Width => field!(width),
						Field::Height => field!(height),
						Field::Weight => field!(weight)
					}
				}

				set!(family);
				set!(index);
				set!(name);
				set!(width);
				set!(height);
				set!(weight);

				let mut ret = ConsoleFont::new();
				ret.family = family;
				ret.index = index;
				ret.name = name;
				ret.size = Vector2::new(width, height);
				ret.weight = weight;

				Ok(ret)
			}
		}

		const FIELDS: &'static [&'static str] = &[
			"anchor_x", "anchor_y", "bottom_right_x",
			"bottom_right_y", "empty", "mouse_down",
			"rect", "selecting", "top_left_x",
			"top_left_y"
		];
		deserializer.deserialize_struct("ConsoleFont", FIELDS, ConsoleFontVisitor)
	}
}
