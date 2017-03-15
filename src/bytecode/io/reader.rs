use std::io::{ Cursor, Read, Error, ErrorKind };
use std::cell::Cell;
use super::super::classfile::*;

pub struct ClassReader {
}

impl ClassReader {

    pub fn read_class<T>(source: &mut T) -> Result<Classfile, Error> where T: Read {
        let mut reader = BlockReader::new(source);

        let fns: Vec<fn(&mut BlockReader, &ClassFragment) -> Result<ClassFragment, Error>> = vec![
            ClassReader::read_magic_bytes,
            ClassReader::read_classfile_version,
            ClassReader::read_constant_pool,
            ClassReader::read_access_flags,
            ClassReader::read_this_class,
            ClassReader::read_super_class,
            ClassReader::read_interfaces,
            ClassReader::read_fields,
            ClassReader::read_methods,
            ClassReader::read_class_attributes
        ];

        let result = fns.iter().fold(Ok(ClassFragment::default()), |acc, x| {
            match acc {
                Ok(acc_fragment) => match x(&mut reader, &acc_fragment) {
                    Ok(cur_fragment) => Ok(acc_fragment.merge(cur_fragment)),
                    err@_ => err
                },
                err@_ => err
            }
        });

        match result {
            Ok(fragment) => Ok(fragment.to_class()),
            Err(err) => Err(err)
        }
    }

    fn read_magic_bytes(reader: &mut BlockReader, _: &ClassFragment) -> Result<ClassFragment, Error> {
        match reader.read_u32() {
            Ok(0xCAFEBABE) => Ok(ClassFragment::default()),
            _ => Err(Error::new(ErrorKind::InvalidData, "Invalid magic bytes"))
        }
    }

    fn read_classfile_version(reader: &mut BlockReader, _: &ClassFragment) -> Result<ClassFragment, Error> {
        match (reader.read_u16(), reader.read_u16()) {
            (Ok(minor_version), Ok(major_version)) => {
                Ok(ClassFragment {
                    version: Some(ClassfileVersion::new(major_version, minor_version)),
                    ..Default::default()
                })
            },
            _ => Err(Error::new(ErrorKind::UnexpectedEof, "Could not read classfile version number"))
        }
    }

    fn read_constant_pool(reader: &mut BlockReader, _: &ClassFragment) -> Result<ClassFragment, Error> {
        match reader.read_u16() {
            Ok(cp_len) => {
                let mut constants: Vec<Constant> = vec![ Constant::Placeholder ];

                for _ in 1..cp_len {
                    if constants.len() < cp_len as usize {
                        match ClassReader::read_constant(reader) {
                            Ok(constant) => {
                                let constant_size = constant.cp_size();

                                constants.push(constant);

                                for _ in 1..constant_size {
                                    constants.push(Constant::Placeholder);
                                }
                            },
                            Err(err) => return Err(err)
                        }
                    }
                }

                Ok(ClassFragment {
                    constant_pool: Some(ConstantPool::new(constants)),
                    ..Default::default()
                })
            },
            Err(err) => Err(err)
        }
    }

    fn read_constant(reader: &mut BlockReader) -> Result<Constant, Error> {
        let tag = reader.read_u8();

        match tag {
            Ok(1) => match reader.read_u16() {
                Ok(str_len) => match reader.read_n(str_len as usize) {
                    Ok(bytes) => {
                        Ok(Constant::Utf8(bytes))
                    },
                    Err(err) => Err(err)
                },
                Err(err) => Err(err)
            },
            Ok(3) => reader.read_u32().map(|value| Constant::Integer(value)),
            Ok(4) => reader.read_u32().map(|value| Constant::Float(value)),
            Ok(5) => reader.read_u64().map(|value| Constant::Long(value)),
            Ok(6) => reader.read_u64().map(|value| Constant::Double(value)),
            Ok(7) => reader.read_u16().map(|idx| Constant::Class(ConstantPoolIndex::new(idx as usize))),
            Ok(8) => reader.read_u16().map(|idx| Constant::String(ConstantPoolIndex::new(idx as usize))),
            Ok(9) => ClassReader::require_n(reader, 4, |mut r| Constant::FieldRef {
                class_index: ConstantPoolIndex::new(r.get_u16() as usize),
                name_and_type_index: ConstantPoolIndex::new(r.get_u16() as usize)
            }),
            Ok(10) => ClassReader::require_n(reader, 4, |mut r| Constant::MethodRef {
                class_index: ConstantPoolIndex::new(r.get_u16() as usize),
                name_and_type_index: ConstantPoolIndex::new(r.get_u16() as usize)
            }),
            Ok(11) => ClassReader::require_n(reader, 4, |mut r| Constant::InterfaceMethodRef {
                class_index: ConstantPoolIndex::new(r.get_u16() as usize),
                name_and_type_index: ConstantPoolIndex::new(r.get_u16() as usize)
            }),
            Ok(12) => ClassReader::require_n(reader, 4, |mut r| Constant::NameAndType {
                    name_index: ConstantPoolIndex::new(r.get_u16() as usize),
                    descriptor_index: ConstantPoolIndex::new(r.get_u16() as usize)
            }),
            Ok(15) => ClassReader::require_n(reader, 3, |mut r| Constant::MethodHandle {
                reference_kind: ReferenceKind::from_u8(r.get_u8()),
                reference_index: ConstantPoolIndex::new(r.get_u16() as usize)
            }),
            Ok(16) => reader.read_u16().map(|idx| Constant::MethodType(ConstantPoolIndex::new(idx as usize))),
            Ok(18) => ClassReader::require_n(reader, 4, |mut r| Constant::InvokeDynamic {
                bootstrap_method_attr_index: ConstantPoolIndex::new(r.get_u16() as usize),
                name_and_type_index: ConstantPoolIndex::new(r.get_u16() as usize)
            }),
            Ok(tag) => Ok(Constant::Unknown(tag)),
            Err(err) => Err(err)
        }
    }

    fn read_access_flags(reader: &mut BlockReader, _: &ClassFragment) -> Result<ClassFragment, Error> {
        match reader.read_u16() {
            Ok(val) => Ok(ClassFragment {
                access_flags: Some(AccessFlags::of(val)),
                ..Default::default()
            }),
            Err(err) => Err(err)
        }
    }

    fn read_this_class(reader: &mut BlockReader, _: &ClassFragment) -> Result<ClassFragment, Error> {
        match ClassReader::read_constant_pool_index(reader) {
            Ok(idx) => Ok(ClassFragment {
                this_class: Some(idx),
                ..Default::default()
            }),
            Err(err) => Err(err)
        }
    }

    fn read_super_class(reader: &mut BlockReader, _: &ClassFragment) -> Result<ClassFragment, Error> {
        match ClassReader::read_constant_pool_index(reader) {
            Ok(idx) => Ok(ClassFragment {
                super_class: Some(idx),
                ..Default::default()
            }),
            Err(err) => Err(err)
        }
    }

    fn read_interfaces(reader: &mut BlockReader, _: &ClassFragment) -> Result<ClassFragment, Error> {
        match reader.read_u16() {
            Ok(ifs_len) => {
                (0..ifs_len).fold(Ok(vec![]), |acc, _| {
                    match acc {
                        Ok(mut ifs) => match ClassReader::read_constant_pool_index(reader) {
                            Ok(interface) => {
                                ifs.push(interface);
                                Ok(ifs)
                            },
                            Err(err) => Err(err)
                        },
                        err@_ => err
                    }
                })
            },
            Err(err) => Err(err)
        }.map(|ifs| ClassFragment {
            interfaces: Some(ifs),
            ..Default::default()
        })
    }

    fn read_fields(reader: &mut BlockReader, cf: &ClassFragment) -> Result<ClassFragment, Error> {
        match reader.read_u16() {
            Ok(fields_len) => {
                (0..fields_len).fold(Ok(vec![]), |acc, _| {
                    match acc {
                        Ok(mut fields) => match ClassReader::read_field(reader, cf) {
                            Ok(field) => {
                                fields.push(field);
                                Ok(fields)
                            },
                            Err(err) => Err(err)
                        },
                        err@_ => err
                    }
                })
            },
            Err(err) => Err(err)
        }.map(|fields| ClassFragment {
            fields: Some(fields),
            ..Default::default()
        })
    }

    fn read_field(reader: &mut BlockReader, cf: &ClassFragment) -> Result<Field, Error> {
        match ClassReader::require_n(reader, 6, |mut r| { (r.get_u16(), r.get_u16(), r.get_u16()) }) {
            Ok((flags, n_idx, d_idx)) => match ClassReader::read_attributes(reader, cf) {
                Ok(attributes) => Ok(Field {
                    access_flags: AccessFlags::of(flags),
                    name_index: ConstantPoolIndex::new(n_idx as usize),
                    descriptor_index: ConstantPoolIndex::new(d_idx as usize),
                    attributes: attributes
                }),
                Err(err) => Err(err)
            },
            Err(err) => Err(err)
        }
    }

    fn read_methods(reader: &mut BlockReader, cf: &ClassFragment) -> Result<ClassFragment, Error> {
        match reader.read_u16() {
            Ok(methods_len) => {
                (0..methods_len).fold(Ok(vec![]), |acc, _| {
                    match acc {
                        Ok(mut methods) => match ClassReader::read_method(reader, cf) {
                            Ok(method) => {
                                methods.push(method);
                                Ok(methods)
                            },
                            Err(err) => Err(err)
                        },
                        err@_ => err
                    }
                })
            },
            Err(err) => Err(err)
        }.map(|methods| ClassFragment {
            methods: Some(methods),
            ..Default::default()
        })
    }

    fn read_method(reader: &mut BlockReader, cf: &ClassFragment) -> Result<Method, Error> {
        match ClassReader::require_n(reader, 6, |mut r| { (r.get_u16(), r.get_u16(), r.get_u16()) }) {
            Ok((flags, n_idx, d_idx)) => match ClassReader::read_attributes(reader, cf) {
                Ok(attributes) => Ok(Method {
                    access_flags: AccessFlags::of(flags),
                    name_index: ConstantPoolIndex::new(n_idx as usize),
                    descriptor_index: ConstantPoolIndex::new(d_idx as usize),
                    attributes: attributes
                }),
                Err(err) => Err(err)
            },
            Err(err) => Err(err)
        }
    }

    fn read_class_attributes(reader: &mut BlockReader, cf: &ClassFragment) -> Result<ClassFragment, Error> {
        match ClassReader::read_attributes(reader, cf) {
            Ok(attributes) => Ok(ClassFragment {
                attributes: Some(attributes),
                ..Default::default()
            }),
            Err(err) => Err(err)
        }
    }

    fn read_attributes(reader: &mut BlockReader, cf: &ClassFragment) -> Result<Vec<Attribute>, Error> {
        match reader.read_u16() {
            Ok(attr_len) => (0..attr_len).fold(Ok(vec![]), |acc, _| {
                match acc {
                    Ok(mut attributes) => match ClassReader::read_attribute(reader, cf) {
                        Ok(attribute) => {
                            attributes.push(attribute);
                            Ok(attributes)
                        },
                        Err(err) => Err(err)
                    },
                    err@_ => err
                }
            }),
            Err(err) => Err(err)
        }
    }

    fn read_attribute(reader: &mut BlockReader, cf: &ClassFragment) -> Result<Attribute, Error> {
        match reader.read_u16() {
            Ok(n_idx) => match reader.read_u32() {
                Ok(a_len) => match reader.read_n(a_len as usize) {
                    Ok(mut bytes) => Ok(ClassReader::parse_attribute(n_idx, BlockReader::new(&mut Cursor::new(&mut bytes)), cf)),
                    Err(err) => Err(err)
                },
                Err(err) => Err(err)
            },
            Err(err) => Err(err)
        }
    }

    fn parse_code(len: usize, reader: &mut BlockReader) -> Vec<Instruction> {
        /*
        let read_bytes: Cell<usize> = Cell::new(0);

        (0..len).take_while(|_| read_bytes.get() < len).map(|_| {
            let instruction = ClassReader::parse_instruction(reader, read_bytes.get());

            read_bytes.set(read_bytes.get() + instruction.len());
            instruction
        }).collect()*/
        let read_bytes: Cell<usize> = Cell::new(0);

        (0..len).take_while(|_| read_bytes.get() < len).map(|_| {
            let instruction = ClassReader::parse_instruction(reader, read_bytes.get());

            read_bytes.set(reader.position());
            instruction
        }).collect()
    }

    fn parse_instruction(reader: &mut BlockReader, current_offset: usize) -> Instruction {
        let opcode = reader.get_u8();

        let instruction = match opcode {
            0x32 => Instruction::AALOAD,
            0x53 => Instruction::AASTORE,
            0x01 => Instruction::ACONST_NULL,
            0x19 => Instruction::ALOAD(reader.get_u8()),
            0x2a => Instruction::ALOAD_0,
            0x2b => Instruction::ALOAD_1,
            0x2c => Instruction::ALOAD_2,
            0x2d => Instruction::ALOAD_3,
            0xbd => Instruction::ANEWARRAY(reader.get_u16()),
            0xb0 => Instruction::ARETURN,
            0xbe => Instruction::ARRAYLENGTH,
            0x3a => Instruction::ASTORE(reader.get_u8()),
            0x4b => Instruction::ASTORE_0,
            0x4c => Instruction::ASTORE_1,
            0x4d => Instruction::ASTORE_2,
            0x4e => Instruction::ASTORE_3,
            0xbf => Instruction::ATHROW,
            0x33 => Instruction::BALOAD,
            0x54 => Instruction::BASTORE,
            0x10 => Instruction::BIPUSH(reader.get_u8()),
            0x34 => Instruction::CALOAD,
            0x55 => Instruction::CASTORE,
            0xc0 => Instruction::CHECKCAST(reader.get_u16()),
            0x90 => Instruction::D2F,
            0x8e => Instruction::D2I,
            0x8f => Instruction::D2L,
            0x63 => Instruction::DADD,
            0x31 => Instruction::DALOAD,
            0x52 => Instruction::DASTORE,
            0x97 => Instruction::DCMPL,
            0x98 => Instruction::DCMPG,
            0x0e => Instruction::DCONST_0,
            0x0f => Instruction::DCONST_1,
            0x6f => Instruction::DDIV,
            0x18 => Instruction::DLOAD(reader.get_u8()),
            0x26 => Instruction::DLOAD_0,
            0x27 => Instruction::DLOAD_1,
            0x28 => Instruction::DLOAD_2,
            0x29 => Instruction::DLOAD_3,
            0x6b => Instruction::DMUL,
            0x77 => Instruction::DNEG,
            0x73 => Instruction::DREM,
            0xaf => Instruction::DRETURN,
            0x39 => Instruction::DSTORE(reader.get_u8()),
            0x47 => Instruction::DSTORE_0,
            0x48 => Instruction::DSTORE_1,
            0x49 => Instruction::DSTORE_2,
            0x4a => Instruction::DSTORE_3,
            0x67 => Instruction::DSUB,
            0x59 => Instruction::DUP,
            0x5a => Instruction::DUP_X1,
            0x5b => Instruction::DUP_X2,
            0x5c => Instruction::DUP2,
            0x5d => Instruction::DUP2_X1,
            0x5e => Instruction::DUP2_X2,
            0x8d => Instruction::F2D,
            0x8b => Instruction::F2I,
            0x8c => Instruction::F2L,
            0x62 => Instruction::FADD,
            0x30 => Instruction::FALOAD,
            0x51 => Instruction::FASTORE,
            0x95 => Instruction::FCMPL,
            0x96 => Instruction::FCMPG,
            0x0b => Instruction::FCONST_0,
            0x0c => Instruction::FCONST_1,
            0x0d => Instruction::FCONST_2,
            0x6e => Instruction::FDIV,
            0x17 => Instruction::FLOAD(reader.get_u8()),
            0x22 => Instruction::FLOAD_0,
            0x23 => Instruction::FLOAD_1,
            0x24 => Instruction::FLOAD_2,
            0x25 => Instruction::FLOAD_3,
            0x6a => Instruction::FMUL,
            0x76 => Instruction::FNEG,
            0x72 => Instruction::FREM,
            0xae => Instruction::FRETURN,
            0x38 => Instruction::FSTORE(reader.get_u8()),
            0x43 => Instruction::FSTORE_0,
            0x44 => Instruction::FSTORE_1,
            0x45 => Instruction::FSTORE_2,
            0x46 => Instruction::FSTORE_3,
            0x66 => Instruction::FSUB,
            0xb4 => Instruction::GETFIELD(reader.get_u16()),
            0xb2 => Instruction::GETSTATIC(reader.get_u16()),
            0xa7 => Instruction::GOTO(reader.get_u16() as i16),
            0xc8 => Instruction::GOTO_W(reader.get_u32() as i32),
            0x91 => Instruction::I2B,
            0x92 => Instruction::I2C,
            0x87 => Instruction::I2D,
            0x86 => Instruction::I2F,
            0x85 => Instruction::I2L,
            0x93 => Instruction::I2S,
            0x60 => Instruction::IADD,
            0x2e => Instruction::IALOAD,
            0x7e => Instruction::IAND,
            0x4f => Instruction::IASTORE,
            0x02 => Instruction::ICONST_M1,
            0x03 => Instruction::ICONST_0,
            0x04 => Instruction::ICONST_1,
            0x05 => Instruction::ICONST_2,
            0x06 => Instruction::ICONST_3,
            0x07 => Instruction::ICONST_4,
            0x08 => Instruction::ICONST_5,
            0x6c => Instruction::IDIV,
            0xa5 => Instruction::IF_ACMPEQ(reader.get_u16() as i16),
            0xa6 => Instruction::IF_ACMPNE(reader.get_u16() as i16),
            0x9f => Instruction::IF_ICMPEQ(reader.get_u16() as i16),
            0xa0 => Instruction::IF_ICMPNE(reader.get_u16() as i16),
            0xa1 => Instruction::IF_ICMPLT(reader.get_u16() as i16),
            0xa2 => Instruction::IF_ICMPGE(reader.get_u16() as i16),
            0xa3 => Instruction::IF_ICMPGT(reader.get_u16() as i16),
            0xa4 => Instruction::IF_ICMPLE(reader.get_u16() as i16),
            0x99 => Instruction::IFEQ(reader.get_u16() as i16),
            0x9a => Instruction::IFNE(reader.get_u16() as i16),
            0x9b => Instruction::IFLT(reader.get_u16() as i16),
            0x9c => Instruction::IFGE(reader.get_u16() as i16),
            0x9d => Instruction::IFGT(reader.get_u16() as i16),
            0x9e => Instruction::IFLE(reader.get_u16() as i16),
            0xc7 => Instruction::IFNONNULL(reader.get_u16() as i16),
            0xc6 => Instruction::IFNULL(reader.get_u16() as i16),
            0x84 => Instruction::IINC(reader.get_u8(), reader.get_u8() as i8),
            0x15 => Instruction::ILOAD(reader.get_u8()),
            0x1a => Instruction::ILOAD_0,
            0x1b => Instruction::ILOAD_1,
            0x1c => Instruction::ILOAD_2,
            0x1d => Instruction::ILOAD_3,
            0x68 => Instruction::IMUL,
            0x74 => Instruction::INEG,
            0xc1 => Instruction::INSTANCEOF(reader.get_u16()),
            0xba => (Instruction::INVOKEDYNAMIC(reader.get_u16()), reader.get_u16()).0,
            0xb9 => (Instruction::INVOKEINTERFACE(reader.get_u16(), reader.get_u8()), reader.get_u8()).0,
            0xb7 => Instruction::INVOKESPECIAL(reader.get_u16()),
            0xb8 => Instruction::INVOKESTATIC(reader.get_u16()),
            0xb6 => Instruction::INVOKEVIRTUAL(reader.get_u16()),
            0x80 => Instruction::IOR,
            0x70 => Instruction::IREM,
            0xac => Instruction::IRETURN,
            0x78 => Instruction::ISHL,
            0x7a => Instruction::ISHR,
            0x36 => Instruction::ISTORE(reader.get_u8()),
            0x3b => Instruction::ISTORE_0,
            0x3c => Instruction::ISTORE_1,
            0x3d => Instruction::ISTORE_2,
            0x3e => Instruction::ISTORE_3,
            0x64 => Instruction::ISUB,
            0x7c => Instruction::IUSHR,
            0x82 => Instruction::IXOR,
            0xa8 => Instruction::JSR(reader.get_u16() as i16),
            0xc9 => Instruction::JSR_W(reader.get_u32() as i32),
            0x8a => Instruction::L2D,
            0x89 => Instruction::L2F,
            0x88 => Instruction::L2I,
            0x61 => Instruction::LADD,
            0x2f => Instruction::LALOAD,
            0x7f => Instruction::LAND,
            0x50 => Instruction::LASTORE,
            0x94 => Instruction::LCMP,
            0x09 => Instruction::LCONST_0,
            0x0a => Instruction::LCONST_1,
            0x12 => Instruction::LDC(reader.get_u8()),
            0x13 => Instruction::LDC_W(reader.get_u16()),
            0x14 => Instruction::LDC2_W(reader.get_u16()),
            0x6d => Instruction::LDIV,
            0x16 => Instruction::LLOAD(reader.get_u8()),
            0x1e => Instruction::LLOAD_0,
            0x1f => Instruction::LLOAD_1,
            0x20 => Instruction::LLOAD_2,
            0x21 => Instruction::LLOAD_3,
            0x69 => Instruction::LMUL,
            0x75 => Instruction::LNEG,
            0xab => {
                let padding = (4 - ((current_offset + 1) % 4)) % 4;
                let _ = reader.get_n(padding);
                let default =  reader.get_u32() as i32;
                let n = reader.get_u32();

                Instruction::LOOKUPSWITCH(default, {
                    (0..n).map(|_| (reader.get_u32() as i32, reader.get_u32() as i32)).collect()
                })
            },
            0x81 => Instruction::LOR,
            0x71 => Instruction::LREM,
            0xad => Instruction::LRETURN,
            0x79 => Instruction::LSHL,
            0x7b => Instruction::LSHR,
            0x37 => Instruction::LSTORE(reader.get_u8()),
            0x3f => Instruction::LSTORE_0,
            0x40 => Instruction::LSTORE_1,
            0x41 => Instruction::LSTORE_2,
            0x42 => Instruction::LSTORE_3,
            0x65 => Instruction::LSUB,
            0x7d => Instruction::LUSHR,
            0x83 => Instruction::LXOR,
            0xc2 => Instruction::MONITORENTER,
            0xc3 => Instruction::MONITOREXIT,
            0xc5 => Instruction::MULTIANEWARRAY(reader.get_u16(), reader.get_u8()),
            0xbb => Instruction::NEW(reader.get_u16()),
            0xbc => Instruction::NEWARRAY(reader.get_u8()),
            0x00 => Instruction::NOP,
            0x57 => Instruction::POP,
            0x58 => Instruction::POP2,
            0xb5 => Instruction::PUTFIELD(reader.get_u16()),
            0xb3 => Instruction::PUTSTATIC(reader.get_u16()),
            0xa9 => Instruction::RET(reader.get_u8()),
            0xb1 => Instruction::RETURN,
            0x35 => Instruction::SALOAD,
            0x56 => Instruction::SASTORE,
            0x11 => Instruction::SIPUSH(reader.get_u16()),
            0x5f => Instruction::SWAP,
            0xaa => {
                let padding = (4 - ((current_offset + 1) % 4)) % 4;
                let _ = reader.get_n(padding);

                let default = reader.get_u32() as i32;
                let low = reader.get_u32() as i32;
                let high = reader.get_u32() as i32;

                Instruction::TABLESWITCH(default, low, high, (0..high - low + 1).map(|_| reader.get_u32() as i32).collect())
            },
            0xc4 => {
                let opcode = reader.get_u8();
                let index = reader.get_u16();

                match opcode {
                    0x15 => Instruction::ILOAD_W(index),
                    0x17 => Instruction::FLOAD_W(index),
                    0x19 => Instruction::ALOAD_W(index),
                    0x16 => Instruction::LLOAD_W(index),
                    0x18 => Instruction::DLOAD_W(index),
                    0x36 => Instruction::ISTORE_W(index),
                    0x38 => Instruction::FSTORE_W(index),
                    0x3a => Instruction::ASTORE_W(index),
                    0x37 => Instruction::LSTORE_W(index),
                    0x39 => Instruction::DSTORE_W(index),
                    0xa9 => Instruction::RET_W(index),
                    0x84 => {
                        let constbyte = reader.get_u16();
                        Instruction::IINC_W(index, constbyte as i16)
                    },
                    // This should not be happening
                    _ => Instruction::WTF(opcode as u32)

                }
            },
            _ => Instruction::WTF(opcode as u32)
        };

        instruction
    }

    fn parse_attribute(idx: u16, mut reader: BlockReader, cf: &ClassFragment) -> Attribute {
        match cf.constant_pool {
            Some(ref cp) => match cp.get_utf8_string(idx) {
                Some(ref s) => match s.as_str() {
                    "ConstantValue" => Some(Attribute::ConstantValue(ConstantPoolIndex::new(reader.get_u16() as usize))),
                    "Code" => Some(Attribute::Code {
                        max_stack: reader.get_u16(),
                        max_locals: reader.get_u16(),
                        code: {
                            let n = reader.get_u32() as usize;
                            ClassReader::parse_code(n, &mut BlockReader::new(&mut Cursor::new(reader.get_n(n as usize))))
                        },
                        exception_table: {
                            let n = reader.get_u16();
                            (0..n).map(|_| ExceptionHandler { start_pc: reader.get_u16(), end_pc: reader.get_u16(), handler_pc: reader.get_u16(), catch_type: ConstantPoolIndex::new(reader.get_u16() as usize) }).collect()
                        },
                        attributes: ClassReader::read_attributes(&mut reader, cf).unwrap_or(vec![])
                        }),
                    "StackMapTable" => Some(Attribute::StackMapTable({
                        let n = reader.get_u16();
                        (0..n).map(|_| {
                            let frame_type = reader.get_u8();

                            let read_verification_type = |r: &mut BlockReader| match r.get_u8() {
                                0 => VerificationType::Top,
                                1 => VerificationType::Integer,
                                2 => VerificationType::Float,
                                3 => VerificationType::Double,
                                4 => VerificationType::Long,
                                5 => VerificationType::Null,
                                6 => VerificationType::UninitializedThis,
                                7 => VerificationType::Object { cpool_index: ConstantPoolIndex::new(r.get_u16() as usize ) },
                                8 => VerificationType::Uninitialized { offset: r.get_u16() },
                                _ => VerificationType::Top
                            };

                            match frame_type {
                                tag@0...63 => StackMapFrame::SameFrame { tag: tag },
                                tag@64...127 => StackMapFrame::SameLocals1StackItemFrame { tag: tag, stack: read_verification_type(&mut reader) },
                                247 => StackMapFrame::SameLocals1StackItemFrameExtended { offset_delta: reader.get_u16(), stack: read_verification_type(&mut reader) },
                                tag@248...250 => StackMapFrame::ChopFrame { tag: tag, offset_delta: reader.get_u16() },
                                251 => StackMapFrame::SameFrameExtended { offset_delta: reader.get_u16() },
                                tag@252...254 => StackMapFrame::AppendFrame { tag: tag, offset_delta: reader.get_u16(), locals: (0..tag - 251).map(|_| read_verification_type(&mut reader)).collect() },
                                255 => StackMapFrame::FullFrame { offset_delta: reader.get_u16(), locals: {
                                    let n = reader.get_u16();
                                    (0..n).map(|_| read_verification_type(&mut reader)).collect()
                                }, stack: {
                                    let n = reader.get_u16();
                                    (0..n).map(|_| read_verification_type(&mut reader)).collect()
                                }},
                                tag@_ => StackMapFrame::FutureUse { tag: tag },
                            }
                        }).collect()
                    })),
                    "Exceptions" => Some(Attribute::Exceptions({
                        let n = reader.get_u16();
                        (0..n).map(|_| ConstantPoolIndex::new(reader.get_u16() as usize)).collect()
                        })),
                    "InnerClasses" => Some(Attribute::InnerClasses({
                        let n = reader.get_u16();
                        (0..n).map(|_| InnerClass {
                            inner_class_info_index: ConstantPoolIndex::new(reader.get_u16() as usize),
                            outer_class_info_index: ConstantPoolIndex::new(reader.get_u16() as usize),
                            inner_name_index: ConstantPoolIndex::new(reader.get_u16() as usize),
                            access_flags: AccessFlags::of(reader.get_u16())
                            }).collect()
                        })),
                    "EnclosingMethod" => Some(Attribute::EnclosingMethod { class_index: ConstantPoolIndex::new(reader.get_u16() as usize), method_index: ConstantPoolIndex::new(reader.get_u16() as usize)}),
                    "Synthetic" => Some(Attribute::Synthetic),
                    "Signature" => Some(Attribute::Signature(ConstantPoolIndex::new(reader.get_u16() as usize))),
                    "SourceFile" => Some(Attribute::SourceFile(ConstantPoolIndex::new(reader.get_u16() as usize))),
                    "SourceDebugExtension" => Some(Attribute::SourceDebugExtension(reader.get_bytes())),
                    "LineNumberTable" => Some(Attribute::LineNumberTable({
                        let n = reader.get_u16();
                        (0..n).map(|_| LineNumberTable {
                            start_pc: reader.get_u16(),
                            line_number: reader.get_u16()
                        }).collect()
                    })),
                    "LocalVariableTable" => Some(Attribute::LocalVariableTable({
                        let n = reader.get_u16();
                        (0..n).map(|_| LocalVariableTable {
                            start_pc: reader.get_u16(),
                            length: reader.get_u16(),
                            name_index: ConstantPoolIndex::new(reader.get_u16() as usize),
                            descriptor_index: ConstantPoolIndex::new(reader.get_u16() as usize),
                            index: reader.get_u16()
                        }).collect()
                    })),
                    "LocalVariableTypeTable" => Some(Attribute::LocalVariableTypeTable({
                        let n = reader.get_u16();
                        (0..n).map(|_| LocalVariableTypeTable {
                            start_pc: reader.get_u16(),
                            length: reader.get_u16(),
                            name_index: ConstantPoolIndex::new(reader.get_u16() as usize),
                            signature_index: ConstantPoolIndex::new(reader.get_u16() as usize),
                            index: reader.get_u16()
                        }).collect()
                    })),
                    "Deprecated" => Some(Attribute::Deprecated),
                    "RuntimeVisibleAnnotations" => Some(Attribute::RuntimeVisibleAnnotations({
                        let n = reader.get_u16();
                        (0..n).map(|_| ClassReader::read_annotation(&mut reader)).collect()
                    })),
                    "RuntimeInvisibleAnnotations" => Some(Attribute::RuntimeInvisibleAnnotations({
                        let n = reader.get_u16();
                        (0..n).map(|_| ClassReader::read_annotation(&mut reader)).collect()
                    })),
                    "RuntimeVisibleParameterAnnotations" => Some(Attribute::RuntimeVisibleParameterAnnotations({
                        let n = reader.get_u8();

                        (0..n).map(|_| {
                            let m = reader.get_u16();
                            (0..m).map(|_| ClassReader::read_annotation(&mut reader)).collect()
                        }).collect()
                    })),
                    "RuntimeInvisibleParameterAnnotations" => Some(Attribute::RuntimeInvisibleParameterAnnotations({
                        let n = reader.get_u8();

                        (0..n).map(|_| {
                            let m = reader.get_u16();
                            (0..m).map(|_| ClassReader::read_annotation(&mut reader)).collect()
                        }).collect()
                    })),
                    "RuntimeVisibleTypeAnnotations" => Some(Attribute::RuntimeVisibleTypeAnnotations({
                        let n = reader.get_u16();
                        (0..n).map(|_| ClassReader::read_type_annotation(&mut reader)).collect()
                    })),
                    "AnnotationDefault" => Some(Attribute::AnnotationDefault(ClassReader::read_element_value(&mut reader))),
                    "BootstrapMethods" => Some(Attribute::BootstrapMethods({
                        let n = reader.get_u16();
                        (0..n).map(|_| BootstrapMethod {
                            bootstrap_method_ref: ConstantPoolIndex::new(reader.get_u16() as usize),
                            bootstrap_arguments: {
                                let m = reader.get_u16();
                                (0..m).map(|_| ConstantPoolIndex::new(reader.get_u16() as usize)).collect()
                            }
                        }).collect()
                    })),
                    "MethodParameters" => Some(Attribute::MethodParameters({
                        let n = reader.get_u8();
                        (0..n).map(|_| MethodParameter {
                            name_index: ConstantPoolIndex::new(reader.get_u16() as usize),
                            access_flags: AccessFlags::of(reader.get_u16())
                        }).collect()
                    })),
                    _ => None
                },
                _ => None
            },
            _ => None
        }.unwrap_or(Attribute::RawAttribute { name_index: ConstantPoolIndex::new(idx as usize), info: reader.get_bytes() })
    }

    fn read_annotation(reader: &mut BlockReader) -> Annotation {
        Annotation {
            type_index: ConstantPoolIndex::new(reader.get_u16() as usize),
            element_value_pairs: {
                let en = reader.get_u16();
                (0..en).map(|_| ElementValuePair {
                    element_name_index: ConstantPoolIndex::new(reader.get_u16() as usize),
                    value: ClassReader::read_element_value(reader)
                }).collect()
            }
        }
    }

    fn read_type_annotation(reader: &mut BlockReader) -> TypeAnnotation {
        TypeAnnotation {
            target_info: match reader.get_u8() {
                // 0x00 type parameter declaration of generic class or interface
                // 0x01 type parameter declaration of generic method or constructor
                subtype @ 0x00...0x01 => TargetInfo::TypeParameter { subtype: subtype, idx: reader.get_u8() },
                // type in extends or implements clause of class declaration (including the direct superclass or direct superinterface of an anonymous class declaration), or in extends clause of interface declaration
                0x10 => TargetInfo::SuperType { idx: reader.get_u16() },
                // 0x11 type in bound of type parameter declaration of generic class or interface
                // 0x12 type in bound of type parameter declaration of generic method or constructor
                subtype @ 0x11...0x12 => TargetInfo::TypeParameterBound { subtype: subtype, param_idx: reader.get_u8(), bound_index: reader.get_u8() },
                // 0x13 type in field declaration
                // 0x14 return type of method, or type of newly constructed object
                // 0x15 receiver type of method or constructor
                subtype @ 0x13...0x15 => TargetInfo::Empty { subtype: subtype },
                // type in formal parameter declaration of method, constructor, or lambda expression
                0x16 => TargetInfo::MethodFormalParameter { idx: reader.get_u8() },
                // type in throws clause of method or constructor
                0x17 => TargetInfo::Throws { idx: reader.get_u16() },
                // 0x40 type in local variable declaration
                // 0x41 type in resource variable declaration
                subtype @ 0x40...0x41 => TargetInfo::LocalVar { subtype: subtype, target: {
                    let count = reader.get_u16();

                                        //u2 start_pc;    u2 length;        u2 index;
                    (0..count).map(|_| (reader.get_u16(), reader.get_u16(), reader.get_u16())).collect()
                }},
                // type in exception parameter declaration
                0x42 => TargetInfo::Catch { idx: reader.get_u16() },
                // 0x43 type in instanceof expression
                // 0x44 type in new expression
                // 0x45 type in method reference expression using ::new
                // 0x46 type in method reference expression using ::Identifier
                subtype @ 0x43...0x46 => TargetInfo::Offset { subtype: subtype, idx: reader.get_u16() },
                // 0x48 type argument for generic constructor in new expression or explicit constructor invocation statement
                // 0x49 type argument for generic method in method invocation expression
                // 0x4A type argument for generic constructor in method reference expression using ::new
                // 0x4B type argument for generic method in method reference expression using ::Identifier
                subtype @ 0x47...0x4b => TargetInfo::TypeArgument { subtype: subtype, offset: reader.get_u16(), type_arg_idx: reader.get_u8() },
                // TODO replace the below fallback branch with proper error handling
                _ => TargetInfo::Empty { subtype: 0 }
            },
            target_path: TypePath {
                path: {
                    let n = reader.get_u8();
                    (0..n).map(|_| (match reader.get_u8() {
                        0 => TypePathKind::Array,
                        1 => TypePathKind::Nested,
                        2 => TypePathKind::Wildcard,
                        3 => TypePathKind::TypeArgument,
                        // TODO replace the below fallback branch with proper error handling
                        _ => TypePathKind::Array
                    }, reader.get_u8())).collect()
                }
            },
            type_index: ConstantPoolIndex::new(reader.get_u16() as usize),
            element_value_pairs: {
                let n = reader.get_u16();
                (0..n).map(|_| ElementValuePair {
                    element_name_index: ConstantPoolIndex::new(reader.get_u16() as usize),
                    value: ClassReader::read_element_value(reader)
                }).collect()
            }
        }
    }

    fn read_element_value(reader: &mut BlockReader) -> ElementValue {
        let tag = reader.get_u8();

        match tag {
            66 /* B */ => ElementValue::ConstantValue(tag, ConstantPoolIndex::new(reader.get_u16() as usize)),
            67 /* C */ => ElementValue::ConstantValue(tag, ConstantPoolIndex::new(reader.get_u16() as usize)),
            68 /* D */ => ElementValue::ConstantValue(tag, ConstantPoolIndex::new(reader.get_u16() as usize)),
            70 /* F */ => ElementValue::ConstantValue(tag, ConstantPoolIndex::new(reader.get_u16() as usize)),
            73 /* I */ => ElementValue::ConstantValue(tag, ConstantPoolIndex::new(reader.get_u16() as usize)),
            74 /* J */ => ElementValue::ConstantValue(tag, ConstantPoolIndex::new(reader.get_u16() as usize)),
            83 /* S */ => ElementValue::ConstantValue(tag, ConstantPoolIndex::new(reader.get_u16() as usize)),
            90 /* Z */ => ElementValue::ConstantValue(tag, ConstantPoolIndex::new(reader.get_u16() as usize)),
            115 /* s */ => ElementValue::ConstantValue(tag, ConstantPoolIndex::new(reader.get_u16() as usize)),
            101 /* e */ => ElementValue::Enum {
                type_name_index: ConstantPoolIndex::new(reader.get_u16() as usize),
                const_name_index: ConstantPoolIndex::new(reader.get_u16() as usize) },
            99 /* c */ => ElementValue::ClassInfo(ConstantPoolIndex::new(reader.get_u16() as usize)),
            64 /* @ */ => ElementValue::Annotation(ClassReader::read_annotation(reader)),
            91 /* [ */ => ElementValue::Array({
                let n = reader.get_u16();
                (0..n).map(|_| ClassReader::read_element_value(reader)).collect()
            }),
            _ => ElementValue::ConstantValue(tag, ConstantPoolIndex::new(0)) // TODO this deserves a better error handling
        }
    }

    fn read_constant_pool_index(reader: &mut BlockReader) -> Result<ConstantPoolIndex, Error> {
        match reader.read_u16() {
            Ok(idx) => Ok(ConstantPoolIndex::new(idx as usize)),
            Err(err) => Err(err)
        }
    }

    fn require_n<T, U>(reader: &mut BlockReader, count: usize, extractor: U) -> Result<T, Error> where U: Fn(BlockReader) -> T {
        match reader.read_n(count) {
            Ok(bytes) => {
                let mut cursor = Cursor::new(bytes);
                let r = BlockReader::new(&mut cursor);

                Ok(extractor(r))
            },
            Err(err) => Err(err)
        }
    }
}

// TODO remove pub after testing
pub struct BlockReader<'a> {
    source: &'a mut Read,
    position: usize
}

impl<'a> BlockReader<'a> {

    pub fn new<T>(source: &'a mut T) -> BlockReader where T: Read {
        BlockReader { source: source, position: 0 }
    }

    pub fn read_u64(&mut self) -> Result<u64, Error> {
        let mut buf: [u8; 8] = [0; 8];

        match self.source.read_exact(&mut buf) {
            Ok(_) => {
                self.position += 8;

                Ok(
                ((buf[0] as u64) << 56) +
                ((buf[1] as u64) << 48) +
                ((buf[2] as u64) << 40) +
                ((buf[3] as u64) << 32) +
                ((buf[4] as u64) << 24) +
                ((buf[5] as u64) << 16) +
                ((buf[6] as u64) << 8) +
                buf[7] as u64)
            },
            Err(err) => Err(err)
        }
    }

    #[allow(dead_code)]
    pub fn get_u64(&mut self) -> u64 {
        self.read_u64().unwrap_or(0)
    }

    pub fn read_u32(&mut self) -> Result<u32, Error> {
        let mut buf: [u8; 4] = [0; 4];

        match self.source.read_exact(&mut buf) {
            Ok(_) => {
                self.position += 4;
                Ok(
                ((buf[0] as u32) << 24) +
                ((buf[1] as u32) << 16) +
                ((buf[2] as u32) << 8) +
                buf[3] as u32)
            },
            Err(err) => Err(err)
        }
    }

    #[allow(dead_code)]
    pub fn get_u32(&mut self) -> u32 {
        self.read_u32().unwrap_or(0)
    }

    pub fn read_u16(&mut self) -> Result<u16, Error> {
        let mut buf: [u8; 2] = [0; 2];

        match self.source.read_exact(&mut buf) {
            Ok(_) => {
                self.position += 2;
                Ok(((buf[0] as u16) << 8) + buf[1] as u16)
            },
            Err(err) => Err(err)
        }
    }

    pub fn get_u16(&mut self) -> u16 {
        self.read_u16().unwrap_or(0)
    }

    pub fn read_u8(&mut self) -> Result<u8, Error> {
        let mut buf: [u8; 1] = [0; 1];

        match self.source.read_exact(&mut buf) {
            Ok(_) => {
                self.position += 1;
                Ok(buf[0])
            },
            Err(err) => Err(err)
        }
    }

    pub fn get_u8(&mut self) -> u8 {
        self.read_u8().unwrap_or(0)
    }

    pub fn read_n(&mut self, count: usize) -> Result<Vec<u8>, Error> {
        let mut tmp: Vec<u8> = Vec::with_capacity(count);

        match self.source.take(count as u64).read_to_end(&mut tmp) {
            Ok(_) => {
                self.position += count;
                Ok(tmp)
            },
            Err(err) => Err(err)
        }
    }

    pub fn get_n(&mut self, count: usize) -> Vec<u8> {
        match self.read_n(count) {
            Ok(bytes) => bytes,
            Err(_) => vec![]
        }
    }

    pub fn read_bytes(&mut self) -> Result<Vec<u8>, Error> {
        let mut tmp: Vec<u8> = vec![];

        match self.source.read_to_end(&mut tmp) {
            Ok(_) => {
                self.position += tmp.len();
                Ok(tmp)
            },
            Err(err) => Err(err)
        }
    }

    pub fn get_bytes(&mut self) -> Vec<u8> {
        let mut tmp: Vec<u8> = vec![];

        let _ = self.source.read_to_end(&mut tmp);

        tmp
    }

    pub fn position(&self) -> usize {
        self.position
    }
}


struct ClassFragment {
    pub version: Option<ClassfileVersion>,
    pub constant_pool: Option<ConstantPool>,
    pub access_flags: Option<AccessFlags>,
    pub this_class: Option<ConstantPoolIndex>,
    pub super_class: Option<ConstantPoolIndex>,
    pub interfaces: Option<Vec<ConstantPoolIndex>>,
    pub fields: Option<Vec<Field>>,
    pub methods: Option<Vec<Method>>,
    pub attributes: Option<Vec<Attribute>>
}

impl ClassFragment {
    pub fn merge(mut self, other: Self) -> Self {
        self.version = other.version.or(self.version);
        self.constant_pool = other.constant_pool.or(self.constant_pool);
        self.access_flags = other.access_flags.or(self.access_flags);
        self.this_class = other.this_class.or(self.this_class);
        self.super_class = other.super_class.or(self.super_class);
        self.interfaces = other.interfaces.or(self.interfaces);
        self.fields = other.fields.or(self.fields);
        self.methods = other.methods.or(self.methods);
        self.attributes = other.attributes.or(self.attributes);
        self
    }

    /// Transform this class fragment into a final class file. Members set on the fragment will
    /// be defined on the class too, other members will be initialized with their default values
    pub fn to_class(self) -> Classfile {
        Classfile {
            version: self.version.unwrap_or(ClassfileVersion::default()),
            constant_pool: self.constant_pool.unwrap_or(ConstantPool::default()),
            access_flags: self.access_flags.unwrap_or(AccessFlags::new()),
            this_class: self.this_class.unwrap_or(ConstantPoolIndex::default()),
            super_class: self.super_class.unwrap_or(ConstantPoolIndex::default()),
            interfaces: self.interfaces.unwrap_or(vec![]),
            fields: self.fields.unwrap_or(vec![]),
            methods: self.methods.unwrap_or(vec![]),
            attributes: self.attributes.unwrap_or(vec![])
        }
    }
}

impl Default for ClassFragment {
    fn default() -> Self {
        ClassFragment {
            version: None,
            constant_pool: None,
            access_flags: None,
            this_class: None,
            super_class: None,
            interfaces: None,
            fields: None,
            methods: None,
            attributes: None
        }
    }
}
