extern crate jni_sys;

use log::LogLevel::Debug;
use jni_sys::{JNIEnv, jclass, jint, jobject, jmethodID};
use jvmti_sys::{jvmtiEnv, jthread, jvmtiFrameInfo, jvmtiLocalVariableEntry, jvmtiError};
use std::ptr;
use util;
use std::os::raw::{c_char, c_uchar};
use std::slice;
use std::ffi::CStr;

pub static mut JVMTI_ENV: *mut jvmtiEnv = 0 as *mut jvmtiEnv;

#[no_mangle]
#[allow(non_snake_case)]
pub unsafe extern "C" fn Java_stackparam_StackParamNative_loadStackParams(_jni_env: *mut JNIEnv,
                                                                          _cls: jclass,
                                                                          thread: jthread,
                                                                          max_depth: jint) -> jobject {
    // TODO
    match get_params(thread, max_depth) {
        Result::Err(err_str) => debug!("Stack param err: {}", err_str),
        _ => ()
    }
    return ptr::null_mut();
}

unsafe fn get_params(thread: jthread, max_depth: jint) -> Result<(), String> {
    // Grab the trace
    let trace = get_stack_trace(thread, max_depth)?;

    for frame in trace {
        get_frame_args(frame)?;

        // TODO: the rest
    }

    return Result::Ok(());
}

unsafe fn method_name(method: jmethodID) -> Result<String, String> {
    let mut name: *mut c_char = 0 as *mut c_char;
    let name_res = (**JVMTI_ENV).GetMethodName.unwrap()(JVMTI_ENV, method, &mut name, ptr::null_mut(), ptr::null_mut());
    util::unit_or_jvmti_err(name_res)?;
    let name_str = CStr::from_ptr(name).to_string_lossy().clone().into_owned();
    dealloc(name)?;
    return Result::Ok(name_str);
}

unsafe fn get_stack_trace(thread: jthread, max_depth: jint) -> Result<Vec<jvmtiFrameInfo>, String> {
    let mut frames: Vec<jvmtiFrameInfo> = Vec::with_capacity(max_depth as usize);
    let mut frame_count: jint = 0;
    let trace_res = (**JVMTI_ENV).GetStackTrace.unwrap()(JVMTI_ENV,
                                                         thread,
                                                         0,
                                                         max_depth,
                                                         frames.as_mut_ptr(), &mut frame_count);
    util::unit_or_jvmti_err(trace_res)?;
    frames.set_len(frame_count as usize);
    frames.shrink_to_fit();
    return Result::Ok(frames);
}

unsafe fn get_frame_args(frame: jvmtiFrameInfo) -> Result<Vec<jobject>, String> {
    if log_enabled!(Debug) { debug!("Getting info for {}", method_name(frame.method)?); }
    let mut method = get_method_arg_info(frame.method)?;
    if method.mods & 0x00000100 != 0 {
        debug!("Native method, not applying local table or getting values");
        return Result::Ok(Vec::new());
    }
    debug!("Applying local table");
    apply_local_var_table(frame.method, &mut method)?;
    for arg in method.args {
        debug!("Var named {} at slot {} has type {}", arg.name, arg.slot, arg.typ);
    }
    return Result::Ok(Vec::new());
}

struct MethodInfo {
    mods: jint,
    args: Vec<Arg>
}

struct Arg {
    name: String,
    typ: String,
    slot: jint
}

unsafe fn get_method_arg_info(method: jmethodID) -> Result<MethodInfo, String> {
    let mut ret = MethodInfo {
        mods: get_method_modifiers(method)?,
        args: Vec::new(),
    };
    let is_static = ret.mods & 0x00000008 != 0;
    let mut sig: *mut c_char = 0 as *mut c_char;
    let name_res = (**JVMTI_ENV).GetMethodName.unwrap()(JVMTI_ENV, method, ptr::null_mut(), &mut sig, ptr::null_mut());
    util::unit_or_jvmti_err(name_res)?;
    // Parse the sig
    let sig_str = CStr::from_ptr(sig).to_str().map_err(|_| "Error parsing method sig")?;
    let mut sig_chars = sig_str.chars();
    if sig_chars.next() != Some('(') { return Result::Err(format!("Str {} missing opening param", sig_str)); }
    let mut working_str = "".to_string();
    let mut in_obj = false;
    let mut slot_counter = 0;
    if !is_static {
        ret.args.push(Arg {
            name: "this".to_string(),
            typ: get_class_signature(get_method_declaring_class(method)?)?,
            slot: slot_counter,
        });
        slot_counter += 1;
    }
    let mut arg_counter = 0;
    loop {
        match sig_chars.next() {
            None => {
                let _ = dealloc(sig as *mut c_char);
                return Result::Err("Unexpected end of desc".to_string());
            },
            Some(c) => match c {
                ')' => {
                    dealloc(sig)?;
                    return Result::Ok(ret);
                },
                ';' if in_obj => {
                    working_str.push(';');
                    ret.args.push(Arg {
                        name: format!("arg{}", arg_counter),
                        typ: working_str.clone(),
                        slot: slot_counter,
                    });
                    arg_counter += 1;
                    slot_counter += 1;
                    in_obj = false;
                    working_str.clear();
                },
                _ if in_obj => working_str.push(c),
                'L' => {
                    in_obj = true;
                    working_str.push('L');
                }
                '[' => working_str.push('['),
                'B' | 'C' | 'D' | 'F' | 'I' | 'J' | 'S' | 'Z' => {
                    working_str.push(c);
                    ret.args.push(Arg {
                        name: format!("arg{}", arg_counter),
                        typ: working_str.clone(),
                        slot: slot_counter,
                    });
                    arg_counter += 1;
                    if c == 'J' || c == 'D' {
                        slot_counter += 2;
                    } else {
                        slot_counter += 1;
                    }
                    working_str.clear();
                },
                _ => {
                    // Ignore dealloc err
                    let _ = dealloc(sig as *mut c_char);
                    return Result::Err(format!("Unrecognized char: {}", c));
                },
            },
        }
    }
}

unsafe fn dealloc<T>(v: *mut T) -> Result<(), String> {
    let de_res = (**JVMTI_ENV).Deallocate.unwrap()(JVMTI_ENV, v as *mut c_uchar);
    return util::unit_or_jvmti_err(de_res);
}

unsafe fn get_class_signature(class: jclass) -> Result<String, String> {
    let mut ret: *mut c_char = ptr::null_mut();
    let sig_res = (**JVMTI_ENV).GetClassSignature.unwrap()(JVMTI_ENV, class, &mut ret, ptr::null_mut());
    util::unit_or_jvmti_err(sig_res)?;
    let sig_str = CStr::from_ptr(ret).to_string_lossy().clone().into_owned();
    dealloc(ret)?;
    return Result::Ok(sig_str);
}

unsafe fn get_method_declaring_class(method: jmethodID) -> Result<jclass, String> {
    let mut ret: jclass = ptr::null_mut();
    let cls_res = (**JVMTI_ENV).GetMethodDeclaringClass.unwrap()(JVMTI_ENV, method, &mut ret);
    return util::result_or_jvmti_err(ret, cls_res);
}

unsafe fn get_method_modifiers(method: jmethodID) -> Result<jint, String> {
    let mut mods: jint = 0;
    let mod_res = (**JVMTI_ENV).GetMethodModifiers.unwrap()(JVMTI_ENV, method, &mut mods);
    return util::result_or_jvmti_err(mods, mod_res);
}

unsafe fn apply_local_var_table(method: jmethodID, info: &mut MethodInfo) -> Result<(), String> {
    let mut entries: *mut jvmtiLocalVariableEntry = ptr::null_mut();
    let mut entry_count: jint = 0;
    let table_res = (**JVMTI_ENV).GetLocalVariableTable.unwrap()(JVMTI_ENV, method, &mut entry_count, &mut entries);
    if table_res as u32 == jvmtiError::JVMTI_ERROR_ABSENT_INFORMATION as u32 {
        // When information is absent, we don't care
        return Result::Ok(());
    }
    util::unit_or_jvmti_err(table_res)?;
    let entry_slice = slice::from_raw_parts(entries, entry_count as usize);
    if log_enabled!(Debug) {
        for entry in entry_slice {
            debug!("Var table entry named {} at slot {} has type {}",
            CStr::from_ptr(entry.name).to_string_lossy(),
            entry.slot, CStr::from_ptr(entry.signature).to_string_lossy());
        }
    }
    let mut err: Option<String> = None;
    'arg_loop: for arg in info.args.iter_mut() {
        // Find the entry at the expected slot and start location 0, but break
        // if there is something else at that slot but not at location 0
        let mut maybe_entry: Option<&jvmtiLocalVariableEntry> = None;
        for entry in entry_slice {
            if entry.slot == arg.slot {
                if entry.start_location != 0 {
                    err = Some(format!("Var at slot {} should be location 0, but is {}", entry.slot, entry.start_location));
                    break 'arg_loop;
                }
                maybe_entry = Some(entry);
            }
        }
        let entry = match maybe_entry {
            Some(entry) => entry,
            None => {
                err = Some(format!("Can't find var entry for slot {} and location 0", arg.slot));
                break;
            },
        };
        arg.name = CStr::from_ptr(entry.name).to_string_lossy().clone().into_owned();
        // Don't need to own this
        let type_str = CStr::from_ptr(entry.signature).to_string_lossy();
        if type_str != arg.typ {
            err = Some(format!("Var {} expected type {}, got {}", arg.name, arg.typ, type_str.clone()));
            break;
        }
    }
    // Dealloc everything, ignoring errors
    for entry in entry_slice {
        let _ = dealloc(entry.name);
        let _ = dealloc(entry.signature);
        if !entry.generic_signature.is_null() {
            let _ = dealloc(entry.generic_signature);
        }
    }
    let _ = dealloc(entries);
    return match err {
        Some(err_str) => Result::Err(err_str),
        None => Result::Ok(())
    };
}