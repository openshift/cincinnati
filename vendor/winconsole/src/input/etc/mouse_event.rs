use super::*;

#[cfg(feature = "serde")]
use serde::ser::{Serialize, Serializer, SerializeStruct};
#[cfg(feature = "serde")]
use serde::de::{self, Deserialize, Deserializer, Visitor, SeqAccess, MapAccess};
#[cfg(feature = "serde")]
use std::fmt;

/// Represents an input event which occurred on a mouse button.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MouseEvent {
	/// The mouse button the event occurred on.
	pub button: u8,
	/// The KeyCode of the mouse button which the event occurred on.
	pub key_code: KeyCode,
	/// A ControlKeyState object describing the state of control keys.
	pub modifiers: ControlKeyState,
	/// The character cell the event occurred on.
	pub position: Vector2<u16>,
	/// Is the mouse button pressed?
	pub pressed: bool
}

impl MouseEvent {
	/**
	 Returns an empty MouseEvent.
	 */
	pub fn new() -> MouseEvent {
		MouseEvent {
			button: 0,
			key_code: KeyCode::None,
			modifiers: ControlKeyState::new(),
			position: Vector2::new(0, 0),
			pressed: false
		}
	}
}

impl Into<InputEvent> for MouseEvent {
	fn into(self) -> InputEvent {
		if self.pressed {
			InputEvent::MouseDown(self)
		} else {
			InputEvent::MouseUp(self)
		}
	}
}

#[cfg(feature = "serde")]
impl Serialize for MouseEvent {
	fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
	where S: Serializer {
		let mut s = serializer.serialize_struct("MouseEvent", 6)?;
		s.serialize_field("button", &self.button)?;
		s.serialize_field("key_code", &self.key_code)?;
		s.serialize_field("modifiers", &self.modifiers)?;
		s.serialize_field("x", &self.position.x)?;
		s.serialize_field("y", &self.position.y)?;
		s.serialize_field("pressed", &self.pressed)?;
		s.end() 
	}
}

#[cfg(feature = "serde")]
impl<'de> Deserialize<'de> for MouseEvent {
	fn deserialize<D>(deserializer: D) -> Result<MouseEvent, D::Error>
	where D: Deserializer<'de> {
		enum Field {
			Button,
			KeyCode,
			Modifiers,
			X,
			Y,
			Pressed
		};

		impl<'de> Deserialize<'de> for Field {
			fn deserialize<D>(deserializer: D) -> Result<Field, D::Error>
			where D: Deserializer<'de> {
				struct FieldVisitor;

				impl<'de> Visitor<'de> for FieldVisitor {
					type Value = Field;

					fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
						formatter.write_str(
							"`button`, `key_code`, `modifiers`, `x`, `y`, or `pressed`"
						)
					}

					fn visit_str<E>(self, value: &str) -> Result<Field, E>
					where E: de::Error {
						match value {
							"button" => Ok(Field::Button),
							"key_code" => Ok(Field::KeyCode),
							"modifiers" => Ok(Field::Modifiers),
							"x" => Ok(Field::X),
							"y" => Ok(Field::Y),
							"pressed" => Ok(Field::Pressed),
							_ => Err(de::Error::unknown_field(value, FIELDS)),
						}
					}
				}

				deserializer.deserialize_identifier(FieldVisitor)
			}
		}

		struct MouseEventVisitor;

		impl<'de> Visitor<'de> for MouseEventVisitor {
			type Value = MouseEvent;

			fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
				formatter.write_str("struct MouseEvent")
			}

			fn visit_seq<V>(self, mut seq: V) -> Result<MouseEvent, V::Error>
			where V: SeqAccess<'de> {
				macro_rules! element {
					($name:ident, $val:expr) => {
						let $name = seq.next_element()?
							.ok_or_else(|| de::Error::invalid_length($val, &self))?;
					};
				}
				element!(button, 0);
				element!(key_code, 1);
				element!(modifiers, 2);
				element!(x, 3);
				element!(y, 4);
				element!(pressed, 5);

				let mut ret = MouseEvent::new();
				ret.button = button;
				ret.key_code = key_code;
				ret.modifiers = modifiers;
				ret.position = Vector2::new(x, y);
				ret.pressed = pressed;

				Ok(ret)
			}

			fn visit_map<V>(self, mut map: V) -> Result<MouseEvent, V::Error>
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

				set!(button, true);
				set!(key_code, true);
				set!(modifiers, true);
				set!(x, true);
				set!(y, true);
				set!(pressed, true);

				while let Some(key) = map.next_key()? {
					match key {
						Field::Button => field!(button),
						Field::KeyCode => field!(key_code),
						Field::Modifiers => field!(modifiers),
						Field::X => field!(x),
						Field::Y => field!(y),
						Field::Pressed => field!(pressed)
					}
				}

				set!(button);
				set!(key_code);
				set!(modifiers);
				set!(x);
				set!(y);
				set!(pressed);

				let mut ret = MouseEvent::new();
				ret.button = button;
				ret.key_code = key_code;
				ret.modifiers = modifiers;
				ret.position = Vector2::new(x, y);
				ret.pressed = pressed;

				Ok(ret)
			}
		}

		const FIELDS: &'static [&'static str] = &[
			"button", "key_code", "modifiers",
			"x", "y", "pressed"
		];
		deserializer.deserialize_struct("MouseEvent", FIELDS, MouseEventVisitor)
	}
}
