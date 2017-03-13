extern crate jni_sys;
extern crate env_logger;

use util;
use jni_sys::{JNIEnv, jbyte, jclass};
use jvmti_sys::jvmtiEnv;
use std::ffi::CString;
use std::ptr;

pub trait Manip {
    fn init(&self, jvmti_env: *mut jvmtiEnv, jni_env: *mut JNIEnv) -> Result<(), String>;
    fn add_throwable_method(&self,
                            jvmti_env: *mut jvmtiEnv,
                            jni_env: *mut JNIEnv,
                            bytes: &[u8])
                            -> Result<&[u8], String>;
}

pub fn default_manip() -> Box<Manip> {
    return Box::new(Ow2Asm {});
}

unsafe fn get_manip_class(jni_env: *mut JNIEnv) -> Result<jclass, String> {
    let class_name = CString::new("stackparam/Ow2AsmManip").unwrap();
    let class = (**jni_env).FindClass.unwrap()(jni_env, class_name.as_ref().as_ptr());
    return util::result_or_jni_ex(class, jni_env);
}

unsafe fn define_manip_class(jni_env: *mut JNIEnv) -> Result<(), String> {
    debug!("Defining class");
    // Get the bytes from file
    let class_bytes = include_bytes!("../javalib/ow2-manip/build/classes/main/stackparam/Ow2AsmManip.class");
    // Define the class
    let class_name = CString::new("stackparam/Ow2AsmManip").unwrap();
    // We don't want the defined class, because it is not "prepared", we want to make them ask for it again
    let _ = (**jni_env).DefineClass.unwrap()(jni_env,
                                             class_name.as_ref().as_ptr(),
                                             ptr::null_mut(),
                                             class_bytes.as_ptr() as *const jbyte,
                                             class_bytes.len() as i32);
    // Confirm no exception
    return util::result_or_jni_ex((), jni_env);
}

struct Ow2Asm;

impl Manip for Ow2Asm {
    fn init(&self, _jvmti_env: *mut jvmtiEnv, jni_env: *mut JNIEnv) -> Result<(), String> {
        unsafe {
            return define_manip_class(jni_env);
        }
    }

    fn add_throwable_method(&self,
                            jvmti_env: *mut jvmtiEnv,
                            jni_env: *mut JNIEnv,
                            bytes: &[u8])
                            -> Result<&[u8], String> {
        unsafe {
            let class = try!(get_manip_class(jni_env));
            let method = try!(util::find_method(jvmti_env, class, "addThrowableMethod"));
            // TODO:
            // let new_bytes =
            debug!("TODO: handle method {:?} and bytes of size {}", method, bytes.len());
            return Result::Ok("".as_bytes());
        }
    }
}
