extern crate jni_sys;
extern crate env_logger;

use util;
use jni_sys::{JNIEnv, jbyte, jclass, jint, jlong};
use jvmti_sys::jvmtiEnv;
use std::ffi::CString;
use std::ptr;
use std::os::raw::c_uchar;
use std::io::{Cursor, Error};
use std::slice;
use bytecode::classfile::{AccessFlags, Attribute, Classfile, Constant, ConstantPoolIndex, Field, FieldAccessFlags, Instruction, Method, MethodAccessFlags};
use bytecode::io::reader::ClassReader;
use bytecode::io::writer::ClassWriter;

pub unsafe fn define_manip_class(jni_env: *mut JNIEnv) -> Result<(), String> {
    debug!("Defining class");
    // Get the bytes from file
    let class_bytes = include_bytes!("../javalib/native/build/classes/main/stackparam/StackParamNative.class");
    // Define the class
    let class_name = CString::new("stackparam/StackParamNative").unwrap();
    // We don't want the defined class, because it is not "prepared", we want to make them ask for it again
    let _ = (**jni_env).DefineClass.unwrap()(jni_env,
                                             class_name.as_ref().as_ptr(),
                                             ptr::null_mut(),
                                             class_bytes.as_ptr() as *const jbyte,
                                             class_bytes.len() as i32);
    // Confirm no exception
    util::result_or_jni_ex((), jni_env)?;
    return Result::Ok(());
}

pub unsafe fn manip_throwable_class(jvmti_env: *mut jvmtiEnv,
                                    _jni_env: *mut JNIEnv,
                                    class_data_len: jint,
                                    class_data: *const c_uchar,
                                    new_class_data_len: *mut jint,
                                    new_class_data: *mut *mut c_uchar)
                                    -> Result<(), String> {
    // Read the class
    let mut class_file = read_class(class_data_len, class_data)?;

    // Do transforms
    add_stack_params_field(&mut class_file);
    add_native_stack_params_method(&mut class_file);
    update_fill_method(&mut class_file)?;
    replace_our_trace_method(&mut class_file)?;

    // Write the class
    return write_class(jvmti_env, &class_file, new_class_data_len, new_class_data);
}

pub unsafe fn manip_element_class(jvmti_env: *mut jvmtiEnv,
                                  _jni_env: *mut JNIEnv,
                                  class_data_len: jint,
                                  class_data: *const c_uchar,
                                  new_class_data_len: *mut jint,
                                  new_class_data: *mut *mut c_uchar)
                                  -> Result<(), String> {
    // Read the class
    let mut class_file = read_class(class_data_len, class_data)?;

    // Do transforms
    add_param_info_field(&mut class_file);
    replace_elem_to_string(&mut class_file)?;

    // Write the class
    return write_class(jvmti_env, &class_file, new_class_data_len, new_class_data);
}

unsafe fn read_class(class_data_len: jint, class_data: *const c_uchar) -> Result<Classfile, String> {
    let class_data_bytes = slice::from_raw_parts(class_data, class_data_len as usize);
    let mut rdr = Cursor::new(class_data_bytes);
    return str_err(ClassReader::read_class(&mut rdr));
}

unsafe fn write_class(jvmti_env: *mut jvmtiEnv,
                      class_file: &Classfile,
                      new_class_data_len: *mut jint,
                      new_class_data: *mut *mut c_uchar) -> Result<(), String> {
    let mut new_class_curs = Cursor::new(Vec::new());
    str_err(ClassWriter::new(&mut new_class_curs).write_class(&class_file))?;
    let new_class_vec: &Vec<u8> = new_class_curs.get_ref();
    ptr::write(new_class_data_len, new_class_vec.len() as jint);
    let alloc_res = (**jvmti_env).Allocate.unwrap()(jvmti_env, new_class_vec.len() as jlong, new_class_data);
    util::unit_or_jvmti_err(alloc_res)?;
    ptr::copy_nonoverlapping(new_class_vec.as_ptr(), *new_class_data, new_class_vec.len());
    return Result::Ok(());
}

unsafe fn add_param_info_field(class_file: &mut Classfile) {
    let field_name_idx = ConstantPoolIndex { idx: utf8_const(class_file, "paramInfo") };
    let field_desc_idx = ConstantPoolIndex { idx: utf8_const(class_file, "[Ljava/lang/Object;") };
    class_file.fields.push(Field {
        access_flags: AccessFlags { flags: FieldAccessFlags::Transient as u16 },
        name_index: field_name_idx,
        descriptor_index: field_desc_idx,
        attributes: Vec::new(),
    });

    // Note, even if we had code to manip <init> to set our field as null here, it doesn't
    // help as who knows how the StackTraceElement is inited.
}

unsafe fn replace_elem_to_string(class_file: &mut Classfile) -> Result<(), String> {
    // Change current toString to $$stack_param$$toString and make a new native one

    // Rename
    let mut found = false;
    let meth_to_str_name_idx = utf8_const(class_file, "toString");
    let meth_ret_str_desc_idx = utf8_const(class_file, "()Ljava/lang/String;");
    let new_meth_to_str_name_idx = utf8_const(class_file, "$$stack_param$$toString");
    for method in class_file.methods.iter_mut() {
        if method.name_index.idx == meth_to_str_name_idx && method.descriptor_index.idx == meth_ret_str_desc_idx {
            found = true;
            method.name_index = ConstantPoolIndex { idx: new_meth_to_str_name_idx };
        }
    }
    if !found { return Result::Err("Unable to find toString".to_string()); }

    // Make new native method
    class_file.methods.push(Method {
        access_flags: AccessFlags { flags: MethodAccessFlags::Public as u16 + MethodAccessFlags::Native as u16 },
        name_index: ConstantPoolIndex { idx: meth_to_str_name_idx },
        descriptor_index: ConstantPoolIndex { idx: meth_ret_str_desc_idx },
        attributes: Vec::new()
    });
    return Result::Ok(());
}

unsafe fn add_stack_params_field(class_file: &mut Classfile) {
    // Add "private transient Object[][] stackParams" field
    let field_name_idx = ConstantPoolIndex { idx: utf8_const(class_file, "stackParams") };
    let field_desc_idx = ConstantPoolIndex { idx: utf8_const(class_file, "[[Ljava/lang/Object;") };
    class_file.fields.push(Field {
        access_flags: AccessFlags { flags: FieldAccessFlags::Private as u16 + FieldAccessFlags::Transient as u16 },
        name_index: field_name_idx,
        descriptor_index: field_desc_idx,
        attributes: Vec::new(),
    });

    // Note, we choose not to explicitly set the stackParams field to null in Throwable
    // constructors because we do it in fillInStackTrace one way or another
}

unsafe fn add_native_stack_params_method(class_file: &mut Classfile) {
    // Create native stackParamFillInStackTrace(Thread)
    let sp_fill_meth_name_idx = utf8_const(class_file, "stackParamFillInStackTrace");
    let meth_thread_ret_throwable_idx = utf8_const(class_file, "(Ljava/lang/Thread;)Ljava/lang/Throwable;");
    class_file.methods.push(Method {
        access_flags: AccessFlags { flags: MethodAccessFlags::Private as u16 + MethodAccessFlags::Native as u16 },
        name_index: ConstantPoolIndex { idx: sp_fill_meth_name_idx },
        descriptor_index: ConstantPoolIndex { idx: meth_thread_ret_throwable_idx },
        attributes: Vec::new()
    });
}

unsafe fn update_fill_method(class_file: &mut Classfile) -> Result<(), String> {
    // Change existing fillInStackTrace to call stackParamFillInStackTrace(Thread) right after fillInStackTrace(0)
    let fill_meth_name_idx = utf8_const(class_file, "fillInStackTrace");
    // Get the code
    let curr_thread_ref_idx = method_ref_const(class_file, "java/lang/Thread", "currentThread", "()Ljava/lang/Thread;");
    let meth_ret_throwable_idx = utf8_const(class_file, "()Ljava/lang/Throwable;");
    let native_fill_meth_ref_idx = method_ref_const(class_file, "java/lang/Throwable", "fillInStackTrace", "(I)Ljava/lang/Throwable;");
    let new_native_fill_meth_ref_idx = method_ref_const(class_file,
                                                        "java/lang/Throwable",
                                                        "stackParamFillInStackTrace",
                                                        "(Ljava/lang/Thread;)Ljava/lang/Throwable;");
    let mut fill_meth = class_file.methods.iter_mut().find(|m| {
        m.name_index.idx == fill_meth_name_idx && m.descriptor_index.idx == meth_ret_throwable_idx
    }).ok_or("Cannot find fill method".to_string())?;
    let mut fill_meth_code = get_method_code_mut(&mut fill_meth)?;
    // Find the index of the invoke special
    let fill_invoke_idx = fill_meth_code.iter().position(|i| {
        match i {
            &Instruction::INVOKESPECIAL(ref idx) if *idx == native_fill_meth_ref_idx as u16 => true,
            _ => false
        }
    }).ok_or("Cannot find invoke of native fill".to_string())?;
    // Call mine afterwards. "this" is currently on the stack already. It takes the current thread,
    // so we grab that statically before calling so it is on the stack (current max stack of >= 2 is
    // still ok for us). Result is a throwable so the stack is left how we got it.
    fill_meth_code.insert(fill_invoke_idx + 1, Instruction::INVOKESTATIC(curr_thread_ref_idx as u16));
    fill_meth_code.insert(fill_invoke_idx + 2, Instruction::INVOKESPECIAL(new_native_fill_meth_ref_idx as u16));
    return Result::Ok(());
}

unsafe fn replace_our_trace_method(class_file: &mut Classfile) -> Result<(), String> {
    // Rename getOurStackTrace to $$stack_param$$getOurStackTrace, then create a new
    // (synchronized) version that is our native one.

    // Rename
    let mut found = false;
    let meth_get_our_name_idx = utf8_const(class_file, "getOurStackTrace");
    let meth_ret_elems_desc_idx = utf8_const(class_file, "()[Ljava/lang/StackTraceElement;");
    let new_meth_get_our_name_idx = utf8_const(class_file, "$$stack_param$$getOurStackTrace");
    for method in class_file.methods.iter_mut() {
        if method.name_index.idx == meth_get_our_name_idx && method.descriptor_index.idx == meth_ret_elems_desc_idx {
            found = true;
            method.name_index = ConstantPoolIndex { idx: new_meth_get_our_name_idx };
        }
    }
    if !found { return Result::Err("Unable to find getOurStackTrace".to_string()); }

    // Make new native method
    class_file.methods.push(Method {
        access_flags: AccessFlags { flags: MethodAccessFlags::Private as u16 + MethodAccessFlags::Synchronized as u16 + MethodAccessFlags::Native as u16 },
        name_index: ConstantPoolIndex { idx: meth_get_our_name_idx },
        descriptor_index: ConstantPoolIndex { idx: meth_ret_elems_desc_idx },
        attributes: Vec::new()
    });
    return Result::Ok(());
}

unsafe fn get_method_code_mut(method: &mut Method) -> Result<&mut Vec<Instruction>, String> {
    for attr in method.attributes.iter_mut() {
        match attr {
            &mut Attribute::Code { ref mut code, .. } => return Result::Ok(code),
            _ => ()
        }
    }
    return Result::Err("Unable to find code for method".to_string());
}

#[allow(dead_code)]
unsafe fn get_manip_class(jni_env: *mut JNIEnv) -> Result<jclass, String> {
    let class_name = CString::new("stackparam/StackParamNative").unwrap();
    let class = (**jni_env).FindClass.unwrap()(jni_env, class_name.as_ref().as_ptr());
    return util::result_or_jni_ex(class, jni_env);
}

fn str_err<T>(res: Result<T, Error>) -> Result<T, String> {
    return res.map_err(|err| format!("{}", err))
}

fn utf8_const(class_file: &mut Classfile, str: &str) -> usize {
    for i in 0..class_file.constant_pool.constants.len() {
        match class_file.constant_pool.constants[i] {
            Constant::Utf8(ref bytes) => {
                if bytes.as_slice() == str.as_bytes() {
                    return i;
                }
            },
            _ => ()
        }
    }
    let ret = class_file.constant_pool.constants.len();
    class_file.constant_pool.constants.push(Constant::Utf8(str.as_bytes().to_vec()));
    return ret;
}

#[allow(dead_code)]
fn str_const(class_file: &mut Classfile, str: &str) -> usize {
    let utf8_idx = utf8_const(class_file, str);
    for i in 0..class_file.constant_pool.constants.len() {
        match class_file.constant_pool.constants[i] {
            Constant::String(ref idx) => {
                if idx.idx == utf8_idx {
                    return i;
                }
            },
            _ => ()
        }
    }
    let ret = class_file.constant_pool.constants.len();
    class_file.constant_pool.constants.push(Constant::String(ConstantPoolIndex { idx: utf8_idx }));
    return ret;
}

fn class_const(class_file: &mut Classfile, class_name: &str) -> usize {
    let utf8_idx = utf8_const(class_file, class_name);
    for i in 0..class_file.constant_pool.constants.len() {
        match class_file.constant_pool.constants[i] {
            Constant::Class(ref idx) => {
                if idx.idx == utf8_idx {
                    return i;
                }
            },
            _ => ()
        }
    }
    let ret = class_file.constant_pool.constants.len();
    class_file.constant_pool.constants.push(Constant::Class(ConstantPoolIndex { idx: utf8_idx }));
    return ret;
}

#[allow(dead_code)]
fn name_and_type_const(class_file: &mut Classfile, name: &str, desc: &str) -> usize {
    let name_idx = utf8_const(class_file, name);
    let desc_idx = utf8_const(class_file, desc);
    for i in 0..class_file.constant_pool.constants.len() {
        match class_file.constant_pool.constants[i] {
            Constant::NameAndType { ref name_index, ref descriptor_index } => {
                if name_index.idx == name_idx && descriptor_index.idx == desc_idx {
                    return i;
                }
            },
            _ => ()
        }
    }
    let ret = class_file.constant_pool.constants.len();
    class_file.constant_pool.constants.push(Constant::NameAndType {
        name_index: ConstantPoolIndex { idx: name_idx },
        descriptor_index: ConstantPoolIndex { idx: desc_idx },
    });
    return ret;
}

#[allow(dead_code)]
fn field_ref_const(class_file: &mut Classfile, class_name: &str, field_name: &str, desc: &str) -> usize {
    let class_idx = class_const(class_file, class_name);
    let name_and_type_idx = name_and_type_const(class_file, field_name, desc);
    for i in 0..class_file.constant_pool.constants.len() {
        match class_file.constant_pool.constants[i] {
            Constant::FieldRef { ref class_index, ref name_and_type_index } => {
                if class_index.idx == class_idx && name_and_type_index.idx == name_and_type_idx {
                    return i;
                }
            },
            _ => ()
        }
    }
    let ret = class_file.constant_pool.constants.len();
    class_file.constant_pool.constants.push(Constant::FieldRef {
        class_index: ConstantPoolIndex { idx: class_idx },
        name_and_type_index: ConstantPoolIndex { idx: name_and_type_idx },
    });
    return ret;
}

fn method_ref_const(class_file: &mut Classfile, class_name: &str, method_name: &str, desc: &str) -> usize {
    let class_idx = class_const(class_file, class_name);
    let name_and_type_idx = name_and_type_const(class_file, method_name, desc);
    for i in 0..class_file.constant_pool.constants.len() {
        match class_file.constant_pool.constants[i] {
            Constant::MethodRef { ref class_index, ref name_and_type_index } => {
                if class_index.idx == class_idx && name_and_type_index.idx == name_and_type_idx {
                    return i;
                }
            },
            _ => ()
        }
    }
    let ret = class_file.constant_pool.constants.len();
    class_file.constant_pool.constants.push(Constant::MethodRef {
        class_index: ConstantPoolIndex { idx: class_idx },
        name_and_type_index: ConstantPoolIndex { idx: name_and_type_idx },
    });
    return ret;
}
