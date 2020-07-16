macro_rules! bool_to_num {
	($x:expr) => (if $x { 1 } else { 0 });
}
macro_rules! buf {
	($size:expr) => {
		{
			let vec = vec![0; $size];
			vec.into_boxed_slice()
		}
	};
}
macro_rules! buf_to_str {
	($buf:expr) => {
		{
			let mut vec: Vec<u8> = vec![];
			for c in $buf.to_vec().iter() {
				if *c == 0 { break; }
				vec.push(*c as u8)
			}
			String::from_utf8(vec)?
		}
	}
}
#[cfg(feature = "input")]
macro_rules! buf_to_vec {
	($buf:expr, $len:expr) => {
		{
			let mut result = Vec::new();
			for i in 0usize..($len as usize) {
				let value = $buf[i];
				result.push(value);
			}
			result
		}
	}
}
/**
 Prints a colored message to the console.
 This has a side effect of flushing the console output.

 # Examples
 ```
 #[macro_use] extern crate winconsole;
 use winconsole::console::ConsoleColor;

 fn main() {
 	let thing = "world";
 	cprint!(ConsoleColor::Blue, "Hello, {}!", thing);
 	cprint!(ConsoleColor::Red, " Goodbye, world!");
 }
 ```

 # Panics
 Panics if foreground color cannot be retrieved/set, flushing console output fails,
 or if printing fails.
 */
#[macro_export]
macro_rules! cprint {
    ($color:expr, $($arg:tt)*) => {
		{
			use $crate::console;
			let old_color = console::get_foreground_color().unwrap();
			console::set_foreground_color($color).unwrap();
			print!($($arg)*);
			console::flush_output().unwrap();
			console::set_foreground_color(old_color).unwrap();
		}
	}
}
/**
 Prints a colored message to the console with a newline.
 This has a side effect of flushing the console output.

 # Examples
 ```
 #[macro_use] extern crate winconsole;
 use winconsole::console;
 use winconsole::console::ConsoleColor;

 fn main() {
 	let person = "Ada";
 	print!("Hello, ");
 	console::flush_output().unwrap();
 	cprintln!(ConsoleColor::Magenta, "{}.", person);
 	cprintln!(ConsoleColor::Blue, "How are you?");
 }
 ```

 # Panics
 Panics if foreground color cannot be retrieved/set, flushing console output fails,
 or if printing fails.
 */
#[macro_export]
macro_rules! cprintln {
    ($color: expr, $fmt:expr) => (cprint!($color, concat!($fmt, "\n")));
    ($color: expr, $fmt:expr, $($arg:tt)*) => (cprint!($color, concat!($fmt, "\n"), $($arg)*));
}
macro_rules! enumeration {
	(@inner $(#[$attrs:meta])*
	$name:ident<$repr_type:ty, $type:ty> ($sname:expr) {
		@$default:expr,
        $($(#[$item_attrs:meta])* $member:ident = $value:expr,)+
    }) => (
		use std::fmt;
		use std::fmt::{Display, Formatter};
		#[cfg(feature = "serde")]
		use serde::ser::{Serialize, Serializer};
		#[cfg(feature = "serde")]
		use serde::de::{self, Deserialize, Deserializer, Visitor};

		$(#[$attrs])*
		#[derive(Clone, Copy, Debug, PartialEq)]
		pub enum $name {
			$(
				$(#[$item_attrs])*
				$member = $value,
			)+
		}
		impl $name {
			#[doc = "Returns the integral value of the"]
			#[doc = $sname]
			#[doc = "."]
			pub fn get_value(&self) -> $repr_type {
				*self as $repr_type
			}
		}
		impl From<$type> for $name {
			fn from(value: $type) -> $name {
				match value {
					$(
						$value => $name::$member,
					)+
					_ => $name::from($default)
				}
			}
		}
		impl Into<$type> for $name {
			fn into(self) -> $type {
				self as $type
			}
		}
		impl Display for $name {
			fn fmt(&self, f: &mut Formatter) -> fmt::Result {
				let name = match *self {
					$(
						$name::$member => stringify!($member),
					)+
				};
				write!(f, "{}::{}", $sname, name)
			}
		}

		#[cfg(feature = "serde")]
		impl Serialize for $name {
            fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
            where S: Serializer {
				let num: $type = (*self).into();
                serializer.serialize_u64(num as u64)
            }
        }
		#[cfg(feature = "serde")]
        impl<'de> Deserialize<'de> for $name {
            fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
            where D: Deserializer<'de> {
                struct EnumVisitor;

                impl<'de> Visitor<'de> for EnumVisitor {
                    type Value = $name;

                    fn expecting(&self, formatter: &mut Formatter) -> fmt::Result {
                        formatter.write_str("positive integer")
                    }

                    fn visit_u64<E>(self, value: u64) -> Result<$name, E>
                    where E: de::Error {
						Ok($name::from(value as $type))
                    }
                }

                deserializer.deserialize_u64(EnumVisitor)
            }
        }
	);

	($(#[$attrs:meta])*
	$name:ident<$repr_type:ty, $type:ty> {
		__DEFAULT__ = $default:expr,
        $($(#[$item_attrs:meta])* $member:ident = $value:expr,)+
    }) => (enumeration! {
		@inner
		$(#[$attrs])*
		$name<$repr_type, $type> (stringify!($name)) {
			@$default,
			$($(#[$item_attrs])* $member = $value,)+
		}
	});

	($(#[$attrs:meta])*
	$name:ident<$type:ty> {
		__DEFAULT__ = $default:expr,
        $($(#[$item_attrs:meta])* $member:ident = $value:expr,)+
    }) => (enumeration! {
		@inner
		$(#[$attrs])*
		$name<$type, $type> (stringify!($name)) {
			@$default,
			$($(#[$item_attrs])* $member = $value,)+
		}
	})
}
macro_rules! flags {
	(@inner $(#[$attrs:meta])*
	$name:ident<$type:ty> ($sname:expr) {
        $($(#[$flag_attrs:meta])* $member:ident = $value:expr,)+
    }) => (
		use std::fmt;
		use std::fmt::{Display, Formatter};
		#[cfg(feature = "serde")]
		use serde::ser::{Serialize, Serializer};
		#[cfg(feature = "serde")]
		use serde::de::{self, Deserialize, Deserializer, Visitor};

        $(#[$attrs])*
		#[derive(Clone, Copy, Debug, PartialEq)]
		#[allow(non_snake_case)]
        pub struct $name {
			$(
				$(#[$flag_attrs])*
				pub $member: bool,
			)+
		}

		impl $name {
			#[doc = "Creates a new"]
			#[doc = $sname]
			#[doc = "object with all fields set to false."]
			pub fn new() -> $name {
				$name {
					$($member: false,)+
				}
			}
		}
		impl From<$type> for $name {
			fn from(value: $type) -> $name {
				let mut flags = $name::new();
				$(flags.$member = value & $value != 0;)+
				flags
			}
		}
		impl Into<$type> for $name {
			fn into(self) -> $type {
				let mut value: $type = 0;
				$(if self.$member { value |= $value; })+
				value
			}
		}
		impl Display for $name {
			fn fmt(&self, f: &mut Formatter) -> fmt::Result {
				let mut ret = String::new();
				$(
					if self.$member {
						if ret != "" { ret.push_str(" | "); }
						ret.push_str(stringify!($member));
					}
				)+
				write!(f, "{}({})", $sname, &ret)
			}
		}

		#[cfg(feature = "serde")]
		impl Serialize for $name {
            fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
            where S: Serializer {
				let num: $type = (*self).into();
                serializer.serialize_u64(num as u64)
            }
        }
		#[cfg(feature = "serde")]
        impl<'de> Deserialize<'de> for $name {
            fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
            where D: Deserializer<'de> {
                struct FlagVisitor;

                impl<'de> Visitor<'de> for FlagVisitor {
                    type Value = $name;

                    fn expecting(&self, formatter: &mut Formatter) -> fmt::Result {
                        formatter.write_str("positive integer")
                    }

                    fn visit_u64<E>(self, value: u64) -> Result<$name, E>
                    where E: de::Error {
						Ok($name::from(value as $type))
                    }
                }

                deserializer.deserialize_u64(FlagVisitor)
            }
        }
	);

	($(#[$attrs:meta])*
	$name:ident<$type:ty> {
        $($(#[$flag_attrs:meta])* $member:ident = $value:expr,)+
    }) => (flags! {
		@inner
		$(#[$attrs])*
		$name<$type> (stringify!($name)) {
			$($(#[$flag_attrs])* $member = $value,)+
		}
	});
}
macro_rules! handle {
	($x:expr) => {
		{
			let handle = processenv::GetStdHandle($x);
			if handle as isize == -1 { throw_err!($crate::errors::InvalidHandleError::new()); }
			handle
		}
	};
}
macro_rules! make_colorref {
	($x:expr) => ($x.r as u32 | (($x.g as u32) << 8) | (($x.b as u32) << 16));
}
macro_rules! make_rgb {
	($x:expr) => {
		RGB8 {
			r: ($x & 0x0000ff) as u8,
			g: (($x >> 8) & 0x00ff) as u8,
			b: (($x >> 16) & 0xff) as u8
		}
	}
}
macro_rules! os_err {
	() => (
		{
			use std::io;
			use $crate::errors::*;
			let last_err = io::Error::last_os_error();
			let err = match last_err.raw_os_error().unwrap() {
				6 => WinError::from(InvalidHandleError::new()),
				_ => WinError::from(last_err)
			};
			Err(err)
		}
	);
	($i:expr) => {
		if $i == 0 {
			return os_err!();
		}
	};
	($i:expr, $x:expr) => {
		if $x {
			use std::io;
			let err = io::Error::last_os_error();
			if err.raw_os_error().unwrap() != 0 {
				os_err!($i);
			}
		} else {
			os_err!($i);
		}
	}
}
macro_rules! str_to_buf {
	(@inner $s:expr, $type:ty) => {
		{
			let vec: Vec<$type> = String::from($s)
				.as_bytes()
				.iter()
				.map(|c| *c as $type)
				.collect();
			vec.into_boxed_slice()
		}
	};
	(@inner $s:expr, $size:expr, $type:ty) => {
		{
			let mut buffer: [$type; $size] = [0; $size];
			for (chr, val) in $s.as_bytes().iter().zip(buffer.iter_mut()) {
				*val = *chr as $type;
			}
			buffer
		}
	};

	($s:expr) => (str_to_buf!(@inner $s, CHAR));
	($s:expr, $size:expr) => (str_to_buf!(@inner $s, $size, CHAR));
}
macro_rules! str_to_buf_w {
	($s:expr) => (str_to_buf!(@inner $s, WCHAR));
	($s:expr, $size:expr) => (str_to_buf!(@inner $s, $size, WCHAR));
}
macro_rules! throw_err {
	($err:expr) => {
		Err($crate::errors::WinError::from($err))?;
	}
}
#[cfg(feature = "window")]
macro_rules! window_handle {
	() => (wincon::GetConsoleWindow());
}
