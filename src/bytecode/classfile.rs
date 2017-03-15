
///
/// A `Classfile` represents a definition of a single JVM class or interface. Unlike the bytecode
/// itself, it doesn't represent every byte in the class definition, though, many information are
/// encoded in the type system instead. This approach may seem restrictive but it helps achieving
/// bytecode safety.
#[derive(Debug)]
pub struct Classfile {
    pub version: ClassfileVersion,
    pub constant_pool: ConstantPool,
    pub access_flags: AccessFlags,
    pub this_class: ConstantPoolIndex,
    pub super_class: ConstantPoolIndex,
    pub interfaces: Vec<ConstantPoolIndex>,
    pub fields: Vec<Field>,
    pub methods: Vec<Method>,
    pub attributes: Vec<Attribute>
}

impl Classfile {
    /// Create a new classfile, initialised with sensible default values
    pub fn new() -> Classfile {
        Classfile::default()
    }
}

impl Default for Classfile {
    fn default() -> Self {
        Classfile {
            version: ClassfileVersion::default(),
            constant_pool: ConstantPool::default(),
            access_flags: AccessFlags::default(),
            this_class: ConstantPoolIndex::default(),
            super_class: ConstantPoolIndex::default(),
            interfaces: vec![],
            fields: vec![],
            methods: vec![],
            attributes: vec![]
        }
    }
}

///
/// Describe a classfile version number.
#[derive(Ord, PartialOrd, Eq, PartialEq, Debug)]
pub struct ClassfileVersion {
    pub minor_version: u16,
    pub major_version: u16
}

impl ClassfileVersion {
    pub fn new(major_version: u16, minor_version: u16) -> ClassfileVersion {
        ClassfileVersion { major_version: major_version, minor_version: minor_version }
    }
}

impl Default for ClassfileVersion {
    fn default() -> Self {
        const DEFAULT_MAJOR_VERSION: u16 = 52;
        const DEFAULT_MINOR_VERSION: u16 = 0;

        ClassfileVersion { major_version: DEFAULT_MAJOR_VERSION, minor_version: DEFAULT_MINOR_VERSION }
    }
}

///
/// A `ConstantPool` is a table of various string and number literal constants that are referred
/// within the substructures of the `Classfile`.
#[derive(Debug)]
pub struct ConstantPool {
    pub constants: Vec<Constant>
}

impl ConstantPool {
    pub fn new(constants: Vec<Constant>) -> ConstantPool {
        ConstantPool {
            constants: constants
        }
    }

    pub fn get_utf8(&self, idx: u16) -> Option<&Vec<u8>> {
        match self.constants.get(idx as usize) {
            Some(constant) => match constant {
                &Constant::Utf8(ref bytes) => Some(bytes),
                _ => None
            },
            _ => None
        }
    }

    pub fn get_utf8_string(&self, idx: u16) -> Option<String> {
        match self.get_utf8(idx) {
            Some(bytes) => match String::from_utf8(bytes.clone()) {
                Ok(string) => Some(string),
                _ => None
            },
            _ => None
        }
    }

    pub fn find_ut8_index(&self, utf8: &'static str) -> Option<usize> {
        for i in 0..self.constants.len() {
            match self.constants[i] {
                Constant::Utf8(ref bytes) => {
                    if bytes.as_slice() == utf8.as_bytes() {
                        return Some(i);
                    }
                },
                _ => ()
            }
        }
        None
    }

    pub fn get_utf8_index(&self, utf8: &'static str) -> usize {
        self.find_ut8_index(utf8).unwrap_or(0)
    }

    pub fn resolve_index(&self, idx: &ConstantPoolIndex) -> Option<&Constant> {
        self.constants.get(idx.idx)
    }

    pub fn cp_len(&self) -> usize {
        //self.constants.iter().fold(0, |acc, x| acc + x.cp_size())
        self.constants.len()
    }
}

impl Default for ConstantPool {
    fn default() -> Self {
        ConstantPool {
            constants: vec![]
        }
    }
}

#[derive(Default, Debug)]
pub struct ConstantPoolIndex {
    pub idx: usize
}

impl ConstantPoolIndex {
    pub fn new(idx: usize) -> Self {
        ConstantPoolIndex { idx: idx }
    }
}

#[derive(Debug)]
pub enum Constant {
    Utf8(Vec<u8>),
    Integer(u32),
    Float(u32),
    Long(u64),
    Double(u64),
    Class(ConstantPoolIndex),
    FieldRef { class_index: ConstantPoolIndex, name_and_type_index: ConstantPoolIndex },
    MethodRef { class_index: ConstantPoolIndex, name_and_type_index: ConstantPoolIndex },
    InterfaceMethodRef { class_index: ConstantPoolIndex, name_and_type_index: ConstantPoolIndex },
    String(ConstantPoolIndex),
    NameAndType { name_index: ConstantPoolIndex, descriptor_index: ConstantPoolIndex },
    MethodHandle { reference_kind: ReferenceKind, reference_index: ConstantPoolIndex },
    MethodType(ConstantPoolIndex),
    InvokeDynamic { bootstrap_method_attr_index: ConstantPoolIndex, name_and_type_index: ConstantPoolIndex },
    Unknown(u8),
    Placeholder
}

impl Constant {
    pub fn cp_size(&self) -> usize {
        match self {
            &Constant::Long(_) => 2,
            &Constant::Double(_) => 2,
            &Constant::Placeholder => 0,
            _ => 1
        }
    }
}

#[derive(Debug)]
pub enum ReferenceKind {
    GetField = 1,
    GetStatic = 2,
    PutField = 3,
    PutStatic = 4,
    InvokeVirtual = 5,
    InvokeStatic = 6,
    InvokeSpecial = 7,
    NewInvokeSpecial = 8,
    InvokeInterface = 9,
    Unknown = 255
}

impl ReferenceKind {
    pub fn from_u8(value: u8) -> ReferenceKind {
        match value {
            1 => ReferenceKind::GetField,
            2 => ReferenceKind::GetStatic,
            3 => ReferenceKind::PutField,
            4 => ReferenceKind::PutStatic,
            5 => ReferenceKind::InvokeVirtual,
            6 => ReferenceKind::InvokeStatic,
            7 => ReferenceKind::InvokeSpecial,
            8 => ReferenceKind::NewInvokeSpecial,
            9 => ReferenceKind::InvokeInterface,
            _ => ReferenceKind::Unknown
        }
    }

    pub fn to_u8(&self) -> u8 {
        match *self {
            ReferenceKind::GetField => 1,
            ReferenceKind::GetStatic => 2,
            ReferenceKind::PutField => 3,
            ReferenceKind::PutStatic => 4,
            ReferenceKind::InvokeVirtual => 5,
            ReferenceKind::InvokeStatic => 6,
            ReferenceKind::InvokeSpecial => 7,
            ReferenceKind::NewInvokeSpecial => 8,
            ReferenceKind::InvokeInterface => 9,
            ReferenceKind::Unknown => 255
        }
    }
}

#[derive(Default, Debug)]
pub struct AccessFlags {
    pub flags: u16
}

impl AccessFlags {
    pub fn new() -> AccessFlags {
        AccessFlags::of(0)
    }

    pub fn of(val: u16) -> AccessFlags {
        AccessFlags { flags: val }
    }

    pub fn has_flag(&self, flag: u16) -> bool {
        self.flags & flag > 0
    }

    pub fn set_flag(&mut self, flag: u16) {
        self.flags |= flag;
    }

    pub fn clear_flag(&mut self, flag: u16) {
        self.flags &= flag ^ 0xFFFF;
    }
}

pub enum ClassAccessFlags {
    Public = 0x0001, // Declared public; may be accessed from outside its package.
    Final = 0x0010, // Declared final; no subclasses allowed.
    Super = 0x0020, // Treat superclass methods specially when invoked by the invokespecial instruction.
    Interface = 0x0200, // Is an interface, not a class.
    Abstract = 0x0400, // Declared abstract; must not be instantiated.
    Synthetic = 0x1000, // Declared synthetic; not present in the source code.
    Annotation = 0x2000, // Declared as an annotation type.
    Enum = 0x4000 // Declared as an enum type.
}

pub enum FieldAccessFlags {
    Public = 0x0001, //	Declared public; may be accessed from outside its package.
    Private = 0x0002, //	Declared private; usable only within the defining class.
    Protected = 0x0004, //	Declared protected; may be accessed within subclasses.
    Static = 0x0008, //	Declared static.
    Final = 0x0010, //	Declared final; never directly assigned to after object construction (JLS §17.5).
    Volatile = 0x0040, //	Declared volatile; cannot be cached.
    Transient = 0x0080, //	Declared transient; not written or read by a persistent object manager.
    Synthetic = 0x1000, //	Declared synthetic; not present in the source code.
    Enum = 0x4000 //	Declared as an element of an enum.
}

pub enum MethodAccessFlags {
    Public = 0x0001, //	Declared public; may be accessed from outside its package.
    Private = 0x0002, //	Declared private; accessible only within the defining class.
    Protected = 0x0004, //	Declared protected; may be accessed within subclasses.
    Static = 0x0008, //	Declared static.
    Final = 0x0010, //	Declared final; must not be overridden (§5.4.5).
    Synchronized = 0x0020, //	Declared synchronized; invocation is wrapped by a monitor use.
    Bridge = 0x0040, //	A bridge method, generated by the compiler.
    Varargs = 0x0080, //	Declared with variable number of arguments.
    Native = 0x0100, //	Declared native; implemented in a language other than Java.
    Abstract = 0x0400, //	Declared abstract; no implementation is provided.
    Strict = 0x0800, //	Declared strictfp; floating-point mode is FP-strict.
    Synthetic = 0x1000 //	Declared synthetic; not present in the source code.
}

pub enum InnerClassAccessFlags {
    Public = 0x0001, //	Marked or implicitly public in source.
    Private = 0x0002, //	Marked private in source.
    Protected = 0x0004, //	Marked protected in source.
    Static = 0x0008, //	Marked or implicitly static in source.
    Final = 0x0010, //	Marked final in source.
    Interface = 0x0200, //	Was an interface in source.
    Abstract = 0x0400, //	Marked or implicitly abstract in source.
    Synthetic = 0x1000, //	Declared synthetic; not present in the source code.
    Annotation = 0x2000, //	Declared as an annotation type.
    Enum = 0x4000, //	Declared as an enum type.
}

pub enum ParameterAccessFlags {
    Final = 0x0010,
    Synthetic = 0x1000,
    Mandated = 0x8000
}

#[derive(Default, Debug)]
pub struct Field {
    pub access_flags: AccessFlags,
    pub name_index: ConstantPoolIndex,
    pub descriptor_index: ConstantPoolIndex,
    pub attributes: Vec<Attribute>
}

#[derive(Default, Debug)]
pub struct Method {
    pub access_flags: AccessFlags,
    pub name_index: ConstantPoolIndex,
    pub descriptor_index: ConstantPoolIndex,
    pub attributes: Vec<Attribute>
}

#[derive(Debug)]
pub enum Attribute {
    ConstantValue(ConstantPoolIndex),
    Code { max_stack: u16, max_locals: u16, code: Vec<Instruction>, exception_table: Vec<ExceptionHandler>, attributes: Vec<Attribute> },
    StackMapTable(Vec<StackMapFrame>),
    Exceptions(Vec<ConstantPoolIndex>),
    InnerClasses(Vec<InnerClass>),
    EnclosingMethod { class_index: ConstantPoolIndex, method_index: ConstantPoolIndex },
    Synthetic,
    Signature(ConstantPoolIndex),
    SourceFile(ConstantPoolIndex),
    SourceDebugExtension(Vec<u8>),
    LineNumberTable(Vec<LineNumberTable>),
    LocalVariableTable(Vec<LocalVariableTable>),
    LocalVariableTypeTable(Vec<LocalVariableTypeTable>),
    Deprecated,
    RuntimeVisibleAnnotations(Vec<Annotation>),
    RuntimeInvisibleAnnotations(Vec<Annotation>),
    RuntimeVisibleParameterAnnotations(Vec<Vec<Annotation>>),
    RuntimeInvisibleParameterAnnotations(Vec<Vec<Annotation>>),
    RuntimeVisibleTypeAnnotations(Vec<TypeAnnotation>),
    RuntimeInvisibleTypeAnnotations(Vec<TypeAnnotation>),
    AnnotationDefault(ElementValue),
    BootstrapMethods(Vec<BootstrapMethod>),
    MethodParameters(Vec<MethodParameter>),
    RawAttribute { name_index: ConstantPoolIndex, info: Vec<u8> }
}

#[derive(Debug)]
pub enum StackMapFrame {
    SameFrame { tag: u8 },
    SameLocals1StackItemFrame { tag: u8, stack: VerificationType },
    SameLocals1StackItemFrameExtended { offset_delta: u16, stack: VerificationType },
    ChopFrame { tag: u8, offset_delta: u16 },
    SameFrameExtended { offset_delta: u16 },
    AppendFrame { tag: u8, offset_delta: u16, locals: Vec<VerificationType> },
    FullFrame { offset_delta: u16, locals: Vec<VerificationType>, stack: Vec<VerificationType> },
    FutureUse { tag: u8 }
}

impl StackMapFrame {
    pub fn len(&self) -> usize {
        match self {
            &StackMapFrame::SameFrame { tag: _ } => 1,
            &StackMapFrame::SameLocals1StackItemFrame{ tag: _, ref stack } => 1 + stack.len(),
            &StackMapFrame::SameLocals1StackItemFrameExtended { offset_delta: _, ref stack } => 3 + stack.len(),
            &StackMapFrame::ChopFrame { tag: _, offset_delta: _ } => 3,
            &StackMapFrame::SameFrameExtended { offset_delta: _ } => 3,
            &StackMapFrame::AppendFrame { tag: _, offset_delta: _, ref locals } => 3 + locals.iter().fold(0, |acc, x| acc + x.len()),
            &StackMapFrame::FullFrame { offset_delta: _, ref locals, ref stack } => 7 + locals.iter().fold(0, |acc, x| acc + x.len()) + stack.iter().fold(0, |acc, x| acc + x.len()),
            &StackMapFrame::FutureUse { tag: _ } => 0
        }
    }
}

#[derive(Debug)]
pub enum VerificationType {
    Top,
    Integer,
    Float,
    Long,
    Double,
    Null,
    UninitializedThis,
    Object { cpool_index: ConstantPoolIndex },
    Uninitialized { offset: u16 }
}

impl VerificationType {
    pub fn len(&self) -> usize {
        match self {
            &VerificationType::Object { cpool_index: _ } => 3,
            &VerificationType::Uninitialized { offset: _ } => 3,
            _ => 1
        }
    }
}

#[derive(Debug)]
pub struct ExceptionHandler {
    pub start_pc: u16,
    pub end_pc: u16,
    pub handler_pc: u16,
    pub catch_type: ConstantPoolIndex
}

#[derive(Debug)]
pub struct InnerClass {
    pub inner_class_info_index: ConstantPoolIndex,
    pub outer_class_info_index: ConstantPoolIndex,
    pub inner_name_index: ConstantPoolIndex,
    pub access_flags: AccessFlags
}

#[derive(Debug)]
pub struct LineNumberTable {
    pub start_pc: u16,
    pub line_number: u16
}

#[derive(Debug)]
pub struct LocalVariableTable {
    pub start_pc: u16,
    pub length: u16,
    pub name_index: ConstantPoolIndex,
    pub descriptor_index: ConstantPoolIndex,
    pub index: u16
}

#[derive(Debug)]
pub struct LocalVariableTypeTable {
    pub start_pc: u16,
    pub length: u16,
    pub name_index: ConstantPoolIndex,
    pub signature_index: ConstantPoolIndex,
    pub index: u16
}

#[derive(Debug)]
pub struct Annotation {
    pub type_index: ConstantPoolIndex,
    pub element_value_pairs: Vec<ElementValuePair>
}

impl Annotation {
    pub fn len(&self) -> usize {
        4 + self.element_value_pairs.iter().fold(0, |acc, x| acc + x.len())
        //4 + self.element_value_pairs.len() * 2 + self.element_value_pairs.iter().fold(0, |acc, x| acc + x.len())
    }
}

#[derive(Debug)]
pub struct ElementValuePair {
    pub element_name_index: ConstantPoolIndex,
    pub value: ElementValue
}

impl ElementValuePair {
    pub fn len(&self) -> usize {
        2 + self.value.len()
    }
}

#[derive(Debug)]
pub enum ElementValue {
    ConstantValue(u8, ConstantPoolIndex),
    Enum { type_name_index: ConstantPoolIndex, const_name_index: ConstantPoolIndex },
    ClassInfo(ConstantPoolIndex),
    Annotation(Annotation),
    Array(Vec<ElementValue>)
}

impl ElementValue {
    pub fn len(&self) -> usize {
        match self {
            &ElementValue::ConstantValue(_, _) => 3,
            &ElementValue::Enum { type_name_index: _, const_name_index: _ } => 5,
            &ElementValue::ClassInfo(_) => 3,
            &ElementValue::Annotation(ref annotation) => 1 + annotation.len(),
            &ElementValue::Array(ref table) => table.iter().fold(3, |acc, x| acc + x.len())
        }
    }
}

#[derive(Debug)]
pub struct TypeAnnotation {
    pub target_info: TargetInfo,
    pub target_path: TypePath,
    pub type_index: ConstantPoolIndex,
    pub element_value_pairs: Vec<ElementValuePair>
}

impl TypeAnnotation {
    pub fn len(&self) -> usize {
        5 + self.target_info.len() + self.target_path.len() + self.element_value_pairs.iter().fold(0, |acc, x| acc + x.len())
    }
}

#[derive(Debug)]
pub enum TargetInfo {
    TypeParameter { subtype: u8, idx: u8 },
    SuperType { idx: u16 },
    TypeParameterBound { subtype: u8, param_idx: u8, bound_index: u8 },
    Empty { subtype: u8 },
    MethodFormalParameter { idx: u8 },
    Throws { idx: u16 },
    LocalVar { subtype: u8, target: Vec<(u16, u16, u16)> },
    Catch { idx: u16 },
    Offset { subtype: u8, idx: u16 },
    TypeArgument { subtype: u8, offset: u16, type_arg_idx: u8 }
}

impl TargetInfo {
    pub fn len(&self) -> usize {
        match self {
            &TargetInfo::TypeParameter { subtype: _, idx: _ } => 1,
            &TargetInfo::SuperType { idx: _ } => 2,
            &TargetInfo::TypeParameterBound { subtype: _, param_idx: _, bound_index: _ } => 2,
            &TargetInfo::Empty { subtype: _ } => 0,
            &TargetInfo::MethodFormalParameter { idx: _ } => 1,
            &TargetInfo::Throws { idx: _ } => 2,
            &TargetInfo::LocalVar { subtype: _, ref target } => { 2 + target.len() * 6 },
            &TargetInfo::Catch { idx: _ } => 2,
            &TargetInfo::Offset { subtype: _, idx: _ } => 2,
            &TargetInfo::TypeArgument { subtype: _, offset: _, type_arg_idx: _ } => 3
        }
    }

    pub fn subtype(&self) -> u8 {
        match self {
            &TargetInfo::TypeParameter { subtype, idx: _ } => subtype,
            &TargetInfo::SuperType { idx: _ } => 0x10,
            &TargetInfo::TypeParameterBound { subtype, param_idx: _, bound_index: _ } => subtype,
            &TargetInfo::Empty { subtype } => subtype,
            &TargetInfo::MethodFormalParameter { idx: _ } => 0x16,
            &TargetInfo::Throws { idx: _ } => 0x17,
            &TargetInfo::LocalVar { subtype, target: _ } => subtype,
            &TargetInfo::Catch { idx: _ } => 0x42,
            &TargetInfo::Offset { subtype, idx: _ } => subtype,
            &TargetInfo::TypeArgument { subtype, offset: _, type_arg_idx: _ } => subtype
        }
    }
}

#[derive(Debug)]
pub struct TypePath {
    pub path: Vec<(TypePathKind, u8)>
}

impl TypePath {
    pub fn len(&self) -> usize {
        1 + self.path.len() * 2
    }
}

#[derive(Debug)]
pub enum TypePathKind {
    Array, // Annotation is deeper in an array type
    Nested, // Annotation is deeper in a nested type
    Wildcard, // Annotation is on the bound of a wildcard type argument of a parameterized type
    TypeArgument // Annotation is on a type argument of a parameterized type
}

impl TypePathKind {
    pub fn value(&self) -> u8 {
        match self {
            &TypePathKind::Array => 0,
            &TypePathKind::Nested => 1,
            &TypePathKind::Wildcard => 2,
            &TypePathKind::TypeArgument => 3,
        }
    }
}

#[derive(Debug)]
pub struct BootstrapMethod {
    pub bootstrap_method_ref: ConstantPoolIndex,
    pub bootstrap_arguments: Vec<ConstantPoolIndex>
}

impl BootstrapMethod {
}

#[derive(Debug)]
pub struct MethodParameter {
    pub name_index: ConstantPoolIndex,
    pub access_flags: AccessFlags
}

impl MethodParameter {
    pub fn len(&self) -> usize { 4 }
}

#[allow(non_camel_case_types)]
#[derive(Debug)]
pub enum Instruction {
    AALOAD,
    AASTORE,
    ACONST_NULL,
    ALOAD(u8),
    ALOAD_0,
    ALOAD_1,
    ALOAD_2,
    ALOAD_3,
    ANEWARRAY(u16),
    ARETURN,
    ARRAYLENGTH,
    ASTORE(u8),
    ASTORE_0,
    ASTORE_1,
    ASTORE_2,
    ASTORE_3,
    ATHROW,
    BALOAD,
    BASTORE,
    BIPUSH(u8),
    CALOAD,
    CASTORE,
    CHECKCAST(u16),
    D2F,
    D2I,
    D2L,
    DADD,
    DALOAD,
    DASTORE,
    DCMPL,
    DCMPG,
    DCONST_0,
    DCONST_1,
    DDIV,
    DLOAD(u8),
    DLOAD_0,
    DLOAD_1,
    DLOAD_2,
    DLOAD_3,
    DMUL,
    DNEG,
    DREM,
    DRETURN,
    DSTORE(u8),
    DSTORE_0,
    DSTORE_1,
    DSTORE_2,
    DSTORE_3,
    DSUB,
    DUP,
    DUP_X1,
    DUP_X2,
    DUP2,
    DUP2_X1,
    DUP2_X2,
    F2D,
    F2I,
    F2L,
    FADD,
    FALOAD,
    FASTORE,
    FCMPL,
    FCMPG,
    FCONST_0,
    FCONST_1,
    FCONST_2,
    FDIV,
    FLOAD(u8),
    FLOAD_0,
    FLOAD_1,
    FLOAD_2,
    FLOAD_3,
    FMUL,
    FNEG,
    FREM,
    FRETURN,
    FSTORE(u8),
    FSTORE_0,
    FSTORE_1,
    FSTORE_2,
    FSTORE_3,
    FSUB,
    GETFIELD(u16),
    GETSTATIC(u16),
    GOTO(i16),
    GOTO_W(i32),
    I2B,
    I2C,
    I2D,
    I2F,
    I2L,
    I2S,
    IADD,
    IALOAD,
    IAND,
    IASTORE,
    ICONST_M1,
    ICONST_0,
    ICONST_1,
    ICONST_2,
    ICONST_3,
    ICONST_4,
    ICONST_5,
    IDIV,
    IF_ACMPEQ(i16),
    IF_ACMPNE(i16),
    IF_ICMPEQ(i16),
    IF_ICMPNE(i16),
    IF_ICMPLT(i16),
    IF_ICMPGE(i16),
    IF_ICMPGT(i16),
    IF_ICMPLE(i16),
    IFEQ(i16),
    IFNE(i16),
    IFLT(i16),
    IFGE(i16),
    IFGT(i16),
    IFLE(i16),
    IFNONNULL(i16),
    IFNULL(i16),
    IINC(u8, i8),
    ILOAD(u8),
    ILOAD_0,
    ILOAD_1,
    ILOAD_2,
    ILOAD_3,
    IMUL,
    INEG,
    INSTANCEOF(u16),
    INVOKEDYNAMIC(u16),
    INVOKEINTERFACE(u16, u8),
    INVOKESPECIAL(u16),
    INVOKESTATIC(u16),
    INVOKEVIRTUAL(u16),
    IOR,
    IREM,
    IRETURN,
    ISHL,
    ISHR,
    ISTORE(u8),
    ISTORE_0,
    ISTORE_1,
    ISTORE_2,
    ISTORE_3,
    ISUB,
    IUSHR,
    IXOR,
    JSR(i16),
    JSR_W(i32),
    L2D,
    L2F,
    L2I,
    LADD,
    LALOAD,
    LAND,
    LASTORE,
    LCMP,
    LCONST_0,
    LCONST_1,
    LDC(u8),
    LDC_W(u16),
    LDC2_W(u16),
    LDIV,
    LLOAD(u8),
    LLOAD_0,
    LLOAD_1,
    LLOAD_2,
    LLOAD_3,
    LMUL,
    LNEG,
    LOOKUPSWITCH(i32, Vec<(i32, i32)>),
    LOR,
    LREM,
    LRETURN,
    LSHL,
    LSHR,
    LSTORE(u8),
    LSTORE_0,
    LSTORE_1,
    LSTORE_2,
    LSTORE_3,
    LSUB,
    LUSHR,
    LXOR,
    MONITORENTER,
    MONITOREXIT,
    MULTIANEWARRAY(u16, u8),
    NEW(u16),
    NEWARRAY(u8),
    NOP,
    POP,
    POP2,
    PUTFIELD(u16),
    PUTSTATIC(u16),
    RET(u8),
    RETURN,
    SALOAD,
    SASTORE,
    SIPUSH(u16),
    SWAP,
    TABLESWITCH(i32, i32, i32, Vec<i32>),
    IINC_W(u16, i16),
    ILOAD_W(u16),
    FLOAD_W(u16),
    ALOAD_W(u16),
    LLOAD_W(u16),
    DLOAD_W(u16),
    ISTORE_W(u16),
    FSTORE_W(u16),
    ASTORE_W(u16),
    LSTORE_W(u16),
    DSTORE_W(u16),
    RET_W(u16),
    PADDED_INSTRUCTION(usize),
    WTF(u32),
}

impl Instruction {
    pub fn len(&self) -> usize {
        match self {
            &Instruction::ALOAD(_) => 2,
            &Instruction::ANEWARRAY(_) => 3,
            &Instruction::ASTORE(_) => 2,
            &Instruction::CHECKCAST(_) => 3,
            &Instruction::DLOAD(_) => 2,
            &Instruction::DSTORE(_) => 2,
            &Instruction::FLOAD(_) => 2,
            &Instruction::FSTORE(_) => 2,
            &Instruction::GETFIELD(_) => 3,
            &Instruction::GETSTATIC(_) => 3,
            &Instruction::GOTO(_) => 3,
            &Instruction::GOTO_W(_) => 5,
            &Instruction::IF_ACMPEQ(_) => 3,
            &Instruction::IF_ACMPNE(_) => 3,
            &Instruction::IF_ICMPEQ(_) => 3,
            &Instruction::IF_ICMPNE(_) => 3,
            &Instruction::IF_ICMPLT(_) => 3,
            &Instruction::IF_ICMPGE(_) => 3,
            &Instruction::IF_ICMPGT(_) => 3,
            &Instruction::IF_ICMPLE(_) => 3,
            &Instruction::IFEQ(_) => 3,
            &Instruction::IFNE(_) => 3,
            &Instruction::IFLT(_) => 3,
            &Instruction::IFGE(_) => 3,
            &Instruction::IFGT(_) => 3,
            &Instruction::IFLE(_) => 3,
            &Instruction::IFNONNULL(_) => 3,
            &Instruction::IFNULL(_) => 3,
            &Instruction::IINC(_, _) => 3,
            &Instruction::ILOAD(_) => 2,
            &Instruction::INSTANCEOF(_) => 3,
            &Instruction::INVOKEDYNAMIC(_) => 3,
            &Instruction::INVOKEINTERFACE(_, _) => 4,
            &Instruction::INVOKESPECIAL(_) => 3,
            &Instruction::INVOKESTATIC(_) => 3,
            &Instruction::INVOKEVIRTUAL(_) => 3,
            &Instruction::ISTORE(_) => 2,
            &Instruction::JSR(_) => 3,
            &Instruction::JSR_W(_) => 5,
            &Instruction::LDC(_) => 2,
            &Instruction::LDC_W(_) => 3,
            &Instruction::LDC2_W(_) => 3,
            &Instruction::LLOAD(_) => 2,
            &Instruction::LOOKUPSWITCH(_, ref pairs) => { 5 + pairs.len() * 4 },
            &Instruction::LSTORE(_) => 2,
            &Instruction::MULTIANEWARRAY(_, _) => 4,
            &Instruction::NEW(_) => 3,
            &Instruction::NEWARRAY(_) => 2,
            &Instruction::PUTFIELD(_) => 3,
            &Instruction::PUTSTATIC(_) => 3,
            &Instruction::RET(_) => 2,
            &Instruction::SIPUSH(_) => 3,
            &Instruction::TABLESWITCH(_, _, _, ref indices) => { 13 + (indices.len() * 4) },
            &Instruction::IINC_W(_, _) => 9,
            &Instruction::ILOAD_W(_) => 5,
            &Instruction::FLOAD_W(_) => 5,
            &Instruction::ALOAD_W(_) => 5,
            &Instruction::LLOAD_W(_) => 5,
            &Instruction::DLOAD_W(_) => 5,
            &Instruction::ISTORE_W(_) => 5,
            &Instruction::FSTORE_W(_) => 5,
            &Instruction::ASTORE_W(_) => 5,
            &Instruction::LSTORE_W(_) => 5,
            &Instruction::DSTORE_W(_) => 5,
            &Instruction::RET_W(_) => 5,
            &Instruction::PADDED_INSTRUCTION(padding) => padding,
            _ => 1
        }
    }
}
