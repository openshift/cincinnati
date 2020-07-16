/// Describes an error related to an invalid handle.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct InvalidHandleError;

impl InvalidHandleError {
	/**
	 Creates a new InvalidHandleError.
	 */
	pub fn new() -> InvalidHandleError {
		InvalidHandleError {}
	}
}

impl_err!(InvalidHandleError, "invalid handle", "attempt to use an invalid handle");
