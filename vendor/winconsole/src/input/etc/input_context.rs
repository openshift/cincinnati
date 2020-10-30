use super::*;
use ::input;

/// Used to obtain input events.
pub struct InputContext {
	/// Should repeated events be sent?
	pub repeat_enabled: bool,
	/// Should the context restore the original input mode when it is dropped?
	pub restore_on_drop: bool,

	pub(crate) button_status: [bool; 5],
	pub(crate) held_keys: Vec<KeyCode>,

	filter: InputFilter,
	filter_value: u16,
	original_mode: InputSettings,
	queue: Vec<InputEvent>
}

impl InputContext {
	/**
	 Clears the context's input event queue.

	 # Examples
	 ```
	 # extern crate winconsole;
	 # use winconsole::input;
	 # fn main() {
	 let mut ctx = input::start().unwrap();
	 ctx.flush();
	 # }
	 ```
	 */
	pub fn flush(&mut self) {
		self.queue.clear();
	}
	/**
	 Returns all of the input events which are currently in the queue.

	 # Examples
	 ```
	 # extern crate winconsole;
	 # use winconsole::input;
	 # fn main() {
	 let mut ctx = input::start().unwrap();
	 loop {
	 	let events = ctx.get().unwrap();
	 	for event in events {
	 		println!("{}", event);
	 	}
	 }
	 # }
	 ```

	 # Errors
 	 * [`InvalidHandleError`]: Returned if an invalid handle to the console input is retrieved or used.
 	 * [`IoError`]: Returned if an OS error occurs.

 	 [`InvalidHandleError`]: ../errors/enum.WinError.html#InvalidHandle.v
 	 [`IoError`]: ../errors/enum.WinError.html#Io.v
	 */
	pub fn get(&mut self) -> WinResult<Vec<InputEvent>> {
		self.collect(false, false, 1000)?;
		let events = self.queue.clone();
		self.queue.clear();
		Ok(events)
	}
	/**
	 Returns the current input filter.

	 # Examples
	 ```
	 # extern crate winconsole;
	 # use winconsole::input;
	 # fn main() {
	 let mut ctx = input::start().unwrap();
	 let filter = ctx.get_filter();
	 println!("MouseDown events filtered? {}", filter.MouseDown);
	 # }
	 ```
	 */
	pub fn get_filter(&self) -> InputFilter {
		self.filter
	}
	/**
	 Reads data from the input queue without discarding it.

	 # Arguments
	 * `max_length` - The maximum amount of input events to return.

	 # Examples
	 ```
	 # extern crate winconsole;
	 # use winconsole::input;
	 # fn main() {
	 let mut ctx = input::start().unwrap();
	 let peeked = ctx.peek(5).unwrap();
	 println!("Peeked: {}", peeked.len());
	 for event in peeked.iter() {
	 	println!("{}", event);
	 }
	 # }
	 ```
	
	 # Errors
 	 * [`InvalidHandleError`]: Returned if an invalid handle to the console output is retrieved or used.
 	 * [`IoError`]: Returned if an OS error occurs.

 	 [`IoError`]: ../errors/enum.WinError.html#Io.v
 	 [`InvalidHandleError`]: ../errors/enum.WinError.html#InvalidHandle.v
	 */
	pub fn peek(&mut self, max_length: u32) -> WinResult<Vec<InputEvent>> {
		let filter = self.filter_value;
		let ret = self.collect(false, true, max_length)?
			.into_iter()
			.filter(|event| filter & event.get_type() == 0)
			.collect();
		Ok(ret)
	}
	/**
	 Returns a single input event, or InputEvent::None if none are available.

	 # Examples
	 ```
	 # extern crate winconsole;
	 # use winconsole::input;
	 # use winconsole::input::InputEvent;
	 # fn main() {
	 let mut ctx = input::start().unwrap();
	 loop {
	 	let event = ctx.poll().unwrap();
	 	if event != InputEvent::None {
	 		println!("{}", event);
	 	}
	 }
	 # }
	 ```
	
	 # Errors
 	 * [`InvalidHandleError`]: Returned if an invalid handle to the console output is retrieved or used.
 	 * [`IoError`]: Returned if an OS error occurs.

 	 [`InvalidHandleError`]: ../errors/enum.WinError.html#InvalidHandle.v
 	 [`IoError`]: ../errors/enum.WinError.html#Io.v
	 */
	pub fn poll(&mut self) -> WinResult<InputEvent> {
		if self.queue.len() == 0 {
			self.collect(false, false, 1000)?;
			if self.queue.len() == 0 { return Ok(InputEvent::None); }
		}
		Ok(self.queue.remove(0))
	}
	/**
	 Resets the internal state of the context, clearing data about which keys and buttons are
	 currently held along with the event queue.

	 # Examples
	 ```
	 # extern crate winconsole;
	 # use winconsole::input;
	 # fn main() {
	 let mut ctx = input::start().unwrap();
	 ctx.wait().unwrap();
	 ctx.reset();
	 let event = ctx.wait().unwrap();
	 println!("{}", event);
	 # }
	 ```
	 */
	pub fn reset(&mut self) {
		self.held_keys.clear();
		self.queue.clear();
		for i in 0..5 {
			self.button_status[i] = console::get_key_state(BUTTON_VIRTUAL[i] as u32);
		}
	}
	/**
	 Sets InputEvent types which should not be returned from methods.

	 # Arguments
	 * `filter` - The InputFilter to apply.

	 # Examples
	 ```
	 # extern crate winconsole;
	 # use winconsole::input;
	 # use winconsole::input::InputFilter;
	 # fn main() {
	 let mut ctx = input::start().unwrap();
	 let mut filter = InputFilter::new();
	 filter.MouseDown = true;
	 ctx.set_filter(filter);
	 # }
	 ```
	 */
	pub fn set_filter(&mut self, filter: InputFilter) {
		self.filter = filter;

		let filter: u16 = filter.into();
		self.filter_value = filter;
		self.queue = self.queue.iter()
			.cloned()
			.filter(|event| filter & event.get_type() == 0)
			.collect();
	}
	/**
	 Adds an input event to the input queue.

	 # Arguments
	 * `event` - The InputEvent to add.

	 # Examples
	 ```
	 # extern crate winconsole;
	 # use winconsole::input;
	 # use winconsole::input::{InputEvent, FocusEvent};
	 # fn main() {
	 let mut ctx = input::start().unwrap();
	 let mut focus_event = FocusEvent::new();
	 focus_event.focused = true;
	 ctx.simulate(focus_event);

	 let event = ctx.wait().unwrap();
	 println!("{}", event);
	 # }
	 ```
	 */
	pub fn simulate(&mut self, event: impl Into<InputEvent>) {
		self.push(event.into());
	}
	/**
	 Waits until an input event is available and returns it.

	 # Examples
	 ```
	 # extern crate winconsole;
	 # use winconsole::input;
	 # fn main() {
	 let mut ctx = input::start().unwrap();
	 let event = ctx.wait().unwrap();
	 println!("{}", event);
	 # }
	 ```
	
	 # Errors
 	 * [`InvalidHandleError`]: Returned if an invalid handle to the console output is retrieved or used.
 	 * [`IoError`]: Returned if an OS error occurs.

 	 [`InvalidHandleError`]: ../errors/enum.WinError.html#InvalidHandle.v
 	 [`IoError`]: ../errors/enum.WinError.html#Io.v
	 */
	pub fn wait(&mut self) -> WinResult<InputEvent> {
		if self.queue.len() == 0 {
			self.collect(true, false, 1000)?;
			if self.queue.len() == 0 { return Ok(InputEvent::None); }
		}

		Ok(self.queue.remove(0))
	}

	pub(crate) fn new(original_mode: InputSettings) -> InputContext {
		InputContext {
			original_mode,
			button_status: [false; 5],
			repeat_enabled: true,
			restore_on_drop: true,
			held_keys: Vec::new(),
			queue: Vec::new(),
			filter: InputFilter::from(1),
			filter_value: 1
		}
	}

	fn collect(&mut self, wait: bool, peek: bool, max_length: u32) -> WinResult<Vec<InputEvent>> {
		if !wait && console::num_input_events()? == 0 { return Ok(Vec::new()); }

		let records = if peek {
			console::peek_input(max_length as usize)?
		} else {
			console::read_input(max_length as usize)?
		};

		let events = input::convert_events(&records, self);
		if peek { return Ok(events); }

		for event in events {
			self.push(event);
		}
		Ok(Vec::new())
	}
	fn push(&mut self, event: InputEvent) {
		if self.filter_value & event.get_type() == 0 {
			self.queue.push(event);
		}
	}
}

impl Drop for InputContext {
	fn drop(&mut self) {
		if self.restore_on_drop {
			console::set_input_mode(self.original_mode).unwrap_or(())
		}
	}
}
