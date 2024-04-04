//! Wrap a `FnOnce` into something which behaves like a `FnMut`.
//!
//! To prevent inlining, `clear_stack_on_return` has to hide the passed
//! closure behind a borrow. However, it's not possible to move out of
//! a borrow, so `FnOnce` can't be used.
//!
//! The `FnOption` wraps the `FnOnce` with an `Option`, which can be
//! moved out of.

/// Wraps a `FnOnce` with an `Option`.
pub struct FnOption<R, F: FnOnce() -> R> {
    f: Option<F>,
}

impl<R, F> FnOption<R, F>
where
    F: FnOnce() -> R,
{
    /// Wraps a `FnOnce` with an `Option`.
    #[inline]
    pub fn new(f: F) -> Self {
        FnOption { f: Some(f) }
    }

    /// Calls the `FnOnce`. This function should be called only once.
    /// It will always return `None` after the first time.
    #[inline]
    pub fn call_mut(&mut self) -> Option<R> {
        self.f.take().map(|f| f())
    }
}
