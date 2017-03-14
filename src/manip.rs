extern crate jni_sys;
extern crate env_logger;

use util;
use jni_sys::{JNIEnv, jbyte, jclass, jsize};
use jvmti_sys::jvmtiEnv;
use std::ffi::CString;
use std::ptr;
use std::borrow::Cow;

pub trait Manip {
    fn init(&self, jvmti_env: *mut jvmtiEnv, jni_env: *mut JNIEnv) -> Result<(), String>;
    fn add_throwable_method(&self,
                            jvmti_env: *mut jvmtiEnv,
                            jni_env: *mut JNIEnv,
                            bytes: &[u8])
                            -> Result<Cow<[u8]>, String>;
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
                            -> Result<Cow<[u8]>, String> {
        unsafe {
            let class = try!(get_manip_class(jni_env));
            let method = try!(util::find_method(jvmti_env, class, "addThrowableMethod"));
            // Not worth a direct byte buffer, we copy in and copy on the way back, who cares
            // this is just happening on init.
            let byte_array = (**jni_env).NewByteArray.unwrap()(jni_env, bytes.len() as jsize);
            if byte_array.is_null() {
                return Result::Err("Unable to create new byte array".to_string());
            }
            (**jni_env).SetByteArrayRegion.unwrap()(jni_env,
                                                    byte_array,
                                                    0,
                                                    bytes.len() as jsize,
                                                    bytes.as_ptr() as *const jbyte);
            try!(util::result_or_jni_ex((), jni_env));
            let new_bytes = try!(util::result_or_jni_ex(
                (**jni_env).CallStaticObjectMethod.unwrap()(jni_env, class, method, byte_array), jni_env));
            let new_bytes_len = (**jni_env).GetArrayLength.unwrap()(jni_env, new_bytes);
            let mut new_bytes_vec: Vec<u8> = Vec::with_capacity(new_bytes_len as usize);
            (**jni_env).GetByteArrayRegion.unwrap()(jni_env,
                                                    new_bytes,
                                                    0,
                                                    new_bytes_len,
                                                    new_bytes_vec.as_mut_ptr() as *mut jbyte);
            try!(util::result_or_jni_ex((), jni_env));
            let ret_bytes_vec = Vec::from_raw_parts(new_bytes_vec.as_mut_ptr(),
                                                    new_bytes_len as usize,
                                                    new_bytes_len as usize);
            let ret_bytes: Cow<[u8]> = Cow::from(ret_bytes_vec);
            return Result::Ok(ret_bytes);
        }
    }
}
