extern crate jni_sys;
#[macro_use]
extern crate log;
extern crate env_logger;

mod jvmti_sys;
mod manip;
mod util;
pub mod bytecode;
pub mod native;

use jni_sys::{JavaVM, jint, jclass, jobject, JNIEnv, JNI_OK};
use jvmti_sys::{jvmtiEnv, JVMTI_VERSION, jvmtiEventCallbacks, jvmtiCapabilities, jvmtiEventMode, jvmtiEvent, jthread};
use std::os::raw::{c_char, c_void, c_uchar};
use std::ffi::CStr;
use std::mem::size_of;
use std::ptr;

#[no_mangle]
#[allow(non_snake_case)]
pub unsafe extern "C" fn Agent_OnLoad(vm: *mut JavaVM,
                                      options: *mut c_char,
                                      _reserved: *mut c_void)
                                      -> jint {
    debug!("Agent loading");
    match run(vm, options) {
        Ok(()) => debug!("Agent loaded"),
        Err(errStr) => info!("Agent unable to load: {}", errStr),
    }
    return 0;
}

#[no_mangle]
#[allow(non_snake_case)]
pub extern "C" fn Agent_OnUnload(_vm: *mut JavaVM) {
    debug!("Agent unloaded");
}

unsafe fn run(vm: *mut JavaVM, options: *mut c_char) -> Result<(), String> {
    // Init things like logging
    init(options);

    // Get the environment
    let jvmti_env = get_env(vm)?;

    // Add needed capabilities
    add_capabilities(jvmti_env)?;

    // Set the callbacks
    set_event_callbacks(jvmti_env)?;

    // Enable the notifications
    return enable_notifications(jvmti_env);
}

fn init(_options: *mut c_char) {
    let _ = env_logger::init();
}

unsafe fn get_env(vm: *mut JavaVM) -> Result<*mut jvmtiEnv, String> {
    let mut ptr: *mut c_void = ptr::null_mut() as *mut c_void;
    let env_res = (**vm).GetEnv.unwrap()(vm, &mut ptr, JVMTI_VERSION);
    if env_res != JNI_OK {
        return Result::Err(format!("No environment, err: {}", env_res));
    }
    return Result::Ok(ptr as *mut jvmtiEnv);
}

unsafe fn add_capabilities(jvmti_env: *mut jvmtiEnv) -> Result<(), String> {
    let caps = jvmtiCapabilities {
        // can_access_local_variables | can_generate_all_class_hook_events
        _bindgen_bitfield_1_: 0x00004000 | 0x04000000,
        ..Default::default()
    };
    return util::unit_or_jvmti_err((**jvmti_env).AddCapabilities.unwrap()(jvmti_env, &caps));
}

unsafe fn set_event_callbacks(jvmti_env: *mut jvmtiEnv) -> Result<(), String> {
    // We only need init and load hook
    let cb = jvmtiEventCallbacks {
        ClassFileLoadHook: Some(class_file_load_hook),
        VMInit: Some(vm_init),
        ..Default::default()
    };
    let cb_res = (**jvmti_env).SetEventCallbacks.unwrap()(jvmti_env,
                                                          &cb,
                                                          size_of::<jvmtiEventCallbacks>() as i32);
    return util::unit_or_jvmti_err(cb_res);
}

unsafe fn enable_notifications(jvmti_env: *mut jvmtiEnv) -> Result<(), String> {
    enable_notification(jvmti_env, jvmtiEvent::JVMTI_EVENT_VM_INIT)?;
    return enable_notification(jvmti_env, jvmtiEvent::JVMTI_EVENT_CLASS_FILE_LOAD_HOOK);
}

unsafe fn enable_notification(jvmti_env: *mut jvmtiEnv, event: jvmtiEvent) -> Result<(), String> {
    let mode_res = (**jvmti_env).SetEventNotificationMode.unwrap()(jvmti_env,
                                                                   jvmtiEventMode::JVMTI_ENABLE,
                                                                   event,
                                                                   ptr::null_mut());
    return util::unit_or_jvmti_err(mode_res);
}

unsafe fn transform_class_file(jvmti_env: *mut jvmtiEnv,
                               jni_env: *mut JNIEnv,
                               class_being_redefined: jclass,
                               name: *const c_char,
                               class_data_len: jint,
                               class_data: *const c_uchar,
                               new_class_data_len: *mut jint,
                               new_class_data: *mut *mut c_uchar)
                               -> Result<(), String> {
    // Must have name and must be being first class definition
    if name.is_null() || !class_being_redefined.is_null() {
        return Result::Ok(());
    }
    return match CStr::from_ptr(name).to_str() {
        Ok("java/lang/Throwable") =>
            manip::manip_throwable_class(jvmti_env, jni_env, class_data_len, class_data, new_class_data_len, new_class_data),
        Ok("java/lang/StackTraceElement") =>
            manip::manip_element_class(jvmti_env, jni_env, class_data_len, class_data, new_class_data_len, new_class_data),
        _ =>
            Result::Ok(())
    }
}

unsafe extern "C" fn class_file_load_hook(jvmti_env: *mut jvmtiEnv,
                                          jni_env: *mut JNIEnv,
                                          class_being_redefined: jclass,
                                          _loader: jobject,
                                          name: *const c_char,
                                          _protection_domain: jobject,
                                          class_data_len: jint,
                                          class_data: *const c_uchar,
                                          new_class_data_len: *mut jint,
                                          new_class_data: *mut *mut c_uchar)
                                          -> () {
    match transform_class_file(jvmti_env,
                               jni_env,
                               class_being_redefined,
                               name,
                               class_data_len,
                               class_data,
                               new_class_data_len,
                               new_class_data) {
        Ok(()) => (),
        Err(err_str) => info!("Failed to hook class: {}", err_str)
    }
}

unsafe extern "C" fn vm_init(jvmti_env: *mut jvmtiEnv, jni_env: *mut JNIEnv, _thread: jthread) -> () {
    info!("Agent initializing");
    // Set the global jvmti env for later jni use
    native::init(jvmti_env);
    match manip::define_manip_class(jni_env) {
        Ok(()) => info!("Agent initialized"),
        Err(err_str) => info!("Unable to initialize agent: {}", err_str),
    }
}