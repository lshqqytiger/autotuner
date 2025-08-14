use std::{mem, ptr};

#[inline]
pub(crate) fn round_up(a: usize, b: usize) -> usize {
    (a + b - 1) / b
}

#[inline]
pub(crate) unsafe fn block(signal: i32) {
    unsafe {
        let mut sigset = mem::zeroed();
        libc::sigemptyset(&mut sigset);
        libc::sigaddset(&mut sigset, signal);
        libc::sigprocmask(libc::SIG_BLOCK, &sigset, ptr::null_mut());
    }
}
