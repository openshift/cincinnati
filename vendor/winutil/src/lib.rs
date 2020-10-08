extern crate winapi;

use winapi::um::wow64apiset::IsWow64Process;
use winapi::um::processthreadsapi::GetCurrentProcess;
use winapi::um::winbase::{GetComputerNameA, GetUserNameA};
use winapi::um::winnt::CHAR;

#[cfg(test)]
mod test {
    use super::{is_wow64_process, get_computer_name, get_user_name};

    #[test]
    fn is_wow64_process_returns_bool() {
        match is_wow64_process() {
            Ok(true) => assert!(true),
            Ok(false) => assert!(true),
            _ => assert!(false),
        }
    }

    #[test]
    fn get_computer_name_returns_option() {
        match get_computer_name() {
            Some(_) => assert!(true),
            None => assert!(false),
        };
    }

    #[test]
    fn get_user_name_returns_option() {
        match get_user_name() {
            Some(_) => assert!(true),
            None => assert!(false),
        }
    }
}

#[derive(Debug)]
pub enum WinUtilError<'a> {
    IsWow64ProcessError(&'a str),
}

/// Detect if the current process is running under the WoW64 subsystem.
///
/// Possible results include:
///     64-bit binary on 64-bit OS -> Ok(false)
///     32-bit binary on 32-bit OS -> Ok(false)
///     32-bit binary on 64-bit OS -> Ok(true)
///
pub fn is_wow64_process<'a>() -> Result<bool, WinUtilError<'a>> {
    let mut is_wow = 0;
    let is_wow_ptr = &mut is_wow as *mut i32;

    unsafe {
        match IsWow64Process(GetCurrentProcess(), is_wow_ptr) {
            0 => Err(WinUtilError::IsWow64ProcessError("returned 0")),
            _ => {
                match *is_wow_ptr {
                    0 => Ok(false),
                    1 => Ok(true),
                    _ => unreachable!(),
                }
            }
        }
    }
}

/// Return an Option containing the NetBIOS name of the local computer.
///
pub fn get_computer_name() -> Option<String> {
    const MAX_COMPUTERNAME_LENGTH: usize = 15;

    let mut buf = [0 as CHAR; MAX_COMPUTERNAME_LENGTH + 1];
    let mut len = buf.len() as u32;

    unsafe {
        if GetComputerNameA(buf.as_mut_ptr(), &mut len) == 0 {
            return None;
        };
    }

    let host: Vec<u8> = buf[0..len as usize]
                            .iter()
                            .map(|&e| e as u8)
                            .collect();

    match String::from_utf8(host) {
        Ok(h) => return Some(h),
        Err(_) => return None,
    };
}

/// Return an Option containing the user associated with the current process.
///
pub fn get_user_name() -> Option<String> {
    const UNLEN: usize = 256;

    let mut buf = [0 as CHAR; UNLEN + 1];
    let mut len = buf.len() as u32;

    unsafe {
        if GetUserNameA(buf.as_mut_ptr(), &mut len) == 0 {
            return None;
        };
    }

    let user: Vec<u8> = buf[0..(len - 1) as usize]
                            .iter()
                            .map(|&e| e as u8)
                            .collect();

    match String::from_utf8(user) {
        Ok(u) => return Some(u),
        Err(_) => return None,
    };
}
