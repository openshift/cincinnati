use crate::fnoption::FnOption;
use crate::hide::{hide_mem, hide_ptr};

/// Calls a closure and overwrites its stack on return.
///
/// This function calls `clear_stack` after calling the passed closure,
/// taking care to prevent either of them being inlined, so the stack
/// used by the closure will be overwritten with zeros (as long as a
/// large enough number of `pages` is used).
///
/// For technical reasons, this function can be used only with `Fn` or
/// `FnMut`. If all you have is a `FnOnce`, use the auxiliary function
/// `clear_stack_on_return_fnonce` instead.
///
/// # Example
///
/// ```
/// # use clear_on_drop::clear_stack_on_return;
/// # fn encrypt(input: &[u8]) -> Vec<u8> { input.to_owned() }
/// let input = b"abc";
/// let result = clear_stack_on_return(1, || encrypt(input));
/// ```
#[inline]
pub fn clear_stack_on_return<F, R>(pages: usize, mut f: F) -> R
where
    F: FnMut() -> R,
{
    let _clear = ClearStackOnDrop { pages };
    // Do not inline f to make sure clear_stack uses the same stack space.
    hide_ptr::<&mut dyn FnMut() -> R>(&mut f)()
}

/// Calls a closure and overwrites its stack on return.
///
/// This function is a variant of `clear_stack_on_return` which also
/// accepts `FnOnce`, at the cost of being slightly slower.
///
/// # Example
///
/// ```
/// # use clear_on_drop::clear_stack_on_return_fnonce;
/// # fn encrypt(input: Vec<u8>) -> Vec<u8> { input }
/// let input = vec![97, 98, 99];
/// let result = clear_stack_on_return_fnonce(1, || encrypt(input));
/// ```
#[inline]
pub fn clear_stack_on_return_fnonce<F, R>(pages: usize, f: F) -> R
where
    F: FnOnce() -> R,
{
    let mut f = FnOption::new(f);
    clear_stack_on_return(pages, || f.call_mut()).unwrap()
}

struct ClearStackOnDrop {
    pages: usize,
}

impl Drop for ClearStackOnDrop {
    #[inline]
    fn drop(&mut self) {
        // Do not inline clear_stack.
        hide_ptr::<fn(usize)>(clear_stack)(self.pages);
    }
}

/// Overwrites a few pages of stack.
///
/// This function will overwrite `pages` 4096-byte blocks of the stack
/// with zeros.
pub fn clear_stack(pages: usize) {
    if pages > 0 {
        let mut buf = [0u8; 4096];
        hide_mem(&mut buf); // prevent moving after recursive call
        clear_stack(pages - 1);
        hide_mem(&mut buf); // prevent reuse of stack space for call
    }
}
