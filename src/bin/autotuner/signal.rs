use std::{mem, ptr};

#[macro_export]
macro_rules! with_signal_mask {
    ($signal:expr, $body:block) => {
        unsafe {
            signal::block($signal);
            let result = $body;
            signal::unblock($signal);
            result
        }
    };
}

#[inline]
pub unsafe fn block(signal: i32) {
    unsafe {
        let mut sigset = mem::zeroed();
        libc::sigemptyset(&mut sigset);
        libc::sigaddset(&mut sigset, signal);
        libc::sigprocmask(libc::SIG_BLOCK, &sigset, ptr::null_mut());
    }
}

#[inline]
pub unsafe fn unblock(signal: i32) {
    unsafe {
        let mut sigset = mem::zeroed();
        libc::sigemptyset(&mut sigset);
        libc::sigaddset(&mut sigset, signal);
        libc::sigprocmask(libc::SIG_UNBLOCK, &sigset, ptr::null_mut());
    }
}
