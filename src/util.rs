extern crate jni_sys;
extern crate env_logger;

use jni_sys::{JNIEnv, jclass, jmethodID, jint};
use jvmti_sys::{jvmtiEnv, jvmtiError};
use std::ffi::CStr;
use std::ptr;
use std::slice;
use std::os::raw::c_char;

pub unsafe fn result_or_jni_ex<T>(res: T, jni_env: *mut JNIEnv) -> Result<T, String> {
    if (**jni_env).ExceptionCheck.unwrap()(jni_env) == 1 {
        // TODO: extract the exception info instead of dumping to stderr
        (**jni_env).ExceptionDescribe.unwrap()(jni_env);
        (**jni_env).ExceptionClear.unwrap()(jni_env);
        return Result::Err("Unexpected exception, logged to stderr".to_string());
    }
    return Result::Ok(res);
}

pub unsafe fn unit_or_jvmti_err(res: jvmtiError) -> Result<(), String> {
    if res as i32 != 0 {
        return Result::Err(format!("Unexpected jvmti error: {:?}", res));
    }
    return Result::Ok(());
}

#[allow(dead_code)]
pub unsafe fn find_method(jvmti_env: *mut jvmtiEnv,
                          class: jclass,
                          name: &str)
                          -> Result<jmethodID, String> {
    // TODO: sad we can't use GetMethodID
    // ref: http://stackoverflow.com/questions/42746496/call-class-method-from-manually-defined-class-in-jvmti
    let mut method_count: jint = 0;
    let mut methods: *mut jmethodID = ptr::null_mut();
    let meth_ret =
        (**jvmti_env).GetClassMethods.unwrap()(jvmti_env, class, &mut method_count, &mut methods);
    try!(unit_or_jvmti_err(meth_ret));
    let method_slice: &[jmethodID] = slice::from_raw_parts_mut(methods, method_count as usize);
    let ret = method_slice.into_iter().find(|&&m| {
        let mut method_name: *mut c_char = ptr::null_mut();
        let name_ret = (**jvmti_env).GetMethodName.unwrap()(jvmti_env,
                                                            m,
                                                            &mut method_name,
                                                            ptr::null_mut(),
                                                            ptr::null_mut()) as
                       i32;
        name_ret == 0 && CStr::from_ptr(method_name).to_str().unwrap() == name
    });
    return match ret {
               Some(&method) => Result::Ok(method),
               None => Result::Err("Method not found".to_string()),
           };
}
