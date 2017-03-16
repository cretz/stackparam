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
use bytecode::classfile::{Classfile, Constant, ConstantPoolIndex};
use bytecode::io::reader::ClassReader;
use bytecode::io::writer::ClassWriter;


pub trait Manip {
    fn init(&self, jvmti_env: *mut jvmtiEnv, jni_env: *mut JNIEnv) -> Result<(), String>;

    fn manip_throwable_class(&self,
                            jvmti_env: *mut jvmtiEnv,
                            jni_env: *mut JNIEnv,
                            class_data_len: jint,
                            class_data: *const c_uchar,
                            new_class_data_len: *mut jint,
                            new_class_data: *mut *mut c_uchar)
                            -> Result<(), String>;
}

pub fn default_manip() -> Box<Manip> {
    return Box::new(StandardManip {});
}

#[allow(dead_code)]
unsafe fn get_manip_class(jni_env: *mut JNIEnv) -> Result<jclass, String> {
    let class_name = CString::new("stackparam/StackParamNative").unwrap();
    let class = (**jni_env).FindClass.unwrap()(jni_env, class_name.as_ref().as_ptr());
    return util::result_or_jni_ex(class, jni_env);
}

unsafe fn define_manip_class(jni_env: *mut JNIEnv) -> Result<(), String> {
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
    return util::result_or_jni_ex((), jni_env);
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

#[allow(dead_code)]
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

struct StandardManip;

impl Manip for StandardManip {
    fn init(&self, _jvmti_env: *mut jvmtiEnv, jni_env: *mut JNIEnv) -> Result<(), String> {
        unsafe {
            return define_manip_class(jni_env);
        }
    }

    fn manip_throwable_class(&self,
                             jvmti_env: *mut jvmtiEnv,
                             _jni_env: *mut JNIEnv,
                             class_data_len: jint,
                             class_data: *const c_uchar,
                             new_class_data_len: *mut jint,
                             new_class_data: *mut *mut c_uchar)
                             -> Result<(), String> {
        unsafe {
            // Read the class
            let class_data_bytes = slice::from_raw_parts(class_data, class_data_len as usize);
            let mut rdr = Cursor::new(class_data_bytes);
            let mut class_file = str_err(ClassReader::read_class(&mut rdr))?;

            // Add a method to return "Awesome!!"
            use bytecode::classfile::*;
            let str_idx = str_const(&mut class_file, "Awesome!!");
            let method = Method {
                access_flags: AccessFlags { flags: MethodAccessFlags::Public as u16 + MethodAccessFlags::Static as u16 },
                name_index: ConstantPoolIndex { idx: utf8_const(&mut class_file, "testSomething") },
                descriptor_index: ConstantPoolIndex { idx: utf8_const(&mut class_file, "()Ljava/lang/String;") },
                attributes: vec!(
                    Attribute::Code {
                        max_stack: 1,
                        max_locals: 0,
                        exception_table: Vec::new(),
                        attributes: Vec::new(),
                        code: vec!(
                            {
                                if str_idx > u8::max_value() as usize {
                                    Instruction::LDC_W(str_idx as u16)
                                } else {
                                    Instruction::LDC(str_idx as u8)
                                }
                            },
                            Instruction::ARETURN,
                        )
                    }
                )
            };
            class_file.methods.push(method);

            // Write the class
            let mut new_class_curs = Cursor::new(Vec::new());
            str_err(ClassWriter::new(&mut new_class_curs).write_class(&class_file))?;
            let new_class_vec: &Vec<u8> = new_class_curs.get_ref();
            ptr::write(new_class_data_len, new_class_vec.len() as jint);
            let alloc_res = (**jvmti_env).Allocate.unwrap()(jvmti_env, new_class_vec.len() as jlong, new_class_data);
            try!(util::unit_or_jvmti_err(alloc_res));
            ptr::copy_nonoverlapping(new_class_vec.as_ptr(), *new_class_data, new_class_vec.len());
            return Result::Ok(());
        }
    }
}
