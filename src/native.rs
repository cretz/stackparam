extern crate jni_sys;

use log::LogLevel::Debug;
use jni_sys::{JNIEnv, jclass, jint, jlong, jfloat, jdouble, jobject, jmethodID, jstring, jobjectArray, jsize};
use jvmti_sys::{jvmtiEnv, jthread, jvmtiFrameInfo, jvmtiLocalVariableEntry, jvmtiError};
use std::ptr;
use util;
use std::os::raw::{c_char, c_uchar, c_uint, c_int, c_double};
use std::slice;
use std::ffi::{CStr, CString};
use std::sync::{Once, ONCE_INIT};
use std::mem;

static mut JVMTI_ENV: *mut jvmtiEnv = 0 as *mut jvmtiEnv;

pub unsafe fn init(jvmti_env: *mut jvmtiEnv) {
    JVMTI_ENV = jvmti_env;
}

#[no_mangle]
#[allow(non_snake_case)]
pub unsafe extern "C" fn Java_stackparam_StackParamNative_loadStackParams(jni_env: *mut JNIEnv,
                                                                          _cls: jclass,
                                                                          thread: jthread,
                                                                          max_depth: jint) -> jobject {
    if thread.is_null() {
        let _ = throw_ex_with_msg(jni_env, "java/lang/NullPointerException", "Thread is null");
        return ptr::null_mut();
    }
    if max_depth < 0 {
        let _ = throw_ex_with_msg(jni_env, "java/lang/IllegalArgumentException", "Max depth < 0");
        return ptr::null_mut();
    }
    // TODO
    return match get_params_as_object_array(jni_env, thread, max_depth) {
        Result::Err(err_str) => {
            debug!("Stack param err: {}", err_str);
            let _ = throw_ex_with_msg(jni_env,
                                      "java/lang/RuntimeException",
                                      format!("Unexpected stack param err: {}", err_str).as_ref());
            ptr::null_mut()
        },
        Result::Ok(methods) => methods
    };
}

unsafe fn get_params_as_object_array(jni_env: *mut JNIEnv, thread: jthread, max_depth: jint) -> Result<jobjectArray, String> {
    let methods = get_params(jni_env, thread, max_depth)?;
    let obj_str = CString::new("java/lang/Object").unwrap();
    let obj_class = util::result_or_jni_ex((**jni_env).FindClass.unwrap()(jni_env, obj_str.as_ptr()), jni_env)?;
    let obj_arr_str = CString::new("[Ljava/lang/Object;").unwrap();
    let obj_arr_class = util::result_or_jni_ex((**jni_env).FindClass.unwrap()(jni_env, obj_arr_str.as_ptr()), jni_env)?;
    let ret = util::result_or_jni_ex((**jni_env).NewObjectArray.unwrap()(jni_env,
                                                                         methods.len() as jsize,
                                                                         obj_arr_class,
                                                                         ptr::null_mut()), jni_env)?;
    let mut unknown_param: jstring = ptr::null_mut();
    for (method_index, method) in methods.iter().enumerate() {
        let param_arr = util::result_or_jni_ex((**jni_env).NewObjectArray.unwrap()(jni_env,
                                                                                   (method.params.len() * 3) as jsize,
                                                                                   obj_class,
                                                                                   ptr::null_mut()), jni_env)?;
        for (param_index, param) in method.params.iter().enumerate() {
            // Goes: param name, param sig, val
            (**jni_env).SetObjectArrayElement.unwrap()(jni_env,
                                                       param_arr,
                                                       (param_index * 3) as jsize,
                                                       new_string(jni_env, param.name.as_ref())?);
            util::result_or_jni_ex((), jni_env)?;
            (**jni_env).SetObjectArrayElement.unwrap()(jni_env,
                                                       param_arr,
                                                       ((param_index * 3) + 1) as jsize,
                                                       new_string(jni_env, param.typ.as_ref())?);
            util::result_or_jni_ex((), jni_env)?;
            let val = match param.val {
                Some(val) => val,
                None => {
                    if unknown_param.is_null() {
                        unknown_param = new_string(jni_env, "<unknown>")?;
                    }
                    unknown_param
                }
            };
            (**jni_env).SetObjectArrayElement.unwrap()(jni_env, param_arr, ((param_index * 3) + 2) as jsize, val);
            util::result_or_jni_ex((), jni_env)?;
        }
        (**jni_env).SetObjectArrayElement.unwrap()(jni_env, ret, method_index as jsize, param_arr);
        util::result_or_jni_ex((), jni_env)?;
    }
    return Result::Ok(ret);
}

unsafe fn throw_ex_with_msg(jni_env: *mut JNIEnv, ex_class: &str, ex_msg: &str) -> Result<(), String> {
    let ex_class_str = CString::new(ex_class).unwrap();
    let class = util::result_or_jni_ex((**jni_env).FindClass.unwrap()(jni_env,
                                                                      ex_class_str.as_ptr()), jni_env)?;
    let ex_msg_str = CString::new(ex_msg).unwrap();
    if (**jni_env).ThrowNew.unwrap()(jni_env, class, ex_msg_str.as_ptr()) < 0 {
        return util::result_or_jni_ex((), jni_env);
    }
    return Result::Ok(());
}

unsafe fn get_params(jni_env: *mut JNIEnv, thread: jthread, max_depth: jint) -> Result<Vec<MethodInfo>, String> {
    // Grab the trace
    let trace = get_stack_trace(thread, max_depth)?;
    // Go over every frame getting the info
    let mut ret: Vec<MethodInfo> = Vec::new();
    for (index, frame) in trace.iter().enumerate() {
        ret.push(get_frame_params(jni_env, thread, frame, index as jint)?);
    }
    return Result::Ok(ret);
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

unsafe fn get_frame_params(jni_env: *mut JNIEnv, thread: jthread, frame: &jvmtiFrameInfo, depth: jint) -> Result<MethodInfo, String> {
    if log_enabled!(Debug) { debug!("Getting info for {}", method_name(frame.method)?); }
    let mut method = get_method_param_info(frame.method)?;
    let is_native = method.mods & 0x00000100 != 0;
    if is_native {
        debug!("Native method, not applying local table or getting values");
    } else {
        debug!("Applying local table");
        apply_local_var_table(frame.method, &mut method)?;
    }
    // Apply the param values if we can get them
    for param in method.params.iter_mut() {
        debug!("Var named {} at slot {} has type {}", param.name, param.slot, param.typ);
        // Now get the local var if we can
        if param.slot == 0 && param.name == "this" {
            param.val = Some(get_this(thread, depth)?);
        } else if !is_native {
            param.val = Some(get_local_var(jni_env, thread, depth, param.slot, param.typ.as_ref())?);
        }
    }
    return Result::Ok(method);
}

unsafe fn new_string(jni_env: *mut JNIEnv, str: &str) -> Result<jstring, String> {
    let cstr = CString::new(str).unwrap();
    return util::result_or_jni_ex((**jni_env).NewStringUTF.unwrap()(jni_env, cstr.as_ptr()), jni_env);
}

unsafe fn get_this(thread: jthread, depth: jint) -> Result<jobject, String> {
    let mut obj: jobject = ptr::null_mut();
    let inst_res = (**JVMTI_ENV).GetLocalInstance.unwrap()(JVMTI_ENV, thread, depth, &mut obj);
    return util::result_or_jvmti_err(obj, inst_res);
}

unsafe fn get_local_var(jni_env: *mut JNIEnv, thread: jthread, depth: jint, slot: jint, typ: &str) -> Result<jobject, String> {
    return match typ {
        "Z" => {
            let val = get_local_int(thread, depth, slot)?;
            let (box_class, box_meth) = primitive_box_methods(jni_env)?.boolean;
            util::result_or_jni_ex(
                (**jni_env).CallStaticObjectMethod.unwrap()(jni_env, box_class, box_meth, val as c_uint), jni_env)
        },
        "B" => {
            let val = get_local_int(thread, depth, slot)?;
            let (box_class, box_meth) = primitive_box_methods(jni_env)?.byte;
            util::result_or_jni_ex(
                (**jni_env).CallStaticObjectMethod.unwrap()(jni_env, box_class, box_meth, val as c_int), jni_env)
        },
        "C" => {
            let val = get_local_int(thread, depth, slot)?;
            let (box_class, box_meth) = primitive_box_methods(jni_env)?.char;
            util::result_or_jni_ex(
                (**jni_env).CallStaticObjectMethod.unwrap()(jni_env, box_class, box_meth, val as c_uint), jni_env)
        },
        "S" => {
            let val = get_local_int(thread, depth, slot)?;
            let (box_class, box_meth) = primitive_box_methods(jni_env)?.short;
            util::result_or_jni_ex(
                (**jni_env).CallStaticObjectMethod.unwrap()(jni_env, box_class, box_meth, val as c_int), jni_env)
        },
        "I" => {
            let val = get_local_int(thread, depth, slot)?;
            let (box_class, box_meth) = primitive_box_methods(jni_env)?.int;
            util::result_or_jni_ex(
                (**jni_env).CallStaticObjectMethod.unwrap()(jni_env, box_class, box_meth, val), jni_env)
        },
        "J" => {
            let mut val: jlong = 0;
            util::unit_or_jvmti_err((**JVMTI_ENV).GetLocalLong.unwrap()(JVMTI_ENV, thread, depth, slot, &mut val))?;
            let (box_class, box_meth) = primitive_box_methods(jni_env)?.long;
            util::result_or_jni_ex(
                (**jni_env).CallStaticObjectMethod.unwrap()(jni_env, box_class, box_meth, val), jni_env)
        },
        "F" => {
            let mut val: jfloat = 0.0;
            util::unit_or_jvmti_err((**JVMTI_ENV).GetLocalFloat.unwrap()(JVMTI_ENV, thread, depth, slot, &mut val))?;
            let (box_class, box_meth) = primitive_box_methods(jni_env)?.float;
            util::result_or_jni_ex(
                (**jni_env).CallStaticObjectMethod.unwrap()(jni_env, box_class, box_meth, val as c_double), jni_env)
        },
        "D" => {
            let mut val: jdouble = 0.0;
            util::unit_or_jvmti_err((**JVMTI_ENV).GetLocalDouble.unwrap()(JVMTI_ENV, thread, depth, slot, &mut val))?;
            let (box_class, box_meth) = primitive_box_methods(jni_env)?.double;
            util::result_or_jni_ex(
                (**jni_env).CallStaticObjectMethod.unwrap()(jni_env, box_class, box_meth, val as c_double), jni_env)
        },
        typ if typ.starts_with("[") || typ.starts_with("L") => {
            let mut val: jobject = ptr::null_mut();
            let local_res = (**JVMTI_ENV).GetLocalObject.unwrap()(JVMTI_ENV, thread, depth, slot, &mut val);
            util::result_or_jvmti_err(val, local_res)
        },
        _ => Result::Err(format!("Unrecognized type: {}", typ))
    }
}

unsafe fn get_local_int(thread: jthread, depth: jint, slot: jint) -> Result<jint, String> {
    let mut val: jint = 0;
    let local_res = (**JVMTI_ENV).GetLocalInt.unwrap()(JVMTI_ENV, thread, depth, slot, &mut val);
    return util::result_or_jvmti_err(val, local_res);
}

struct MethodInfo {
    mods: jint,
    params: Vec<Param>,
}

struct Param {
    name: String,
    typ: String,
    slot: jint,
    val: Option<jobject>,
}

unsafe fn get_method_param_info(method: jmethodID) -> Result<MethodInfo, String> {
    let mut ret = MethodInfo {
        mods: get_method_modifiers(method)?,
        params: Vec::new(),
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
        ret.params.push(Param {
            name: "this".to_string(),
            typ: get_class_signature(get_method_declaring_class(method)?)?,
            slot: slot_counter,
            val: None,
        });
        slot_counter += 1;
    }
    let mut param_counter = 0;
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
                    ret.params.push(Param {
                        name: format!("arg{}", param_counter),
                        typ: working_str.clone(),
                        slot: slot_counter,
                        val: None,
                    });
                    param_counter += 1;
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
                    ret.params.push(Param {
                        name: format!("arg{}", param_counter),
                        typ: working_str.clone(),
                        slot: slot_counter,
                        val: None,
                    });
                    param_counter += 1;
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
    'param_loop: for param in info.params.iter_mut() {
        // Find the entry at the expected slot and start location 0, but break
        // if there is something else at that slot but not at location 0
        let mut maybe_entry: Option<&jvmtiLocalVariableEntry> = None;
        for entry in entry_slice {
            if entry.slot == param.slot {
                if entry.start_location != 0 {
                    err = Some(format!("Var at slot {} should be location 0, but is {}", entry.slot, entry.start_location));
                    break 'param_loop;
                }
                maybe_entry = Some(entry);
            }
        }
        let entry = match maybe_entry {
            Some(entry) => entry,
            None => {
                err = Some(format!("Can't find var entry for slot {} and location 0", param.slot));
                break;
            },
        };
        param.name = CStr::from_ptr(entry.name).to_string_lossy().clone().into_owned();
        // Don't need to own this
        let type_str = CStr::from_ptr(entry.signature).to_string_lossy();
        if type_str != param.typ {
            err = Some(format!("Var {} expected type {}, got {}", param.name, param.typ, type_str.clone()));
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

type MethodRef = (jclass, jmethodID);

struct PrimitiveBoxMethods {
    boolean: MethodRef,
    byte: MethodRef,
    char: MethodRef,
    short: MethodRef,
    int: MethodRef,
    long: MethodRef,
    float: MethodRef,
    double: MethodRef
}

unsafe fn primitive_box_methods(jni_env: *mut JNIEnv) -> Result<PrimitiveBoxMethods, String> {
    static mut PRIM_BOX_METHS: *const Result<PrimitiveBoxMethods, String> = 0 as *const Result<PrimitiveBoxMethods, String>;
    static ONCE: Once = ONCE_INIT;
    ONCE.call_once(|| {
        unsafe fn method_ref(jni_env: *mut JNIEnv, class_name: &str, method_desc: &str) -> Result<MethodRef, String> {
            let class_name_str = CString::new(class_name).unwrap();
            let class = util::result_or_jni_ex((**jni_env).FindClass.unwrap()(jni_env,
                                                                              class_name_str.as_ptr()), jni_env)?;
            let meth_name_str = CString::new("valueOf").unwrap();
            let desc_str = CString::new(method_desc).unwrap();
            let method = util::result_or_jni_ex((**jni_env).GetStaticMethodID.unwrap()(jni_env,
                                                                                       class,
                                                                                       meth_name_str.as_ptr(),
                                                                                       desc_str.as_ptr()), jni_env)?;
            return Result::Ok((class, method));
        }
        unsafe fn prim_box_meths(jni_env: *mut JNIEnv) -> Result<PrimitiveBoxMethods, String> {
            return Result::Ok(PrimitiveBoxMethods {
                boolean: method_ref(jni_env, "java/lang/Boolean", "(Z)Ljava/lang/Boolean;")?,
                byte: method_ref(jni_env, "java/lang/Byte", "(B)Ljava/lang/Byte;")?,
                char: method_ref(jni_env, "java/lang/Character", "(C)Ljava/lang/Character;")?,
                short: method_ref(jni_env, "java/lang/Short", "(S)Ljava/lang/Short;")?,
                int: method_ref(jni_env, "java/lang/Integer", "(I)Ljava/lang/Integer;")?,
                long: method_ref(jni_env, "java/lang/Long", "(J)Ljava/lang/Long;")?,
                float: method_ref(jni_env, "java/lang/Float", "(F)Ljava/lang/Float;")?,
                double: method_ref(jni_env, "java/lang/Double", "(D)Ljava/lang/Double;")?,
            });
        }
        PRIM_BOX_METHS = mem::transmute(Box::new(prim_box_meths(jni_env)));
    });
    return ptr::read(PRIM_BOX_METHS);
}