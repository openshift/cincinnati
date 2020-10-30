use super::*;

#[cfg(feature = "serde")]
use serde::ser::{Serialize, Serializer, SerializeStruct};
#[cfg(feature = "serde")]
use serde::de::{self, Deserialize, Deserializer, Visitor, SeqAccess, MapAccess};
#[cfg(feature = "serde")]
use std::fmt;

/// Represents an input event which occurred as a result of a mouse scroll wheel.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MouseWheelEvent {
	/// The direction and value of the scroll.
	pub delta: i16,
	/// Did the scroll event occur on a horizontal scroll wheel?
	pub horizontal: bool,
	/// A ControlKeyState object describing the state of control keys.
	pub modifiers: ControlKeyState,
	/// The character cell the event occurred on.
	pub position: Vector2<u16>
}

impl MouseWheelEvent {
	/**
	 Returns an empty MouseWheelEvent.
	 */
	pub fn new() -> MouseWheelEvent {
		MouseWheelEvent {
			delta: 0,
			horizontal: false,
			modifiers: ControlKeyState::new(),
			position: Vector2::new(0, 0)
		}
	}
}

impl Into<InputEvent> for MouseWheelEvent {
	fn into(self) -> InputEvent {
		InputEvent::MouseWheel(self)
	}
}

#[cfg(feature = "serde")]
impl Serialize for MouseWheelEvent {
	fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
	where S: Serializer {
		let mut s = serializer.serialize_struct("MouseWheelEvent", 5)?;
		s.serialize_field("delta", &self.delta)?;
		s.serialize_field("horizontal", &self.horizontal)?;
		s.serialize_field("modifiers", &self.modifiers)?;
		s.serialize_field("x", &self.position.x)?;
		s.serialize_field("y", &self.position.y)?;
		s.end()
	}
}

#[cfg(feature = "serde")]
impl<'de> Deserialize<'de> for MouseWheelEvent {
	fn deserialize<D>(deserializer: D) -> Result<MouseWheelEvent, D::Error>
	where D: Deserializer<'de> {
		enum Field {
			Delta,
			Horizontal,
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
						formatter.write_str("`delta`, `horizontal`, `modifiers`, `x`, or `y`")
					}

					fn visit_str<E>(self, value: &str) -> Result<Field, E>
					where E: de::Error {
						match value {
							"delta" => Ok(Field::Delta),
							"horizontal" => Ok(Field::Horizontal),
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

		struct MouseWheelEventVisitor;

		impl<'de> Visitor<'de> for MouseWheelEventVisitor {
			type Value = MouseWheelEvent;

			fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
				formatter.write_str("struct MouseWheelEvent")
			}

			fn visit_seq<V>(self, mut seq: V) -> Result<MouseWheelEvent, V::Error>
			where V: SeqAccess<'de> {
				macro_rules! element {
					($name:ident, $val:expr) => {
						let $name = seq.next_element()?
							.ok_or_else(|| de::Error::invalid_length($val, &self))?;
					};
				}
				element!(delta, 0);
				element!(horizontal, 1);
				element!(modifiers, 2);
				element!(x, 3);
				element!(y, 4);

				let mut ret = MouseWheelEvent::new();
				ret.delta = delta;
				ret.horizontal = horizontal;
				ret.modifiers = modifiers;
				ret.position = Vector2::new(x, y);

				Ok(ret)
			}

			fn visit_map<V>(self, mut map: V) -> Result<MouseWheelEvent, V::Error>
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

				set!(delta, true);
				set!(horizontal, true);
				set!(modifiers, true);
				set!(x, true);
				set!(y, true);

				while let Some(key) = map.next_key()? {
					match key {
						Field::Delta => field!(delta),
						Field::Horizontal => field!(horizontal),
						Field::Modifiers => field!(modifiers),
						Field::X => field!(x),
						Field::Y => field!(y)
					}
				}

				set!(delta);
				set!(horizontal);
				set!(modifiers);
				set!(x);
				set!(y);

				let mut ret = MouseWheelEvent::new();
				ret.delta = delta;
				ret.horizontal = horizontal;
				ret.modifiers = modifiers;
				ret.position = Vector2::new(x, y);

				Ok(ret)
			}
		}

		const FIELDS: &'static [&'static str] = &["delta", "horizontal", "modifiers", "x", "y"];
		deserializer.deserialize_struct("MouseWheelEvent", FIELDS, MouseWheelEventVisitor)
	}
}
