use std::{mem, ptr};

use winapi::shared::windef::{HWND, POINT, RECT};
use winapi::um::{wincon, winuser};
use winapi::um::winuser::{
	FLASHWINFO,
	WINDOWPLACEMENT
};

use super::errors::*;

mod window_main;
mod etc;

pub use self::window_main::*;
pub use self::etc::*;
