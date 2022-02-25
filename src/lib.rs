//! Low-level bindings for the [libunwind] library.
//!
//! Please see the libunwind  [C API documentation] for function descriptions.
//!
//! [libunwind]: http://www.nongnu.org/libunwind/
//! [C API documentation]: https://www.nongnu.org/libunwind/docs.html

#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]

include!(concat!(env!("OUT_DIR"), "/bindings.rs"));

pub use crate::native::*;

#[cfg_attr(target_arch = "x86_64", path = "x86_64.rs")]
#[cfg_attr(target_arch = "x86", path = "x86.rs")]
#[cfg_attr(target_arch = "arm", path = "arm.rs")]
mod native;

#[cfg(test)]
mod tests {
    extern crate libc;

    use crate::*;
    use libc::c_char;
    use libc::c_void;
    use std::ffi::CStr;
    use std::ffi::CString;
    use std::io;
    use std::mem::MaybeUninit;
    use std::path::PathBuf;
    use std::process::Command;
    use std::ptr;
    use std::thread;
    use std::time::Duration;

    #[test]
    #[cfg(target_arch = "x86_64")]
    fn test_local_unwind() {
        unsafe {
            let mut c = MaybeUninit::uninit();
            let mut uc = MaybeUninit::uninit();
            let mut ip: unw_word_t = 0;
            let _ret = unw_init_local(c.as_mut_ptr(), uc.as_mut_ptr());
            let mut backtrace = String::new();
            loop {
                unw_get_reg(
                    c.as_mut_ptr(),
                    UNW_TDEP_IP as ::std::os::raw::c_int,
                    &mut ip,
                );
                let mut off = MaybeUninit::uninit();
                let mut name_vec: Vec<c_char> = vec![0; 64];
                unw_get_proc_name(c.as_mut_ptr(), name_vec.as_mut_ptr(), 64, off.as_mut_ptr());
                let name = CStr::from_ptr(name_vec.as_mut_ptr());
                backtrace.push_str(&format!("0x{:x} in {:?} ()\n", ip, name.to_str().unwrap()));
                let ret = unw_step(c.as_mut_ptr());
                if ret <= 0 {
                    break;
                }
            }
            println!("{}", backtrace);
            assert!(backtrace.contains("test_local_unwind"), true);
            assert!(
                backtrace.contains("start_thread") || backtrace.contains("start"),
                true
            );
        }
    }
}
