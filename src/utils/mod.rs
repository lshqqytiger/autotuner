pub(crate) mod interner;
pub(crate) mod manually_move;
pub(crate) mod union;

use std::{mem, ptr};

#[inline]
pub(crate) unsafe fn block(signal: i32) {
    unsafe {
        let mut sigset = mem::zeroed();
        libc::sigemptyset(&mut sigset);
        libc::sigaddset(&mut sigset, signal);
        libc::sigprocmask(libc::SIG_BLOCK, &sigset, ptr::null_mut());
    }
}

#[inline]
pub(crate) unsafe fn unblock(signal: i32) {
    unsafe {
        let mut sigset = mem::zeroed();
        libc::sigemptyset(&mut sigset);
        libc::sigaddset(&mut sigset, signal);
        libc::sigprocmask(libc::SIG_UNBLOCK, &sigset, ptr::null_mut());
    }
}
