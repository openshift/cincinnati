use super::*;

#[cfg(feature = "serde")]
use serde::ser::{Serialize, Serializer, SerializeStruct};
#[cfg(feature = "serde")]
use serde::de::{self, Deserialize, Deserializer, Visitor, SeqAccess, MapAccess};
#[cfg(feature = "serde")]
use std::fmt;

/// Represents an input event which occurred as a result of a buffer resize.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ResizeEvent {
	/// The size of the screen buffer.
	pub size: Vector2<u16>
}

impl ResizeEvent {
	/**
	 Returns an empty ResizeEvent.
	 */
	pub fn new() -> ResizeEvent {
		ResizeEvent {
			size: Vector2::new(0, 0)
		}
	}
}

impl Into<InputEvent> for ResizeEvent {
	fn into(self) -> InputEvent {
		InputEvent::Resize(self)
	}
}

#[cfg(feature = "serde")]
impl Serialize for ResizeEvent {
	fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
	where S: Serializer {
		let mut s = serializer.serialize_struct("ResizeEvent", 2)?;
		s.serialize_field("x", &self.size.x)?;
		s.serialize_field("y", &self.size.y)?;
		s.end()
	}
}

#[cfg(feature = "serde")]
impl<'de> Deserialize<'de> for ResizeEvent {
	fn deserialize<D>(deserializer: D) -> Result<ResizeEvent, D::Error>
	where D: Deserializer<'de> {
		enum Field {
			X,
			Y
		};

		impl<'de> Deserialize<'de> for Field {
			fn deserialize<D>(deserializer: D) -> Result<Field, D::Error>
			where D: Deserializer<'de> {
				struct FieldVisitor;

				impl<'de> Visitor<'de> for FieldVisitor {
					type Value = Field;

					fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
						formatter.write_str("`x` or `y`")
					}

					fn visit_str<E>(self, value: &str) -> Result<Field, E>
					where E: de::Error {
						match value {
							"x" => Ok(Field::X),
							"y" => Ok(Field::Y),
							_ => Err(de::Error::unknown_field(value, FIELDS))
						}
					}
				}

				deserializer.deserialize_identifier(FieldVisitor)
			}
		}

		struct ResizeEventVisitor;

		impl<'de> Visitor<'de> for ResizeEventVisitor {
			type Value = ResizeEvent;

			fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
				formatter.write_str("struct ResizeEvent")
			}

			fn visit_seq<V>(self, mut seq: V) -> Result<ResizeEvent, V::Error>
			where V: SeqAccess<'de> {
				macro_rules! element {
					($name:ident, $val:expr) => {
						let $name = seq.next_element()?
							.ok_or_else(|| de::Error::invalid_length($val, &self))?;
					};
				}
				element!(x, 0);
				element!(y, 1);

				let mut ret = ResizeEvent::new();
				ret.size = Vector2::new(x, y);

				Ok(ret)
			}

			fn visit_map<V>(self, mut map: V) -> Result<ResizeEvent, V::Error>
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

				set!(x, true);
				set!(y, true);

				while let Some(key) = map.next_key()? {
					match key {
						Field::X => field!(x),
						Field::Y => field!(y)
					}
				}

				set!(x);
				set!(y);

				let mut ret = ResizeEvent::new();
				ret.size = Vector2::new(x, y);

				Ok(ret)
			}
		}

		const FIELDS: &'static [&'static str] = &["x", "y"];
		deserializer.deserialize_struct("ResizeEvent", FIELDS, ResizeEventVisitor)
	}
}
