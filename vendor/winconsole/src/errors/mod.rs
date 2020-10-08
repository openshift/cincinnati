macro_rules! impl_err {
	($name:ident, $desc:expr, $fmt:expr) => {
		impl_err!($name, $desc, $fmt,);
	};
	($name:ident, $desc:expr, $fmt:expr, $($arg:ident),*) => (
		use std::{error, fmt};
		impl error::Error for $name {
			fn description(&self) -> &str {
				$desc
			}
		}
		impl fmt::Display for $name {
			fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
				write!(f, $fmt, $(&self.$arg,)*)
			}
		}
	);
}

mod argument_error;
mod invalid_handle_error;
mod win_error;

pub use self::argument_error::ArgumentError;
pub use self::invalid_handle_error::InvalidHandleError;
pub use self::win_error::WinError;

/// Represents a result which contains either a returned value or a WinError.
pub type WinResult<T> = Result<T, WinError>;
