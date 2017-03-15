use std::io::{ Write, Error, ErrorKind };
use super::super::classfile::*;

pub struct ClassWriter<'a> {
    target: &'a mut Write
}

impl<'a> ClassWriter<'a> {
    pub fn new<T>(target: &'a mut T) -> ClassWriter where T: Write {
        ClassWriter { target: target }
    }

    pub fn write_class(&mut self, classfile: &Classfile) -> Result<usize, Error> {
        self.write_magic_bytes()
        .and(self.write_classfile_version(&classfile.version))
        .and(self.write_constant_pool(&classfile.constant_pool))
        .and(self.write_access_flags(&classfile.access_flags))
        .and(self.write_constant_pool_index(&classfile.this_class))
        .and(self.write_constant_pool_index(&classfile.super_class))
        .and(self.write_interfaces(&classfile.interfaces))
        .and(self.write_fields(&classfile.fields, &classfile.constant_pool))
        .and(self.write_methods(&classfile.methods, &classfile.constant_pool))
        .and(self.write_attributes(&classfile.attributes, &classfile.constant_pool))
    }

    pub fn write_magic_bytes(&mut self) -> Result<usize, Error> {
        self.write_u32(0xCAFEBABE)
    }

    pub fn write_classfile_version(&mut self, version: &ClassfileVersion) -> Result<usize, Error> {
        self.write_u16(version.minor_version)
        .and(self.write_u16(version.major_version))
    }

    pub fn write_constant_pool(&mut self, cp: &ConstantPool) -> Result<usize, Error> {
        cp.constants.iter().fold(self.write_u16(cp.cp_len() as u16), |acc, x| {
            match acc {
                Ok(ctr) => self.write_constant(x).map(|c| c + ctr),
                err@_ => err
            }
        })
    }

    fn write_constant(&mut self, constant: &Constant) -> Result<usize, Error> {
        match constant {
            &Constant::Utf8(ref bytes) => self.write_u8(1).and(self.write_u16(bytes.len() as u16)).and(self.write_n(bytes)),
            &Constant::Integer(ref value) => self.write_u8(3).and(self.write_u32(*value)),
            &Constant::Float(ref value) => self.write_u8(4).and(self.write_u32(*value)),
            &Constant::Long(ref value) => self.write_u8(5).and(self.write_u64(*value)),
            &Constant::Double(ref value) => self.write_u8(6).and(self.write_u64(*value)),
            &Constant::Class(ref idx) => self.write_u8(7).and(self.write_u16(idx.idx as u16)),
            &Constant::String(ref idx) => self.write_u8(8).and(self.write_u16(idx.idx as u16)),
            &Constant::MethodType(ref idx) => self.write_u8(16).and(self.write_u16(idx.idx as u16)),
            &Constant::FieldRef { class_index: ref c_idx, name_and_type_index: ref n_idx } => self.write_u8(9).and(self.write_u16(c_idx.idx as u16)).and(self.write_u16(n_idx.idx as u16)),
            &Constant::MethodRef { class_index: ref c_idx, name_and_type_index: ref n_idx } => self.write_u8(10).and(self.write_u16(c_idx.idx as u16)).and(self.write_u16(n_idx.idx as u16)),
            &Constant::InterfaceMethodRef { class_index: ref c_idx, name_and_type_index: ref n_idx } => self.write_u8(11).and(self.write_u16(c_idx.idx as u16)).and(self.write_u16(n_idx.idx as u16)),
            &Constant::NameAndType { name_index: ref n_idx, descriptor_index: ref d_idx } => self.write_u8(12).and(self.write_u16(n_idx.idx as u16)).and(self.write_u16(d_idx.idx as u16)),
            &Constant::MethodHandle { reference_kind: ref kind, reference_index: ref r_idx } => self.write_u8(15).and(self.write_u8(kind.to_u8())).and(self.write_u16(r_idx.idx as u16)),
            &Constant::InvokeDynamic { bootstrap_method_attr_index: ref m_idx, name_and_type_index: ref n_idx } => self.write_u8(18).and(self.write_u16(m_idx.idx as u16)).and(self.write_u16(n_idx.idx as u16)),
            &Constant::Placeholder => Ok(0),
            _ => Err(Error::new(ErrorKind::InvalidData, "Unknown constant detected"))
        }
    }

    fn write_access_flags(&mut self, flags: &AccessFlags) -> Result<usize, Error> {
        self.write_u16(flags.flags)
    }

    fn write_constant_pool_index(&mut self, class_index: &ConstantPoolIndex) -> Result<usize, Error> {
        self.write_u16(class_index.idx as u16)
    }

    fn write_interfaces(&mut self, ifs: &Vec<ConstantPoolIndex>) -> Result<usize, Error> {
        ifs.iter().fold(self.write_u16(ifs.len() as u16), |acc, x| {
            match acc {
                Ok(ctr) => self.write_u16(x.idx as u16).map(|c| c + ctr),
                err@_ => err
            }
        })
    }

    fn write_fields(&mut self, fields: &Vec<Field>, cp: &ConstantPool) -> Result<usize, Error> {
        fields.iter().fold(self.write_u16(fields.len() as u16), |acc, x| {
            match acc {
                Ok(ctr) => self.write_field(x, cp).map(|c| c + ctr),
                err@_ => err
            }
        })
    }

    fn write_field(&mut self, field: &Field, cp: &ConstantPool) -> Result<usize, Error> {
        self.write_access_flags(&field.access_flags)
            .and(self.write_constant_pool_index(&field.name_index))
            .and(self.write_constant_pool_index(&field.descriptor_index))
            .and(self.write_attributes(&field.attributes, cp))
    }

    fn write_methods(&mut self, methods: &Vec<Method>, cp: &ConstantPool) -> Result<usize, Error> {
        methods.iter().fold(self.write_u16(methods.len() as u16), |acc, x| {
            match acc {
                Ok(ctr) => self.write_method(x, cp).map(|c| c + ctr),
                err@_ => err
            }
        })
    }

    fn write_method(&mut self, method: &Method, cp: &ConstantPool) -> Result<usize, Error> {
        self.write_access_flags(&method.access_flags)
            .and(self.write_constant_pool_index(&method.name_index))
            .and(self.write_constant_pool_index(&method.descriptor_index))
            .and(self.write_attributes(&method.attributes, cp))
    }

    fn write_attributes(&mut self, attributes: &Vec<Attribute>, cp: &ConstantPool) -> Result<usize, Error> {
        attributes.iter().fold(self.write_u16(attributes.len() as u16), |acc, x| {
            match acc {
                Ok(ctr) => self.write_attribute(x, cp).map(|c| c + ctr),
                err@_ => err
            }
        })
    }

    fn write_attribute(&mut self, attribute: &Attribute, cp: &ConstantPool) -> Result<usize, Error> {
        match attribute {
            &Attribute::RawAttribute { name_index: ref n_idx, info: ref bytes } => self.write_u16(n_idx.idx as u16).and(self.write_u32(bytes.len() as u32)).and(self.write_n(bytes)),
            &Attribute::ConstantValue(ref idx) => self.write_u16(cp.get_utf8_index("ConstantValue") as u16).and(self.write_u32(2)).and(self.write_u16(idx.idx as u16)),
            &Attribute::Code { max_stack, max_locals, ref code, ref exception_table, ref attributes } => {
                let mut target: Vec<u8> = vec![];

                {
                    let mut code_writer = ClassWriter::new(&mut target);

                    let _ = code_writer.write_u16(max_stack)
                    .and(code_writer.write_u16(max_locals))
                    .and(code_writer.write_instructions(code))
                    .and(code_writer.write_exception_handlers(exception_table))
                    .and(code_writer.write_attributes(attributes, cp));
                }

                self.write_u16(cp.get_utf8_index("Code") as u16)
                .and(self.write_u32(target.len() as u32))
                .and(self.write_n(&target))
            },
            &Attribute::StackMapTable(ref table) => self.write_stack_map_table(table, cp),
            &Attribute::Exceptions(ref table) => self.write_u16(cp.get_utf8_index("Exceptions") as u16).and(self.write_u32(2 + (table.len() as u32) * 2)).and(self.write_u16(table.len() as u16)).and(table.iter().fold(Ok(0), |_, x| self.write_u16(x.idx as u16))),
            &Attribute::InnerClasses(ref table) => self.write_u16(cp.get_utf8_index("InnerClasses") as u16).and(self.write_u32(2 + (table.len() as u32) * 8)).and(self.write_u16(table.len() as u16)).and(table.iter().fold(Ok(0), |_, x| {
                self.write_u16(x.inner_class_info_index.idx as u16)
                .and(self.write_u16(x.outer_class_info_index.idx as u16))
                .and(self.write_u16(x.inner_name_index.idx as u16))
                .and(self.write_u16(x.access_flags.flags))
            })),
            &Attribute::EnclosingMethod { ref class_index, ref method_index } => self.write_u16(cp.get_utf8_index("EnclosingMethod") as u16).and(self.write_u32(4)).and(self.write_u16(class_index.idx as u16)).and(self.write_u16(method_index.idx as u16)),
            &Attribute::Synthetic => self.write_u16(cp.get_utf8_index("Synthetic") as u16).and(self.write_u32(0)),
            &Attribute::Signature(ref idx) => self.write_u16(cp.get_utf8_index("Signature") as u16).and(self.write_u32(2)).and(self.write_u16(idx.idx as u16)),
            &Attribute::SourceFile(ref idx) => self.write_u16(cp.get_utf8_index("SourceFile") as u16).and(self.write_u32(2)).and(self.write_u16(idx.idx as u16)),
            &Attribute::SourceDebugExtension(ref vec) => self.write_u16(cp.get_utf8_index("SourceDebugExtension") as u16).and(self.write_u32(vec.len() as u32)).and(self.write_n(vec)),
            &Attribute::LineNumberTable(ref table) => self.write_u16(cp.get_utf8_index("LineNumberTable") as u16).and(self.write_u32(2 + table.len() as u32 * 4)).and(self.write_u16(table.len() as u16)).and(table.iter().fold(Ok(0), |_, x| {
                self.write_u16(x.start_pc).and(self.write_u16(x.line_number))
            })),
            &Attribute::LocalVariableTable(ref table) => self.write_u16(cp.get_utf8_index("LocalVariableTable") as u16).and(self.write_u32(2 + table.len() as u32 * 10)).and(self.write_u16(table.len() as u16)).and(table.iter().fold(Ok(0), |_, x| {
                self.write_u16(x.start_pc)
                .and(self.write_u16(x.length))
                .and(self.write_u16(x.name_index.idx as u16))
                .and(self.write_u16(x.descriptor_index.idx as u16))
                .and(self.write_u16(x.index))
            })),
            &Attribute::LocalVariableTypeTable(ref table) => self.write_u16(cp.get_utf8_index("LocalVariableTypeTable") as u16).and(self.write_u32(2 + table.len() as u32 * 10)).and(self.write_u16(table.len() as u16)).and(table.iter().fold(Ok(0), |_, x| {
                self.write_u16(x.start_pc)
                .and(self.write_u16(x.length))
                .and(self.write_u16(x.name_index.idx as u16))
                .and(self.write_u16(x.signature_index.idx as u16))
                .and(self.write_u16(x.index))
            })),
            &Attribute::Deprecated => self.write_u16(cp.get_utf8_index("Deprecated") as u16).and(self.write_u32(0)),
            &Attribute::RuntimeVisibleAnnotations(ref table) => {
                self.write_u16(cp.get_utf8_index("RuntimeVisibleAnnotations") as u16)
                // attribute_length
                .and(self.write_u32(table.iter().fold(2, |acc, x| acc + x.len() as u32)))
                // num_annotations
                .and(self.write_u16(table.len() as u16))
                // annotations
                .and(table.iter().fold(Ok(0), |_, x| self.write_annotation(x, cp)))
            },
            &Attribute::RuntimeInvisibleAnnotations(ref table) => {
                self.write_u16(cp.get_utf8_index("RuntimeInvisibleAnnotations") as u16)
                // attribute_length
                .and(self.write_u32(table.iter().fold(2, |acc, x| acc + x.len() as u32)))
                // num_annotations
                .and(self.write_u16(table.len() as u16))
                // annotations
                .and(table.iter().fold(Ok(0), |_, x| self.write_annotation(x, cp)))
            },
            &Attribute::RuntimeVisibleParameterAnnotations(ref table) => {
                self.write_u16(cp.get_utf8_index("RuntimeVisibleParameterAnnotations") as u16)
                // attribute_length
                .and(self.write_u32(table.iter().fold(1, |acc, x| acc + x.iter().fold(2, |acc2, x2| acc2 + x2.len()) as u32)))
                // num_parameters
                .and(self.write_u8(table.len() as u8))
                // parameter_annotations
                .and(table.iter().fold(Ok(0), |_, ann_table| self.write_u16(ann_table.len() as u16).and(ann_table.iter().fold(Ok(0), |_, ann| self.write_annotation(ann, cp)))))
            },
            &Attribute::RuntimeInvisibleParameterAnnotations(ref table) => {
                self.write_u16(cp.get_utf8_index("RuntimeInvisibleParameterAnnotations") as u16)
                // attribute_length
                .and(self.write_u32(table.iter().fold(1, |acc, x| acc + x.iter().fold(2, |acc2, x2| acc2 + x2.len()) as u32)))
                // num_parameters
                .and(self.write_u8(table.len() as u8))
                // parameter_annotations
                .and(table.iter().fold(Ok(0), |_, ann_table| self.write_u16(ann_table.len() as u16).and(ann_table.iter().fold(Ok(0), |_, ann| self.write_annotation(ann, cp)))))
            },
            &Attribute::RuntimeVisibleTypeAnnotations(ref table) => {
                self.write_u16(cp.get_utf8_index("RuntimeVisibleTypeAnnotations") as u16)
                // attribute_length
                .and(self.write_u32(table.iter().fold(2, |acc, x| acc + x.len() as u32)))
                // num_annotations
                .and(self.write_u16(table.len() as u16))
                // annotations
                .and(table.iter().fold(Ok(0), |_, x| self.write_type_annotation(x, cp)))
            },
            &Attribute::RuntimeInvisibleTypeAnnotations(ref table) => {
                self.write_u16(cp.get_utf8_index("RuntimeInvisibleTypeAnnotations") as u16)
                // attribute_length
                .and(self.write_u32(table.iter().fold(2, |acc, x| acc + x.len() as u32)))
                // num_annotations
                .and(self.write_u16(table.len() as u16))
                // annotations
                .and(table.iter().fold(Ok(0), |_, x| self.write_type_annotation(x, cp)))
            },
            &Attribute::AnnotationDefault(ref value) => {
                self.write_u16(cp.get_utf8_index("AnnotationDefault") as u16)
                .and(self.write_u32(value.len() as u32))
                .and(self.write_element_value(value, cp))
            },
            &Attribute::BootstrapMethods(ref table) => {
                self.write_u16(cp.get_utf8_index("BootstrapMethods") as u16)
                // attribute_length
                .and(self.write_u32(table.iter().fold(2, |acc, method| acc + 4 + method.bootstrap_arguments.len() as u32 * 2)))
                // num_bootstrap_methods
                .and(self.write_u16(table.len() as u16))
                // bootstrap_methods
                .and(table.iter().fold(Ok(0), |_, method| {
                    // bootstrap_method_ref
                    self.write_u16(method.bootstrap_method_ref.idx as u16)
                    // num_bootstrap_arguments
                    .and(self.write_u16(method.bootstrap_arguments.len() as u16))
                    // bootstrap_arguments
                    .and(method.bootstrap_arguments.iter().fold(Ok(0), |_, arg| self.write_u16(arg.idx as u16)))
                }))
            },
            &Attribute::MethodParameters(ref table) => {
                self.write_u16(cp.get_utf8_index("MethodParameters") as u16)
                .and(self.write_u32(1 + table.len() as u32 * 4))
                .and(table.iter().fold(Ok(0), |_, p| self.write_u16(p.name_index.idx as u16).and(self.write_u16(p.access_flags.flags as u16))))
            }
        }
    }

    fn write_stack_map_table(&mut self, table: &Vec<StackMapFrame>, cp: &ConstantPool) -> Result<usize, Error> {
        // attribute_name_index
        self.write_u16(cp.get_utf8_index("StackMapTable") as u16)
            // attribute_length = number_of_entries length (2) + sum of entries' length
            .and(self.write_u32(2 + table.iter().map(|st| st.len()).fold(0, |acc, x| acc + x) as u32))
            // number_of_entries
            .and(self.write_u16(table.len() as u16))
            // entries
            .and(table.iter().fold(Ok(0), |_, x| {
                match x {
                    &StackMapFrame::SameFrame { tag } => self.write_u8(tag),
                    &StackMapFrame::SameLocals1StackItemFrame{ tag, ref stack } => self.write_u8(tag).and(self.write_verification_type(stack)),
                    &StackMapFrame::SameLocals1StackItemFrameExtended { offset_delta, ref stack } => self.write_u8(247).and(self.write_u16(offset_delta)).and(self.write_verification_type(stack)),
                    &StackMapFrame::ChopFrame { tag, offset_delta } => self.write_u8(tag).and(self.write_u16(offset_delta)),
                    &StackMapFrame::SameFrameExtended { offset_delta } => self.write_u8(251).and(self.write_u16(offset_delta)),
                    &StackMapFrame::AppendFrame { tag, offset_delta, ref locals } => self.write_u8(tag).and(self.write_u16(offset_delta)).and(locals.iter().fold(Ok(0), |_, x| self.write_verification_type(x))),
                    &StackMapFrame::FullFrame { offset_delta, ref locals, ref stack } => {
                        // full frame tag
                        self.write_u8(255)
                            // offset_delta
                            .and(self.write_u16(offset_delta))
                            // number_of_locals
                            .and(self.write_u16(locals.len() as u16))
                            // locals
                            .and(locals.iter().fold(Ok(0), |_, x| self.write_verification_type(x)))
                            // number_of_stack_items
                            .and(self.write_u16(stack.len() as u16))
                            // stack
                            .and(stack.iter().fold(Ok(0), |_, x| self.write_verification_type(x)))
                    },
                    &StackMapFrame::FutureUse { tag } => self.write_u8(tag)
                }
            }))
    }

    fn write_verification_type(&mut self, info: &VerificationType) -> Result<usize, Error> {
        match info {
            &VerificationType::Top => self.write_u8(0),
            &VerificationType::Integer => self.write_u8(1),
            &VerificationType::Float => self.write_u8(2),
            &VerificationType::Long => self.write_u8(4),
            &VerificationType::Double => self.write_u8(3),
            &VerificationType::Null => self.write_u8(5),
            &VerificationType::UninitializedThis => self.write_u8(6),
            &VerificationType::Object { ref cpool_index } => self.write_u8(7).and(self.write_u16(cpool_index.idx as u16)),
            &VerificationType::Uninitialized { offset } => self.write_u8(8).and(self.write_u16(offset))
        }
    }

    fn write_element_value(&mut self, element_value: &ElementValue, cp: &ConstantPool) -> Result<usize, Error> {
        match element_value {
            &ElementValue::ConstantValue(tag, ref idx) => self.write_u8(tag).and(self.write_u16(idx.idx as u16)),
            &ElementValue::Enum { ref type_name_index, ref const_name_index } => self.write_u8(101).and(self.write_u16(type_name_index.idx as u16)).and(self.write_u16(const_name_index.idx as u16)),
            &ElementValue::ClassInfo(ref idx) => self.write_u8(99).and(self.write_u16(idx.idx as u16)),
            &ElementValue::Annotation(ref annotation) => self.write_u8(64).and(self.write_annotation(annotation, cp)),
            &ElementValue::Array(ref table) => self.write_u8(91).and(self.write_u16(table.len() as u16)).and(table.iter().fold(Ok(0), |_, x| { self.write_element_value(x, cp) }))
        }
    }

    fn write_element_value_pair(&mut self, pair: &ElementValuePair, cp: &ConstantPool) -> Result<usize, Error> {
        self.write_u16(pair.element_name_index.idx as u16).and(self.write_element_value(&pair.value, cp))
    }

    fn write_annotation(&mut self, annotation: &Annotation, cp: &ConstantPool) -> Result<usize, Error> {
        // type_index
        self.write_u16(annotation.type_index.idx as u16)
        // num_element_value_pairs
        .and(self.write_u16(annotation.element_value_pairs.len() as u16))
        // element_value_pairs
        .and(annotation.element_value_pairs.iter().fold(Ok(0), |_, x| self.write_element_value_pair(x, cp)))
    }

    fn write_type_annotation(&mut self, annotation: &TypeAnnotation, cp: &ConstantPool) -> Result<usize, Error> {
        // target_type
        self.write_u8(annotation.target_info.subtype())
        // target_info
        .and({
            match &annotation.target_info {
                &TargetInfo::TypeParameter { subtype: _, idx } => self.write_u8(idx),
                &TargetInfo::SuperType { idx } => self.write_u16(idx),
                &TargetInfo::TypeParameterBound { subtype: _, param_idx, bound_index } => self.write_u8(param_idx).and(self.write_u8(bound_index)),
                &TargetInfo::Empty { subtype: _ } => Ok(0),
                &TargetInfo::MethodFormalParameter { idx } => self.write_u8(idx),
                &TargetInfo::Throws { idx } => self.write_u16(idx),
                &TargetInfo::LocalVar { subtype: _, ref target } => self.write_u16(target.len() as u16).and(target.iter().fold(Ok(0), |_, x| self.write_u16(x.0).and(self.write_u16(x.1)).and(self.write_u16(x.2)))),
                &TargetInfo::Catch { idx } => self.write_u16(idx),
                &TargetInfo::Offset { subtype: _, idx } => self.write_u16(idx),
                &TargetInfo::TypeArgument { subtype: _, offset, type_arg_idx } => self.write_u16(offset).and(self.write_u8(type_arg_idx))
            }
        })
        .and({
            // path_length
            self.write_u8(annotation.target_path.path.len() as u8)
            // path
            .and(annotation.target_path.path.iter().fold(Ok(0), |_, x| self.write_u8(x.0.value()).and(self.write_u8(x.1))))
        })
        .and(self.write_u16(annotation.type_index.idx as u16))
        .and(self.write_u16(annotation.element_value_pairs.len() as u16))
        .and(annotation.element_value_pairs.iter().fold(Ok(0), |_, x| self.write_element_value_pair(x, cp)))
    }

    fn write_instructions(&mut self, instructions: &Vec<Instruction>) -> Result<usize, Error> {
        let mut target: Vec<u8> = vec![];
        let _ /*written_bytes*/ = {
            let mut instr_writer = ClassWriter::new(&mut target);

            instructions.iter().fold(0 as usize, |counter, instr| {
                counter + instr_writer.render_instruction(instr, counter)
            })
        };

        self.write_u32(target.len() as u32).and_then(|x| self.write_n(&target).map(|y| x + y))
    }

    /// Renders a single instruction into the output stream
    fn render_instruction(&mut self, instruction: &Instruction, offset: usize) -> usize {
        match instruction {
            &Instruction::AALOAD => self.write_u8(0x32),
            &Instruction::AASTORE => self.write_u8(0x53),
            &Instruction::ACONST_NULL => self.write_u8(0x01),
            &Instruction::ALOAD(value) => self.write_u8(0x19).and(self.write_u8(value)).and(Ok(2)),
            &Instruction::ALOAD_0 => self.write_u8(0x2a),
            &Instruction::ALOAD_1 => self.write_u8(0x2b),
            &Instruction::ALOAD_2 => self.write_u8(0x2c),
            &Instruction::ALOAD_3 => self.write_u8(0x2d),
            &Instruction::ANEWARRAY(b) => self.write_u8(0xbd).and(self.write_u16(b)).and(Ok(3)),
            &Instruction::ARETURN => self.write_u8(0xb0),
            &Instruction::ARRAYLENGTH => self.write_u8(0xbe),
            &Instruction::ASTORE(value) => self.write_u8(0x3a).and(self.write_u8(value)).and(Ok(2)),
            &Instruction::ASTORE_0 => self.write_u8(0x4b),
            &Instruction::ASTORE_1 => self.write_u8(0x4c),
            &Instruction::ASTORE_2 => self.write_u8(0x4d),
            &Instruction::ASTORE_3 => self.write_u8(0x4e),
            &Instruction::ATHROW => self.write_u8(0xbf),
            &Instruction::BALOAD => self.write_u8(0x33),
            &Instruction::BASTORE => self.write_u8(0x54),
            &Instruction::BIPUSH(value) => self.write_u8(0x10).and(self.write_u8(value)).and(Ok(2)),
            &Instruction::CALOAD => self.write_u8(0x34),
            &Instruction::CASTORE => self.write_u8(0x55),
            &Instruction::CHECKCAST(value) => self.write_u8(0xc0).and(self.write_u16(value)).and(Ok(3)),
            &Instruction::D2F => self.write_u8(0x90),
            &Instruction::D2I => self.write_u8(0x8e),
            &Instruction::D2L => self.write_u8(0x8f),
            &Instruction::DADD => self.write_u8(0x63),
            &Instruction::DALOAD => self.write_u8(0x31),
            &Instruction::DASTORE => self.write_u8(0x52),
            &Instruction::DCMPL => self.write_u8(0x97),
            &Instruction::DCMPG => self.write_u8(0x98),
            &Instruction::DCONST_0 => self.write_u8(0x0e),
            &Instruction::DCONST_1 => self.write_u8(0x0f),
            &Instruction::DDIV => self.write_u8(0x6f),
            &Instruction::DLOAD(value) => self.write_u8(0x18).and(self.write_u8(value)).and(Ok(2)),
            &Instruction::DLOAD_0 => self.write_u8(0x26),
            &Instruction::DLOAD_1 => self.write_u8(0x27),
            &Instruction::DLOAD_2 => self.write_u8(0x28),
            &Instruction::DLOAD_3 => self.write_u8(0x29),
            &Instruction::DMUL => self.write_u8(0x6b),
            &Instruction::DNEG => self.write_u8(0x77),
            &Instruction::DREM => self.write_u8(0x73),
            &Instruction::DRETURN => self.write_u8(0xaf),
            &Instruction::DSTORE(value) => self.write_u8(0x39).and(self.write_u8(value)).and(Ok(2)),
            &Instruction::DSTORE_0 => self.write_u8(0x47),
            &Instruction::DSTORE_1 => self.write_u8(0x48),
            &Instruction::DSTORE_2 => self.write_u8(0x49),
            &Instruction::DSTORE_3 => self.write_u8(0x4a),
            &Instruction::DSUB => self.write_u8(0x67),
            &Instruction::DUP => self.write_u8(0x59),
            &Instruction::DUP_X1 => self.write_u8(0x5a),
            &Instruction::DUP_X2 => self.write_u8(0x5b),
            &Instruction::DUP2 => self.write_u8(0x5c),
            &Instruction::DUP2_X1 => self.write_u8(0x5d),
            &Instruction::DUP2_X2 => self.write_u8(0x5e),
            &Instruction::F2D => self.write_u8(0x8d),
            &Instruction::F2I => self.write_u8(0x8b),
            &Instruction::F2L => self.write_u8(0x8c),
            &Instruction::FADD => self.write_u8(0x62),
            &Instruction::FALOAD => self.write_u8(0x30),
            &Instruction::FASTORE => self.write_u8(0x51),
            &Instruction::FCMPL => self.write_u8(0x95),
            &Instruction::FCMPG => self.write_u8(0x96),
            &Instruction::FCONST_0 => self.write_u8(0x0b),
            &Instruction::FCONST_1 => self.write_u8(0x0c),
            &Instruction::FCONST_2 => self.write_u8(0x0d),
            &Instruction::FDIV => self.write_u8(0x6e),
            &Instruction::FLOAD(value) => self.write_u8(0x17).and(self.write_u8(value)).and(Ok(2)),
            &Instruction::FLOAD_0 => self.write_u8(0x22),
            &Instruction::FLOAD_1 => self.write_u8(0x23),
            &Instruction::FLOAD_2 => self.write_u8(0x24),
            &Instruction::FLOAD_3 => self.write_u8(0x25),
            &Instruction::FMUL => self.write_u8(0x6a),
            &Instruction::FNEG => self.write_u8(0x76),
            &Instruction::FREM => self.write_u8(0x72),
            &Instruction::FRETURN => self.write_u8(0xae),
            &Instruction::FSTORE(value) => self.write_u8(0x38).and(self.write_u8(value)).and(Ok(2)),
            &Instruction::FSTORE_0 => self.write_u8(0x43),
            &Instruction::FSTORE_1 => self.write_u8(0x44),
            &Instruction::FSTORE_2 => self.write_u8(0x45),
            &Instruction::FSTORE_3 => self.write_u8(0x46),
            &Instruction::FSUB => self.write_u8(0x66),
            &Instruction::GETFIELD(value) => self.write_u8(0xb4).and(self.write_u16(value)).and(Ok(3)),
            &Instruction::GETSTATIC(value) => self.write_u8(0xb2).and(self.write_u16(value)).and(Ok(3)),
            &Instruction::GOTO(value) => self.write_u8(0xa7).and(self.write_u16(value as u16)).and(Ok(3)),
            &Instruction::GOTO_W(value) => self.write_u8(0xc8).and(self.write_u32(value as u32)).and(Ok(5)),
            &Instruction::I2B => self.write_u8(0x91),
            &Instruction::I2C => self.write_u8(0x92),
            &Instruction::I2D => self.write_u8(0x87),
            &Instruction::I2F => self.write_u8(0x86),
            &Instruction::I2L => self.write_u8(0x85),
            &Instruction::I2S => self.write_u8(0x93),
            &Instruction::IADD => self.write_u8(0x60),
            &Instruction::IALOAD => self.write_u8(0x2e),
            &Instruction::IAND => self.write_u8(0x7e),
            &Instruction::IASTORE => self.write_u8(0x4f),
            &Instruction::ICONST_M1 => self.write_u8(0x02),
            &Instruction::ICONST_0 => self.write_u8(0x03),
            &Instruction::ICONST_1 => self.write_u8(0x04),
            &Instruction::ICONST_2 => self.write_u8(0x05),
            &Instruction::ICONST_3 => self.write_u8(0x06),
            &Instruction::ICONST_4 => self.write_u8(0x07),
            &Instruction::ICONST_5 => self.write_u8(0x08),
            &Instruction::IDIV => self.write_u8(0x6c),
            &Instruction::IF_ACMPEQ(value) => self.write_u8(0xa5).and(self.write_u16(value as u16)).and(Ok(3)),
            &Instruction::IF_ACMPNE(value) => self.write_u8(0xa6).and(self.write_u16(value as u16)).and(Ok(3)),
            &Instruction::IF_ICMPEQ(value) => self.write_u8(0x9f).and(self.write_u16(value as u16)).and(Ok(3)),
            &Instruction::IF_ICMPNE(value) => self.write_u8(0xa0).and(self.write_u16(value as u16)).and(Ok(3)),
            &Instruction::IF_ICMPLT(value) => self.write_u8(0xa1).and(self.write_u16(value as u16)).and(Ok(3)),
            &Instruction::IF_ICMPGE(value) => self.write_u8(0xa2).and(self.write_u16(value as u16)).and(Ok(3)),
            &Instruction::IF_ICMPGT(value) => self.write_u8(0xa3).and(self.write_u16(value as u16)).and(Ok(3)),
            &Instruction::IF_ICMPLE(value) => self.write_u8(0xa4).and(self.write_u16(value as u16)).and(Ok(3)),
            &Instruction::IFEQ(value) => self.write_u8(0x99).and(self.write_u16(value as u16)).and(Ok(3)),
            &Instruction::IFNE(value) => self.write_u8(0x9a).and(self.write_u16(value as u16)).and(Ok(3)),
            &Instruction::IFLT(value) => self.write_u8(0x9b).and(self.write_u16(value as u16)).and(Ok(3)),
            &Instruction::IFGE(value) => self.write_u8(0x9c).and(self.write_u16(value as u16)).and(Ok(3)),
            &Instruction::IFGT(value) => self.write_u8(0x9d).and(self.write_u16(value as u16)).and(Ok(3)),
            &Instruction::IFLE(value) => self.write_u8(0x9e).and(self.write_u16(value as u16)).and(Ok(3)),
            &Instruction::IFNONNULL(value) => self.write_u8(0xc7).and(self.write_u16(value as u16)).and(Ok(3)),
            &Instruction::IFNULL(value) => self.write_u8(0xc6).and(self.write_u16(value as u16)).and(Ok(3)),
            &Instruction::IINC(a, b) => self.write_u8(0x84).and(self.write_u8(a)).and(self.write_u8(b as u8)).and(Ok(3)),
            &Instruction::ILOAD(value) => self.write_u8(0x15).and(self.write_u8(value)).and(Ok(2)),
            &Instruction::ILOAD_0 => self.write_u8(0x1a),
            &Instruction::ILOAD_1 => self.write_u8(0x1b),
            &Instruction::ILOAD_2 => self.write_u8(0x1c),
            &Instruction::ILOAD_3 => self.write_u8(0x1d),
            &Instruction::IMUL => self.write_u8(0x68),
            &Instruction::INEG => self.write_u8(0x74),
            &Instruction::INSTANCEOF(value) => self.write_u8(0xc1).and(self.write_u16(value)).and(Ok(3)),
            &Instruction::INVOKEDYNAMIC(value) => self.write_u8(0xba).and(self.write_u16(value)).and(self.write_u16(0)).and(Ok(5)),
            &Instruction::INVOKEINTERFACE(a, b) => self.write_u8(0xb9).and(self.write_u16(a)).and(self.write_u8(b)).and(self.write_u8(0)).and(Ok(5)),
            &Instruction::INVOKESPECIAL(value) => self.write_u8(0xb7).and(self.write_u16(value)).and(Ok(3)),
            &Instruction::INVOKESTATIC(value) => self.write_u8(0xb8).and(self.write_u16(value)).and(Ok(3)),
            &Instruction::INVOKEVIRTUAL(value) => self.write_u8(0xb6).and(self.write_u16(value)).and(Ok(3)),
            &Instruction::IOR => self.write_u8(0x80),
            &Instruction::IREM => self.write_u8(0x70),
            &Instruction::IRETURN => self.write_u8(0xac),
            &Instruction::ISHL => self.write_u8(0x78),
            &Instruction::ISHR => self.write_u8(0x7a),
            &Instruction::ISTORE(value) => self.write_u8(0x36).and(self.write_u8(value)).and(Ok(2)),
            &Instruction::ISTORE_0 => self.write_u8(0x3b),
            &Instruction::ISTORE_1 => self.write_u8(0x3c),
            &Instruction::ISTORE_2 => self.write_u8(0x3d),
            &Instruction::ISTORE_3 => self.write_u8(0x3e),
            &Instruction::ISUB => self.write_u8(0x64),
            &Instruction::IUSHR => self.write_u8(0x7c),
            &Instruction::IXOR => self.write_u8(0x82),
            &Instruction::JSR(value) => self.write_u8(0xa8).and(self.write_u16(value as u16)).and(Ok(3)),
            &Instruction::JSR_W(value) => self.write_u8(0xc9).and(self.write_u32(value as u32)).and(Ok(5)),
            &Instruction::L2D => self.write_u8(0x8a),
            &Instruction::L2F => self.write_u8(0x89),
            &Instruction::L2I => self.write_u8(0x88),
            &Instruction::LADD => self.write_u8(0x61),
            &Instruction::LALOAD => self.write_u8(0x2f),
            &Instruction::LAND => self.write_u8(0x7f),
            &Instruction::LASTORE => self.write_u8(0x50),
            &Instruction::LCMP => self.write_u8(0x94),
            &Instruction::LCONST_0 => self.write_u8(0x09),
            &Instruction::LCONST_1 => self.write_u8(0x0a),
            &Instruction::LDC(value) => self.write_u8(0x12).and(self.write_u8(value)).and(Ok(2)),
            &Instruction::LDC_W(value) => self.write_u8(0x13).and(self.write_u16(value)).and(Ok(3)),
            &Instruction::LDC2_W(value) => self.write_u8(0x14).and(self.write_u16(value)).and(Ok(3)),
            &Instruction::LDIV => self.write_u8(0x6d),
            &Instruction::LLOAD(value) => self.write_u8(0x16).and(self.write_u8(value)).and(Ok(2)),
            &Instruction::LLOAD_0 => self.write_u8(0x1e),
            &Instruction::LLOAD_1 => self.write_u8(0x1f),
            &Instruction::LLOAD_2 => self.write_u8(0x20),
            &Instruction::LLOAD_3 => self.write_u8(0x21),
            &Instruction::LMUL => self.write_u8(0x69),
            &Instruction::LNEG => self.write_u8(0x75),
            &Instruction::LOOKUPSWITCH(a, ref l) => {
                let _ = self.write_u8(0xab);

                let padding = (4 - ((offset + 1) % 4)) % 4;


                for _ in 0..padding {
                    let _ = self.write_u8(0);
                }

                let _ = self.write_u32(a as u32);
                let _ = self.write_u32(l.len() as u32);

                for &(p1, p2) in l {
                    let _ = self.write_u32(p1 as u32);
                    let _ = self.write_u32(p2 as u32);
                }

                let len = 9 + padding + l.len() * 8;

                Ok(len)
            },
            &Instruction::LOR => self.write_u8(0x81),
            &Instruction::LREM => self.write_u8(0x71),
            &Instruction::LRETURN => self.write_u8(0xad),
            &Instruction::LSHL => self.write_u8(0x79),
            &Instruction::LSHR => self.write_u8(0x7b),
            &Instruction::LSTORE(value) => self.write_u8(0x37).and(self.write_u8(value)).and(Ok(2)),
            &Instruction::LSTORE_0 => self.write_u8(0x3f),
            &Instruction::LSTORE_1 => self.write_u8(0x40),
            &Instruction::LSTORE_2 => self.write_u8(0x41),
            &Instruction::LSTORE_3 => self.write_u8(0x42),
            &Instruction::LSUB => self.write_u8(0x65),
            &Instruction::LUSHR => self.write_u8(0x7d),
            &Instruction::LXOR => self.write_u8(0x83),
            &Instruction::MONITORENTER => self.write_u8(0xc2),
            &Instruction::MONITOREXIT => self.write_u8(0xc3),
            &Instruction::MULTIANEWARRAY(a, b) => self.write_u8(0xc5).and(self.write_u16(a)).and(self.write_u8(b)).and(Ok(4)),
            &Instruction::NEW(value) => self.write_u8(0xbb).and(self.write_u16(value)).and(Ok(3)),
            &Instruction::NEWARRAY(value) => self.write_u8(0xbc).and(self.write_u8(value)).and(Ok(2)),
            &Instruction::NOP => self.write_u8(0x00),
            &Instruction::POP => self.write_u8(0x57),
            &Instruction::POP2 => self.write_u8(0x58),
            &Instruction::PUTFIELD(value) => self.write_u8(0xb5).and(self.write_u16(value)).and(Ok(3)),
            &Instruction::PUTSTATIC(value) => self.write_u8(0xb3).and(self.write_u16(value)).and(Ok(3)),
            &Instruction::RET(value) => self.write_u8(0xa9).and(self.write_u8(value)).and(Ok(2)),
            &Instruction::RETURN => self.write_u8(0xb1),
            &Instruction::SALOAD => self.write_u8(0x35),
            &Instruction::SASTORE => self.write_u8(0x56),
            &Instruction::SIPUSH(value) => self.write_u8(0x11).and(self.write_u16(value)).and(Ok(3)),
            &Instruction::SWAP => self.write_u8(0x5f),
            //TABLESWITCH(i32, i32, i32, Vec<i32>),
            &Instruction::TABLESWITCH(a, b, c, ref d) => {
                let _ = self.write_u8(0xaa);

                let padding = (4 - ((offset + 1) % 4)) % 4;

                for _ in 0..padding {
                    let _ = self.write_u8(0);
                }

                let _ = self.write_u32(a as u32);
                let _ = self.write_u32(b as u32);
                let _ = self.write_u32(c as u32);

                for &v in d {
                    let _ = self.write_u32(v as u32);
                }

                Ok(13 + padding + d.len() * 4)
            },
            &Instruction::ILOAD_W(value) => self.write_u16(0xc415).and(self.write_u16(value)).and(Ok(4)),
            &Instruction::FLOAD_W(value) => self.write_u16(0xc417).and(self.write_u16(value)).and(Ok(4)),
            &Instruction::ALOAD_W(value) => self.write_u16(0xc419).and(self.write_u16(value)).and(Ok(4)),
            &Instruction::LLOAD_W(value) => self.write_u16(0xc416).and(self.write_u16(value)).and(Ok(4)),
            &Instruction::DLOAD_W(value) => self.write_u16(0xc418).and(self.write_u16(value)).and(Ok(4)),
            &Instruction::ISTORE_W(value) => self.write_u16(0xc436).and(self.write_u16(value)).and(Ok(4)),
            &Instruction::FSTORE_W(value) => self.write_u16(0xc438).and(self.write_u16(value)).and(Ok(4)),
            &Instruction::ASTORE_W(value) => self.write_u16(0xc43a).and(self.write_u16(value)).and(Ok(4)),
            &Instruction::LSTORE_W(value) => self.write_u16(0xc437).and(self.write_u16(value)).and(Ok(4)),
            &Instruction::DSTORE_W(value) => self.write_u16(0xc439).and(self.write_u16(value)).and(Ok(4)),
            &Instruction::RET_W(value) => self.write_u16(0xc4a9).and(self.write_u16(value)).and(Ok(4)),
            &Instruction::IINC_W(a, b) => self.write_u16(0xc484).and(self.write_u16(a)).and(self.write_u16(b as u16)).and(Ok(6)),
            _ => self.write_u8(0xFF)
        }.ok().unwrap_or(0)
    }

    fn write_exception_handlers(&mut self, exception_table: &Vec<ExceptionHandler>) -> Result<usize, Error> {
        self.write_u16(exception_table.len() as u16)
            .and(exception_table.iter().fold(Ok(0), |_, x| {
                self.write_u16(x.start_pc)
                .and(self.write_u16(x.end_pc))
                .and(self.write_u16(x.handler_pc))
                .and(self.write_u16(x.catch_type.idx as u16))
            }))
    }

    pub fn write_n(&mut self, bytes: &Vec<u8>) -> Result<usize, Error> {
        bytes.iter().fold(Ok(0), |acc, x| match acc {
            Ok(ctr) => self.write_u8(*x).map(|c| c + ctr),
            err@_ => err
        })
    }

    pub fn write_u64(&mut self, value: u64) -> Result<usize, Error> {
        let buf: [u8; 8] = [
            ((value & 0xFF << 56) >> 56) as u8,
            ((value & 0xFF << 48) >> 48) as u8,
            ((value & 0xFF << 40) >> 40) as u8,
            ((value & 0xFF << 32) >> 32) as u8,
            ((value & 0xFF << 24) >> 24) as u8,
            ((value & 0xFF << 16) >> 16) as u8,
            ((value & 0xFF << 8) >> 8) as u8,
            (value & 0xFF) as u8
        ];

        self.target.write(&buf)
    }

    pub fn write_u32(&mut self, value: u32) -> Result<usize, Error> {
        let buf: [u8; 4] = [
            ((value & 0xFF << 24) >> 24) as u8,
            ((value & 0xFF << 16) >> 16) as u8,
            ((value & 0xFF << 8) >> 8) as u8,
            (value & 0xFF) as u8
        ];

        self.target.write(&buf)
    }

    pub fn write_u16(&mut self, value: u16) -> Result<usize, Error> {
        let buf: [u8; 2] = [((value & 0xFF00) >> 8) as u8, (value & 0xFF) as u8];

        self.target.write(&buf)
    }

    pub fn write_u8(&mut self, value: u8) -> Result<usize, Error> {
        self.target.write(&[value])
    }
}
