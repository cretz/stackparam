extern crate jni_sys;
extern crate libc;

use jni_sys::JavaVM;
use libc::{c_char, c_void};

#[no_mangle]
#[allow(non_snake_case, unused_variables)]
pub extern fn Agent_OnLoad(vm: *mut JavaVM, options: *mut c_char, reserved: *mut c_void) -> jni_sys::jint {
    println!("Loaded!");
    return 0;
}

#[no_mangle]
#[allow(non_snake_case, unused_variables)]
pub extern fn Agent_OnUnload(vm: *mut JavaVM) {
    println!("Unloaded!");
}
