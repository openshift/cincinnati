use cgmath::Vector2;

/// Defines the coordinates of the corners of a rectangle.
#[derive(Clone, Copy, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct Rect {
	/// The top of the rectangle.
	pub top: u16,
	/// The bottom of the rectangle.
	pub bottom: u16,
	/// The left of the rectangle.
	pub left: u16,
	/// The right of the rectangle.
	pub right: u16
}

impl Rect {
	/**
	 Creates a new Rect.
	 */
	pub fn new(top: u16, left: u16, right: u16, bottom: u16) -> Rect {
		Rect {
			top,
			bottom,
			left,
			right
		}
	}

	/**
	 Returns a Vector representing the bottom-left corner of the rectangle.

	 # Examples
	 ```
	 # extern crate winconsole;
	 # extern crate cgmath;
	 # use winconsole::console::Rect;
	 # use cgmath::Vector2;
	 # fn main() {
	 let rect = Rect::new(0, 10, 20, 30);
	 let bottom_left = rect.bottom_left();
	 assert_eq!(bottom_left, Vector2::new(10, 30))
	 # }
	 ```
	 */
	pub fn bottom_left(&self) -> Vector2<u16> {
		Vector2::new(self.left, self.bottom)
	}
	/**
	 Returns a Vector representing the bottom-right corner of the rectangle.

	 # Examples
	 ```
	 # extern crate winconsole;
	 # extern crate cgmath;
	 # use winconsole::console::Rect;
	 # use cgmath::Vector2;
	 # fn main() {
	 let rect = Rect::new(0, 10, 20, 30);
	 let bottom_right = rect.bottom_right();
	 assert_eq!(bottom_right, Vector2::new(20, 30))
	 # }
	 ```
	 */
	pub fn bottom_right(&self) -> Vector2<u16> {
		Vector2::new(self.right, self.bottom)
	}
	/**
	 Returns a Vector representing the top-left corner of the rectangle.

	 # Examples
	 ```
	 # extern crate winconsole;
	 # extern crate cgmath;
	 # use winconsole::console::Rect;
	 # use cgmath::Vector2;
	 # fn main() {
	 let rect = Rect::new(0, 10, 20, 30);
	 let top_left = rect.top_left();
	 assert_eq!(top_left, Vector2::new(10, 0));
	 # }
	 ```
	 */
	pub fn top_left(&self) -> Vector2<u16> {
		Vector2::new(self.left, self.top)
	}
	/**
	 Returns a Vector representing the top-right corner of the rectangle.

	 # Examples
	 ```
	 # extern crate winconsole;
	 # extern crate cgmath;
	 # use winconsole::console::Rect;
	 # use cgmath::Vector2;
	 # fn main() {
	 let rect = Rect::new(0, 10, 20, 30);
	 let top_right = rect.top_right();
	 assert_eq!(top_right, Vector2::new(20, 0));
	 # }
	 ```
	 */
	pub fn top_right(&self) -> Vector2<u16> {
		Vector2::new(self.right, self.top)
	}
}
