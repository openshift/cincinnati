#if defined(__GNUC__) && __GNUC__ >= 4
#if !defined(_WIN32) && !defined(__CYGWIN__)
__attribute__ ((visibility ("hidden")))
#endif
#endif
unsigned char *clear_on_drop_hide(unsigned char *ptr) {
    #if defined(__GNUC__)
    /* Not needed with MSVC, since Rust uses LLVM and LTO can't inline this. */
    __asm__ volatile ("" : "=r" (ptr) : "0" (ptr) : "memory");
    #endif
    return ptr;
}
