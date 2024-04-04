//! Prevent agressive code removal optimizations.
//!
//! The functions in this module "hide" a variable from the optimizer,
//! so that it believes the variable has been read and/or modified in
//! unpredictable ways, while in fact nothing happened.
//!
//! Inspired by/based on Linux kernel's `OPTIMIZER_HIDE_VAR`, which in
//! turn was based on the earlier `RELOC_HIDE` macro.

/// Make the optimizer believe the memory pointed to by `ptr` is read
/// and modified arbitrarily.
#[inline]
pub fn hide_mem<T: ?Sized>(ptr: &mut T) {
    hide_mem_impl(ptr);
}

/// Make the optimizer believe the pointer returned by this function is
/// possibly unrelated (except for the lifetime) to `ptr`.
#[inline]
pub fn hide_ptr<P>(mut ptr: P) -> P {
    hide_mem::<P>(&mut ptr);
    ptr
}

pub use self::impls::hide_mem_impl;

// On nightly, inline assembly can be used.
#[cfg(feature = "nightly")]
mod impls {
    use core::arch::asm;

    trait HideMemImpl {
        fn hide_mem_impl(ptr: *mut Self);
    }

    impl<T: ?Sized> HideMemImpl for T {
        #[inline]
        default fn hide_mem_impl(ptr: *mut Self) {
            unsafe {
                //llvm_asm!("" : : "r" (ptr as *mut u8) : "memory");
                asm!("/* {0} */", in(reg) (ptr as *mut u8), options(nostack));
            }
        }
    }

    impl<T: Sized> HideMemImpl for T {
        #[inline]
        fn hide_mem_impl(ptr: *mut Self) {
            unsafe {
                //llvm_asm!("" : "=*m" (ptr) : "*0" (ptr));
                asm!("/* {0} */", in(reg) ptr, options(nostack));
            }
        }
    }

    #[inline]
    pub fn hide_mem_impl<T: ?Sized>(ptr: *mut T) {
        HideMemImpl::hide_mem_impl(ptr)
    }
}

// When a C compiler is available, a dummy C function can be used.
#[cfg(not(feature = "no_cc"))]
mod impls {
    extern "C" {
        fn clear_on_drop_hide(ptr: *mut u8) -> *mut u8;
    }

    #[inline]
    pub fn hide_mem_impl<T: ?Sized>(ptr: *mut T) {
        unsafe {
            clear_on_drop_hide(ptr as *mut u8);
        }
    }
}

// When neither is available, pretend the pointer is sent to a thread,
// and hope this is enough to confuse the optimizer.
#[cfg(all(feature = "no_cc", not(feature = "nightly")))]
mod impls {
    use core::sync::atomic::{AtomicUsize, Ordering};

    #[inline(never)]
    pub fn hide_mem_impl<T: ?Sized>(ptr: *mut T) {
        static DUMMY: AtomicUsize = AtomicUsize::new(0);
        DUMMY.store(ptr as *mut u8 as usize, Ordering::Release);
    }
}

#[cfg(test)]
mod tests {
    struct Place {
        data: [u32; 4],
    }

    const DATA: [u32; 4] = [0x01234567, 0x89abcdef, 0xfedcba98, 0x76543210];

    #[test]
    fn hide_mem() {
        let mut place = Place { data: DATA };
        super::hide_mem(&mut place);
        assert_eq!(place.data, DATA);
    }

    #[test]
    fn hide_ptr() {
        let mut place = Place { data: DATA };
        let before = &mut place as *mut _;
        let after = super::hide_ptr(&mut place);
        assert_eq!(before, after as *mut _);
        assert_eq!(after.data, DATA);
    }
}
