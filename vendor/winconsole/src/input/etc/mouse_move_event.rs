use super::*;

#[cfg(feature = "serde")]
use serde::ser::{Serialize, Serializer, SerializeStruct};
#[cfg(feature = "serde")]
use serde::de::{self, Deserialize, Deserializer, Visitor, SeqAccess, MapAccess};
#[cfg(feature = "serde")]
use std::fmt;

/// Represents an input event which occurred as a result of mouse movement.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MouseMoveEvent {
	/// A ControlKeyState object describing the state of control keys.
	pub modifiers: ControlKeyState,
	/// The character cell the event occurred on.
	pub position: Vector2<u16>
}

impl MouseMoveEvent {
	/**
	 Returns an empty MouseMoveEvent.
	 */
	pub fn new() -> MouseMoveEvent {
		MouseMoveEvent {
			modifiers: ControlKeyState::new(),
			position: Vector2::new(0, 0),
		}
	}
}

impl Into<InputEvent> for MouseMoveEvent {
	fn into(self) -> InputEvent {
		InputEvent::MouseMove(self)
	}
}

#[cfg(feature = "serde")]
impl Serialize for MouseMoveEvent {
	fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
	where S: Serializer {
		let mut s = serializer.serialize_struct("MouseMoveEvent", 3)?;
		s.serialize_field("modifiers", &self.modifiers)?;
		s.serialize_field("x", &self.position.x)?;
		s.serialize_field("y", &self.position.y)?;
		s.end()
	}
}

#[cfg(feature = "serde")]
impl<'de> Deserialize<'de> for MouseMoveEvent {
	fn deserialize<D>(deserializer: D) -> Result<MouseMoveEvent, D::Error>
	where D: Deserializer<'de> {
		enum Field {
			Modifiers,
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
						formatter.write_str("`modifiers`, `x`, or `y`")
					}

					fn visit_str<E>(self, value: &str) -> Result<Field, E>
					where E: de::Error {
						match value {
							"modifiers" => Ok(Field::Modifiers),
							"x" => Ok(Field::X),
							"y" => Ok(Field::Y),
							_ => Err(de::Error::unknown_field(value, FIELDS)),
						}
					}
				}

				deserializer.deserialize_identifier(FieldVisitor)
			}
		}

		struct MouseMoveEventVisitor;

		impl<'de> Visitor<'de> for MouseMoveEventVisitor {
			type Value = MouseMoveEvent;

			fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
				formatter.write_str("struct MouseMoveEvent")
			}

			fn visit_seq<V>(self, mut seq: V) -> Result<MouseMoveEvent, V::Error>
			where V: SeqAccess<'de> {
				macro_rules! element {
					($name:ident, $val:expr) => {
						let $name = seq.next_element()?
							.ok_or_else(|| de::Error::invalid_length($val, &self))?;
					};
				}
				element!(modifiers, 0);
				element!(x, 1);
				element!(y, 2);

				let mut ret = MouseMoveEvent::new();
				ret.modifiers = modifiers;
				ret.position = Vector2::new(x, y);

				Ok(ret)
			}

			fn visit_map<V>(self, mut map: V) -> Result<MouseMoveEvent, V::Error>
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

				set!(modifiers, true);
				set!(x, true);
				set!(y, true);

				while let Some(key) = map.next_key()? {
					match key {
						Field::Modifiers => field!(modifiers),
						Field::X => field!(x),
						Field::Y => field!(y)
					}
				}

				set!(modifiers);
				set!(x);
				set!(y);

				let mut ret = MouseMoveEvent::new();
				ret.modifiers = modifiers;
				ret.position = Vector2::new(x, y);

				Ok(ret)
			}
		}

		const FIELDS: &'static [&'static str] = &["modifiers", "x", "y"];
		deserializer.deserialize_struct("MouseMoveEvent", FIELDS, MouseMoveEventVisitor)
	}
}
