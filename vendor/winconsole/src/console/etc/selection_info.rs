use cgmath::Vector2;
use super::Rect;

#[cfg(feature = "serde")]
use serde::ser::{Serialize, Serializer, SerializeStruct};
#[cfg(feature = "serde")]
use serde::de::{self, Deserialize, Deserializer, Visitor, SeqAccess, MapAccess};
#[cfg(feature = "serde")]
use std::fmt;

/// Information about a console selection.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SelectionInfo {
	/// The console selection anchor.
	pub anchor: Vector2<u16>,
	/// Is the selection empty?
	pub empty: bool,
	/// Is the mouse down?
	pub mouse_down: bool,
	/// The selection rectangle.
	pub rect: Rect,
	/// Is a selection occurring?
	pub selecting: bool
}

impl SelectionInfo {
	/**
	 Returns an empty SelectionInfo object.
	 */
	pub fn new() -> SelectionInfo {
		SelectionInfo {
			anchor: Vector2::new(0, 0),
			empty: false,
			mouse_down: false,
			rect: Rect::new(0, 0, 0, 0),
			selecting: false
		}
	}
}

#[cfg(feature = "serde")]
impl Serialize for SelectionInfo {
	fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
	where S: Serializer {
		let mut s = serializer.serialize_struct("SelectionInfo", 6)?;
		s.serialize_field("anchor_x", &self.anchor.x)?;
		s.serialize_field("anchor_y", &self.anchor.y)?;
		s.serialize_field("empty", &self.empty)?;
		s.serialize_field("mouse_down", &self.mouse_down)?;
		s.serialize_field("rect", &self.rect)?;
		s.serialize_field("selecting", &self.selecting)?;
		s.end() 
	}
}

#[cfg(feature = "serde")]
impl<'de> Deserialize<'de> for SelectionInfo {
	fn deserialize<D>(deserializer: D) -> Result<SelectionInfo, D::Error>
	where D: Deserializer<'de> {
		enum Field {
			AnchorX,
			AnchorY,
			Empty,
			MouseDown,
			Rect,
			Selecting
		};

		impl<'de> Deserialize<'de> for Field {
			fn deserialize<D>(deserializer: D) -> Result<Field, D::Error>
			where D: Deserializer<'de> {
				struct FieldVisitor;

				impl<'de> Visitor<'de> for FieldVisitor {
					type Value = Field;

					fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
						formatter.write_str(concat!(
							"`anchor_x`, `anchor_y`,",
							" `empty`, `mouse_down`, `rect`, or `selecting`"
						))
					}

					fn visit_str<E>(self, value: &str) -> Result<Field, E>
					where E: de::Error {
						match value {
							"anchor_x" => Ok(Field::AnchorX),
							"anchor_y" => Ok(Field::AnchorY),
							"empty" => Ok(Field::Empty),
							"mouse_down" => Ok(Field::MouseDown),
							"rect" => Ok(Field::Rect),
							"selecting" => Ok(Field::Selecting),
							_ => Err(de::Error::unknown_field(value, FIELDS)),
						}
					}
				}

				deserializer.deserialize_identifier(FieldVisitor)
			}
		}

		struct SelectionInfoVisitor;

		impl<'de> Visitor<'de> for SelectionInfoVisitor {
			type Value = SelectionInfo;

			fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
				formatter.write_str("struct SelectionInfo")
			}

			fn visit_seq<V>(self, mut seq: V) -> Result<SelectionInfo, V::Error>
			where V: SeqAccess<'de> {
				macro_rules! element {
					($name:ident, $val:expr) => {
						let $name = seq.next_element()?
							.ok_or_else(|| de::Error::invalid_length($val, &self))?;
					};
				}
				element!(anchor_x, 0);
				element!(anchor_y, 1);
				element!(empty, 2);
				element!(mouse_down, 3);
				element!(rect, 4);
				element!(selecting, 5);

				let mut ret = SelectionInfo::new();
				ret.anchor = Vector2::new(anchor_x, anchor_y);
				ret.empty = empty;
				ret.mouse_down = mouse_down;
				ret.rect = rect;
				ret.selecting = selecting;

				Ok(ret)
			}

			fn visit_map<V>(self, mut map: V) -> Result<SelectionInfo, V::Error>
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

				set!(anchor_x, true);
				set!(anchor_y, true);
				set!(empty, true);
				set!(mouse_down, true);
				set!(rect, true);
				set!(selecting, true);

				while let Some(key) = map.next_key()? {
					match key {
						Field::AnchorX => field!(anchor_x),
						Field::AnchorY => field!(anchor_y),
						Field::Empty => field!(empty),
						Field::MouseDown => field!(mouse_down),
						Field::Rect => field!(rect),
						Field::Selecting => field!(selecting),
					}
				}

				set!(anchor_x);
				set!(anchor_y);
				set!(empty);
				set!(mouse_down);
				set!(rect);
				set!(selecting);

				let mut ret = SelectionInfo::new();
				ret.anchor = Vector2::new(anchor_x, anchor_y);
				ret.empty = empty;
				ret.mouse_down = mouse_down;
				ret.rect = rect;
				ret.selecting = selecting;

				Ok(ret)
			}
		}

		const FIELDS: &'static [&'static str] = &[
			"anchor_x", "anchor_y",
			"empty", "mouse_down",
			"rect", "selecting"
		];
		deserializer.deserialize_struct("SelectionInfo", FIELDS, SelectionInfoVisitor)
	}
}
