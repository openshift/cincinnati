use super::*;
use std::io::Error as IoError;
use std::string::{FromUtf8Error, FromUtf16Error};

macro_rules! win_errs {
	($($(#[$item_attrs:meta])* $name:ident : $err_name:ident),*) => (
		use std::{
			error::Error,
			fmt::{Display, Formatter, Result}
		};

		/// Contains wrapped error types.
		#[derive(Debug)]
		pub enum WinError {
			$(
				$(#[$item_attrs])*
				$name($err_name),
			)*
		}

		impl Error for WinError {
			fn description(&self) -> &str {
				match self {
					$(
						&WinError::$name(ref err) => Error::description(err as &Error),
					)*
				}
			}
			fn cause(&self) -> Option<&Error> {
				match self {
					$(
						&WinError::$name(ref err) => Some(err as &Error),
					)*
				}
			}
		}
		impl Display for WinError {
			fn fmt(&self, f: &mut Formatter) -> Result {
				match self {
					$(
						&WinError::$name(ref err) => Display::fmt(&err, f),
					)*
				}
			}
		}

		$(
			impl From<$err_name> for WinError {
				fn from(value: $err_name) -> WinError {
					WinError::$name(value)
				}
			}
		)*
	);
}

win_errs! {
	/// An argument error.
	Argument: ArgumentError,
	/// An error which occurred while converting to a string from a
	/// UTF-8 byte vector.
	FromUtf8: FromUtf8Error,
	/// An error which occurred while converting to a string from a
	/// UTF-16 byte vector.
	FromUtf16: FromUtf16Error,
	/// An invalid handle error.
	InvalidHandle: InvalidHandleError,
	/// An IO or OS error.
	Io: IoError
}
