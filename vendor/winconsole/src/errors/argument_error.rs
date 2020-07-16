/// Describes an error related to an argument.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct ArgumentError {
	/// The name of the offending argument.
	pub argument: String,
	/// A message describing the error.
	pub message: String
}

impl ArgumentError {
	/**
	 Creates a new ArgumentError.

	 # Arguments
	 * `argument` - The name of the offending argument.
	 * `message` - A message describing the error.
	 */
	pub fn new(argument: impl Into<String>, message: impl Into<String>) -> ArgumentError {
		ArgumentError {
			argument: argument.into(),
			message: message.into()
		}
	}
}

impl_err!(ArgumentError, "invalid argument",
	"argument {} is invalid: {}", argument, message);
