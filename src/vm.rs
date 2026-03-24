pub mod optimization;

use std::collections::{HashMap, hash_map::Entry};
use std::ops::{Deref, DerefMut};
use std::rc::Rc;

use crate::atoms::{Atom, AtomTable, Shape as AtomShape};
use crate::gc::{self, GCHeader, ObjType};
use crate::heap::{
    QArray, QBoolArray, QClass, QClosure, QFloat64Array, QFunction, QInstance, QInt32Array,
    QModule, QNativeClosure, QNativeFunction, QObject, QString, QStringArray, QSymbol, QUint8Array,
};
use crate::js_value::*;
use crate::runtime_trait::{
    ArithmeticOps, AssignmentOps, BitwiseOps, CallOps, CoercionOps, ComparisonOps,
    LogicalAssignOps, LogicalOps, NullishOps, PropertyOps, Ternary, TypeOps, ValueOps,
};

pub type JSString = QString;
const ACC: usize = 255;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PropertyKey {
    Id(u16),
    Atom(Atom),
    Index(u32),
    Value(JSValue),
}

impl PropertyKey {
    fn sort_key(&self) -> (u8, u64) {
        match *self {
            PropertyKey::Id(id) => (0, u64::from(id)),
            PropertyKey::Atom(atom) => (1, u64::from(atom.0)),
            PropertyKey::Index(index) => (2, u64::from(index)),
            PropertyKey::Value(value) => (3, value.bits()),
        }
    }
}

#[derive(Debug, Clone)]
pub enum ObjectKind {
    Ordinary(QObject),
    Array(QArray),
    BoolArray(QBoolArray),
    Uint8Array(QUint8Array),
    Int32Array(QInt32Array),
    Float64Array(QFloat64Array),
    StringArray(QStringArray),
    Iterator { values: Vec<JSValue>, index: usize },
    Function(QFunction),
    Closure(QClosure),
    NativeFunction(QNativeFunction),
    NativeClosure(QNativeClosure),
    Class(QClass),
    Module(QModule),
    Instance(QInstance),
    Symbol(QSymbol),
    Env(QObject),
}

#[repr(C)]
#[derive(Debug)]
pub struct Shape {
    pub header: GCHeader,
    pub id: u32,
    pub parent: Option<*mut Shape>,
    pub key: Option<PropertyKey>,
    pub offset: u32,
    pub property_count: u32,
    pub prototype: Option<*mut Shape>,
    pub proto_cache_offset: u32,
    pub proto_cache_shape: Option<*mut Shape>,
}

#[repr(C, align(16))]
#[derive(Debug)]
pub struct JSObject {
    pub header: GCHeader,
    pub shape: *mut Shape,
    pub properties: HashMap<PropertyKey, JSValue>,
    pub kind: ObjectKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ICState {
    Uninit = 0,
    Mono = 1,
    Poly = 2,
    Mega = 3,
}

#[derive(Debug, Clone)]
pub struct InlineCache {
    pub state: ICState,
    pub shape_id: u32,
    pub offset: u32,
    pub key: Option<PropertyKey>,
    pub shapes: Vec<u32>,
}

impl Default for InlineCache {
    fn default() -> Self {
        Self {
            state: ICState::Uninit,
            shape_id: 0,
            offset: 0,
            key: None,
            shapes: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ValueProfileKind {
    Undefined,
    Null,
    Boolean,
    Number,
    String,
    Object,
    Function,
}

impl ValueProfileKind {
    pub fn from_id(id: u8) -> Option<Self> {
        match id {
            0 => Some(Self::Undefined),
            1 => Some(Self::Null),
            2 => Some(Self::Boolean),
            3 => Some(Self::Number),
            4 => Some(Self::String),
            5 => Some(Self::Object),
            6 => Some(Self::Function),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TypeFeedbackSlot {
    pub last: Option<ValueProfileKind>,
    pub samples: u32,
    pub stable: bool,
}

impl Default for TypeFeedbackSlot {
    fn default() -> Self {
        Self {
            last: None,
            samples: 0,
            stable: true,
        }
    }
}

impl TypeFeedbackSlot {
    fn observe(&mut self, kind: ValueProfileKind) {
        self.samples = self.samples.saturating_add(1);
        self.stable = match self.last {
            Some(previous) => self.stable && previous == kind,
            None => true,
        };
        self.last = Some(kind);
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CallFeedbackSlot {
    pub last: Option<ValueProfileKind>,
    pub samples: u32,
    pub monomorphic: bool,
}

impl Default for CallFeedbackSlot {
    fn default() -> Self {
        Self {
            last: None,
            samples: 0,
            monomorphic: true,
        }
    }
}

impl CallFeedbackSlot {
    fn observe(&mut self, kind: ValueProfileKind) {
        self.samples = self.samples.saturating_add(1);
        self.monomorphic = match self.last {
            Some(previous) => self.monomorphic && previous == kind,
            None => true,
        };
        self.last = Some(kind);
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DeoptReason {
    TypeMismatch {
        expected: ValueProfileKind,
        observed: ValueProfileKind,
    },
    StructMismatch {
        expected: u32,
        observed: u32,
    },
    InlineCacheMismatch {
        slot: usize,
        expected: u32,
        observed: u32,
    },
    SafetyCheck {
        register: usize,
    },
}

#[derive(Debug, Clone, Default)]
pub struct RuntimeFeedback {
    pub type_slots: Vec<TypeFeedbackSlot>,
    pub call_slots: Vec<CallFeedbackSlot>,
    pub return_slot: TypeFeedbackSlot,
    pub hot_loop_counts: HashMap<usize, u32>,
    pub loop_hint_counts: HashMap<usize, u32>,
    pub jit_hints: HashMap<usize, u32>,
    pub osr_entries: u32,
    pub osr_exits: u32,
    pub safety_checks: u32,
    pub failed_safety_checks: u32,
    pub ic_misses: u32,
    pub deopt_count: u32,
    pub last_deopt: Option<DeoptReason>,
    pub last_loop_hint_pc: Option<usize>,
    pub last_call_kind: Option<ValueProfileKind>,
    pub last_ic_slot: Option<usize>,
    pub osr_active: bool,
}

#[derive(Debug)]
pub struct FrameHeader {
    pub return_pc: usize,
    pub function_id: usize,
    pub env: Option<JSValue>,
    pub frame_size: u32,
    pub register_count: u32,
    pub construct_result: Option<JSValue>,
    pub scope_depth: usize,
}

#[derive(Debug)]
pub struct Frame {
    pub header: FrameHeader,
    pub regs: [JSValue; 256],
    pub ic_vector: Vec<InlineCache>,
    pub inline_args: [JSValue; 2],
    pub args: Vec<JSValue>,
    pub argc: u32,
    pub try_stack: Vec<usize>,
    pub scope_stack: Vec<usize>,
}

impl Frame {
    #[inline(always)]
    fn fresh(
        args: Vec<JSValue>,
        this_value: JSValue,
        function_id: usize,
        return_pc: usize,
        construct_result: Option<JSValue>,
        scope_depth: usize,
    ) -> Self {
        let mut frame = Self {
            header: FrameHeader {
                return_pc,
                function_id,
                env: None,
                frame_size: 256,
                register_count: 256,
                construct_result,
                scope_depth,
            },
            regs: [make_undefined(); 256],
            ic_vector: Vec::new(),
            inline_args: [make_undefined(); 2],
            args: Vec::new(),
            argc: 0,
            try_stack: Vec::new(),
            scope_stack: Vec::new(),
        };
        frame.regs[0] = this_value;
        frame.set_args(&args);
        frame
    }

    #[inline(always)]
    fn set_args(&mut self, args: &[JSValue]) {
        self.inline_args = [make_undefined(); 2];
        self.args.clear();
        self.argc = args.len() as u32;

        match args {
            [] => {}
            [arg0] => {
                self.inline_args[0] = *arg0;
            }
            [arg0, arg1] => {
                self.inline_args[0] = *arg0;
                self.inline_args[1] = *arg1;
            }
            _ => {
                self.inline_args[0] = args[0];
                self.inline_args[1] = args[1];
                self.args.extend_from_slice(&args[2..]);
            }
        }
    }

    #[inline(always)]
    fn arg(&self, index: usize) -> JSValue {
        if index >= self.argc as usize {
            return make_undefined();
        }

        match index {
            0 => self.inline_args[0],
            1 => self.inline_args[1],
            _ => self
                .args
                .get(index - self.inline_args.len())
                .copied()
                .unwrap_or(make_undefined()),
        }
    }

    #[inline(always)]
    fn reset(
        &mut self,
        args: &[JSValue],
        this_value: JSValue,
        function_id: usize,
        return_pc: usize,
        construct_result: Option<JSValue>,
        scope_depth: usize,
    ) {
        self.header.return_pc = return_pc;
        self.header.function_id = function_id;
        self.header.env = None;
        self.header.frame_size = 256;
        self.header.register_count = 256;
        self.header.construct_result = construct_result;
        self.header.scope_depth = scope_depth;
        self.regs.fill(make_undefined());
        self.regs[0] = this_value;
        self.ic_vector.clear();
        self.set_args(args);
        self.try_stack.clear();
        self.scope_stack.clear();
    }
}

#[derive(Debug)]
pub struct FrameStack {
    frames: Vec<Frame>,
    sp: usize,
    current: *mut Frame,
}

impl FrameStack {
    #[inline(always)]
    fn new(root: Frame) -> Self {
        let mut frames = Vec::with_capacity(32);
        frames.push(root);
        let current = frames.as_mut_ptr();
        Self {
            frames,
            sp: 0,
            current,
        }
    }

    #[inline(always)]
    fn depth(&self) -> usize {
        self.sp
    }

    #[inline(always)]
    fn sync_current(&mut self) {
        self.current = unsafe { self.frames.as_mut_ptr().add(self.sp) };
    }

    #[inline(always)]
    fn ensure_next_frame(&mut self) -> &mut Frame {
        let next = self.sp + 1;
        if next == self.frames.len() {
            self.frames
                .push(Frame::fresh(Vec::new(), make_undefined(), 0, 0, None, 0));
        }
        self.sp = next;
        self.sync_current();
        unsafe { &mut *self.current }
    }

    #[inline(always)]
    fn pop_frame(&mut self) -> bool {
        if self.sp == 0 {
            return false;
        }
        self.sp -= 1;
        self.sync_current();
        true
    }

    #[inline(always)]
    pub(crate) fn active_frames(&self) -> &[Frame] {
        &self.frames[..=self.sp]
    }
}

impl Deref for FrameStack {
    type Target = Frame;

    #[inline(always)]
    fn deref(&self) -> &Self::Target {
        debug_assert!(!self.current.is_null());
        unsafe { &*self.current }
    }
}

impl DerefMut for FrameStack {
    #[inline(always)]
    fn deref_mut(&mut self) -> &mut Self::Target {
        debug_assert!(!self.current.is_null());
        unsafe { &mut *self.current }
    }
}

#[derive(Debug)]
pub struct VM {
    pub frame: FrameStack,
    pub pc: usize,
    pub bytecode: Vec<u32>,
    pub const_pool: Vec<JSValue>,
    pub objects: Vec<*mut JSObject>,
    pub shapes: Vec<*mut Shape>,
    pub strings: Vec<*mut JSString>,
    pub global_object: HashMap<u16, JSValue>,
    pub scope_chain: Vec<JSValue>,
    pub upvalues: Vec<JSValue>,
    pub last_exception: JSValue,
    pub(crate) interned_strings: HashMap<String, JSValue>,
    pub atoms: AtomTable,
    pub feedback: RuntimeFeedback,
    heap_shape: Rc<AtomShape>,
    next_shape_id: u32,
    last_ic_object: Option<*mut JSObject>,
}

enum CallAction {
    Returned(JSValue),
    EnteredFrame,
}

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Opcode {
    Mov,
    LoadK,
    Add,
    GetPropIc,
    Call,
    Jmp,
    LoadI,
    JmpTrue,
    JmpFalse,
    SetPropIc,
    AddAccImm8,
    IncAcc,
    LoadThis,
    Load0,
    Load1,
    Eq,
    Lt,
    Lte,
    AddAcc,
    SubAcc,
    MulAcc,
    DivAcc,
    LoadNull,
    LoadTrue,
    LoadFalse,
    LoadGlobalIc,
    SetGlobalIc,
    Typeof,
    ToNum,
    ToStr,
    IsUndef,
    IsNull,
    SubAccImm8,
    MulAccImm8,
    DivAccImm8,
    AddStrAcc,
    AddI,
    SubI,
    MulI,
    DivI,
    ModI,
    Neg,
    Inc,
    Dec,
    AddStr,
    ToPrimitive,
    GetPropAcc,
    SetPropAcc,
    GetIdxFast,
    SetIdxFast,
    LoadArg,
    LoadAcc,
    StrictEq,
    StrictNeq,
    BitAnd,
    BitOr,
    BitXor,
    BitNot,
    Shl,
    Shr,
    Ushr,
    Pow,
    LogicalAnd,
    LogicalOr,
    NullishCoalesce,
    In,
    Instanceof,
    GetLengthIc,
    ArrayPushAcc,
    NewObj,
    NewArr,
    NewFunc,
    NewClass,
    GetProp,
    SetProp,
    GetIdxIc,
    SetIdxIc,
    GetGlobal,
    SetGlobal,
    GetUpval,
    SetUpval,
    GetScope,
    SetScope,
    ResolveScope,
    GetSuper,
    SetSuper,
    DeleteProp,
    HasProp,
    Keys,
    ForIn,
    IteratorNext,
    Spread,
    Destructure,
    CreateEnv,
    LoadName,
    StoreName,
    LoadClosure,
    NewThis,
    TypeofName,
    JmpEq,
    JmpNeq,
    JmpLt,
    JmpLte,
    LoopIncJmp,
    Switch,
    LoopHint,
    Ret,
    RetU,
    TailCall,
    Construct,
    CallVar,
    Enter,
    Leave,
    Yield,
    Await,
    Throw,
    Try,
    EndTry,
    Catch,
    Finally,
    CallIc,
    CallIcVar,
    ProfileType,
    ProfileCall,
    ProfileRet,
    CheckType,
    CheckStruct,
    CheckIc,
    IcInit,
    IcUpdate,
    IcMiss,
    OsrEntry,
    ProfileHotLoop,
    OsrExit,
    JitHint,
    SafetyCheck,
    GetPropIcCall,
    IncJmpFalseLoop,
    LoadKAddAcc,
    AddMov,
    EqJmpTrue,
    GetPropAccCall,
    LoadKMulAcc,
    LtJmp,
    GetPropIcMov,
    GetPropAddImmSetPropIc,
    AddAccImm8Mov,
    CallIcSuper,
    LoadThisCall,
    EqJmpFalse,
    LoadKSubAcc,
    GetLengthIcCall,
    AddStrAccMov,
    IncAccJmp,
    GetPropChainAcc,
    TestJmpTrue,
    LoadArgCall,
    MulAccMov,
    LteJmpLoop,
    NewObjInitProp,
    ProfileHotCall,
    AssertValue,
    AssertOk,
    AssertEqual,
    AssertNotEqual,
    AssertDeepEqual,
    AssertNotDeepEqual,
    AssertStrictEqual,
    AssertNotStrictEqual,
    AssertDeepStrictEqual,
    AssertNotDeepStrictEqual,
    AssertThrows,
    AssertDoesNotThrow,
    AssertRejects,
    AssertDoesNotReject,
    AssertFail,
    Call1SubI,
    JmpLteFalse,
    RetReg,
    AddI32,
    AddF64,
    SubI32,
    SubF64,
    MulI32,
    MulF64,
    Reserved(u8),
}

impl From<u8> for Opcode {
    fn from(value: u8) -> Self {
        match value {
            0 => Self::Mov,
            1 => Self::LoadK,
            2 => Self::Add,
            3 => Self::GetPropIc,
            4 => Self::Call,
            5 => Self::Jmp,
            6 => Self::LoadI,
            7 => Self::JmpTrue,
            8 => Self::JmpFalse,
            9 => Self::SetPropIc,
            10 => Self::AddAccImm8,
            11 => Self::IncAcc,
            12 => Self::LoadThis,
            13 => Self::Load0,
            14 => Self::Load1,
            15 => Self::Eq,
            16 => Self::Lt,
            17 => Self::Lte,
            18 => Self::AddAcc,
            19 => Self::SubAcc,
            20 => Self::MulAcc,
            21 => Self::DivAcc,
            22 => Self::LoadNull,
            23 => Self::LoadTrue,
            24 => Self::LoadFalse,
            25 => Self::LoadGlobalIc,
            26 => Self::SetGlobalIc,
            27 => Self::Typeof,
            28 => Self::ToNum,
            29 => Self::ToStr,
            30 => Self::IsUndef,
            31 => Self::IsNull,
            32 => Self::SubAccImm8,
            33 => Self::MulAccImm8,
            34 => Self::DivAccImm8,
            35 => Self::AddStrAcc,
            36 => Self::AddI,
            37 => Self::SubI,
            38 => Self::MulI,
            39 => Self::DivI,
            40 => Self::ModI,
            41 => Self::Neg,
            42 => Self::Inc,
            43 => Self::Dec,
            44 => Self::AddStr,
            45 => Self::ToPrimitive,
            46 => Self::GetPropAcc,
            47 => Self::SetPropAcc,
            48 => Self::GetIdxFast,
            49 => Self::SetIdxFast,
            50 => Self::LoadArg,
            51 => Self::LoadAcc,
            52 => Self::StrictEq,
            53 => Self::StrictNeq,
            54 => Self::BitAnd,
            55 => Self::BitOr,
            56 => Self::BitXor,
            57 => Self::BitNot,
            58 => Self::Shl,
            59 => Self::Shr,
            60 => Self::Ushr,
            117 => Self::Pow,
            118 => Self::LogicalAnd,
            119 => Self::LogicalOr,
            120 => Self::NullishCoalesce,
            121 => Self::In,
            122 => Self::Instanceof,
            64 => Self::GetLengthIc,
            65 => Self::ArrayPushAcc,
            66 => Self::NewObj,
            67 => Self::NewArr,
            68 => Self::NewFunc,
            69 => Self::NewClass,
            70 => Self::GetProp,
            71 => Self::SetProp,
            72 => Self::GetIdxIc,
            73 => Self::SetIdxIc,
            74 => Self::GetGlobal,
            75 => Self::SetGlobal,
            76 => Self::GetUpval,
            77 => Self::SetUpval,
            78 => Self::GetScope,
            79 => Self::SetScope,
            80 => Self::ResolveScope,
            81 => Self::GetSuper,
            82 => Self::SetSuper,
            83 => Self::DeleteProp,
            84 => Self::HasProp,
            85 => Self::Keys,
            86 => Self::ForIn,
            87 => Self::IteratorNext,
            88 => Self::Spread,
            89 => Self::Destructure,
            90 => Self::CreateEnv,
            91 => Self::LoadName,
            92 => Self::StoreName,
            93 => Self::LoadClosure,
            94 => Self::NewThis,
            95 => Self::TypeofName,
            96 => Self::JmpEq,
            97 => Self::JmpNeq,
            98 => Self::JmpLt,
            99 => Self::JmpLte,
            100 => Self::LoopIncJmp,
            101 => Self::Switch,
            102 => Self::LoopHint,
            103 => Self::Ret,
            104 => Self::RetU,
            105 => Self::TailCall,
            106 => Self::Construct,
            107 => Self::CallVar,
            108 => Self::Enter,
            109 => Self::Leave,
            110 => Self::Yield,
            111 => Self::Await,
            112 => Self::Throw,
            113 => Self::Try,
            114 => Self::EndTry,
            115 => Self::Catch,
            116 => Self::Finally,
            128 => Self::CallIc,
            129 => Self::CallIcVar,
            160 => Self::ProfileType,
            161 => Self::ProfileCall,
            162 => Self::ProfileRet,
            163 => Self::CheckType,
            164 => Self::CheckStruct,
            165 => Self::CheckIc,
            166 => Self::IcInit,
            167 => Self::IcUpdate,
            168 => Self::IcMiss,
            169 => Self::OsrEntry,
            170 => Self::ProfileHotLoop,
            171 => Self::OsrExit,
            172 => Self::JitHint,
            173 => Self::SafetyCheck,
            200 => Self::GetPropIcCall,
            201 => Self::IncJmpFalseLoop,
            202 => Self::LoadKAddAcc,
            203 => Self::AddMov,
            204 => Self::EqJmpTrue,
            205 => Self::GetPropAccCall,
            206 => Self::LoadKMulAcc,
            207 => Self::LtJmp,
            208 => Self::GetPropIcMov,
            209 => Self::GetPropAddImmSetPropIc,
            210 => Self::AddAccImm8Mov,
            211 => Self::CallIcSuper,
            212 => Self::LoadThisCall,
            213 => Self::EqJmpFalse,
            214 => Self::LoadKSubAcc,
            215 => Self::GetLengthIcCall,
            216 => Self::AddStrAccMov,
            217 => Self::IncAccJmp,
            218 => Self::GetPropChainAcc,
            219 => Self::TestJmpTrue,
            220 => Self::LoadArgCall,
            221 => Self::MulAccMov,
            222 => Self::LteJmpLoop,
            223 => Self::NewObjInitProp,
            224 => Self::ProfileHotCall,
            225 => Self::AssertValue,
            226 => Self::AssertOk,
            227 => Self::AssertEqual,
            228 => Self::AssertNotEqual,
            229 => Self::AssertDeepEqual,
            230 => Self::AssertNotDeepEqual,
            231 => Self::AssertStrictEqual,
            232 => Self::AssertNotStrictEqual,
            233 => Self::AssertDeepStrictEqual,
            234 => Self::AssertNotDeepStrictEqual,
            235 => Self::AssertThrows,
            236 => Self::AssertDoesNotThrow,
            237 => Self::AssertRejects,
            238 => Self::AssertDoesNotReject,
            239 => Self::AssertFail,
            240 => Self::Call1SubI,
            241 => Self::JmpLteFalse,
            242 => Self::RetReg,
            243 => Self::AddI32,
            244 => Self::AddF64,
            245 => Self::SubI32,
            246 => Self::SubF64,
            247 => Self::MulI32,
            248 => Self::MulF64,
            other => Self::Reserved(other),
        }
    }
}

impl Opcode {
    /// Convert an Opcode to its u8 representation
    pub fn as_u8(self) -> u8 {
        match self {
            Self::Mov => 0,
            Self::LoadK => 1,
            Self::Add => 2,
            Self::GetPropIc => 3,
            Self::Call => 4,
            Self::Jmp => 5,
            Self::LoadI => 6,
            Self::JmpTrue => 7,
            Self::JmpFalse => 8,
            Self::SetPropIc => 9,
            Self::AddAccImm8 => 10,
            Self::IncAcc => 11,
            Self::LoadThis => 12,
            Self::Load0 => 13,
            Self::Load1 => 14,
            Self::Eq => 15,
            Self::Lt => 16,
            Self::Lte => 17,
            Self::AddAcc => 18,
            Self::SubAcc => 19,
            Self::MulAcc => 20,
            Self::DivAcc => 21,
            Self::LoadNull => 22,
            Self::LoadTrue => 23,
            Self::LoadFalse => 24,
            Self::LoadGlobalIc => 25,
            Self::SetGlobalIc => 26,
            Self::Typeof => 27,
            Self::ToNum => 28,
            Self::ToStr => 29,
            Self::IsUndef => 30,
            Self::IsNull => 31,
            Self::SubAccImm8 => 32,
            Self::MulAccImm8 => 33,
            Self::DivAccImm8 => 34,
            Self::AddStrAcc => 35,
            Self::AddI => 36,
            Self::SubI => 37,
            Self::MulI => 38,
            Self::DivI => 39,
            Self::ModI => 40,
            Self::Neg => 41,
            Self::Inc => 42,
            Self::Dec => 43,
            Self::AddStr => 44,
            Self::ToPrimitive => 45,
            Self::GetPropAcc => 46,
            Self::SetPropAcc => 47,
            Self::GetIdxFast => 48,
            Self::SetIdxFast => 49,
            Self::LoadArg => 50,
            Self::LoadAcc => 51,
            Self::StrictEq => 52,
            Self::StrictNeq => 53,
            Self::BitAnd => 54,
            Self::BitOr => 55,
            Self::BitXor => 56,
            Self::BitNot => 57,
            Self::Shl => 58,
            Self::Shr => 59,
            Self::Ushr => 60,
            Self::Pow => 117,
            Self::LogicalAnd => 118,
            Self::LogicalOr => 119,
            Self::NullishCoalesce => 120,
            Self::In => 121,
            Self::Instanceof => 122,
            Self::GetLengthIc => 64,
            Self::ArrayPushAcc => 65,
            Self::NewObj => 66,
            Self::NewArr => 67,
            Self::NewFunc => 68,
            Self::NewClass => 69,
            Self::GetProp => 70,
            Self::SetProp => 71,
            Self::GetIdxIc => 72,
            Self::SetIdxIc => 73,
            Self::GetGlobal => 74,
            Self::SetGlobal => 75,
            Self::GetUpval => 76,
            Self::SetUpval => 77,
            Self::GetScope => 78,
            Self::SetScope => 79,
            Self::ResolveScope => 80,
            Self::GetSuper => 81,
            Self::SetSuper => 82,
            Self::DeleteProp => 83,
            Self::HasProp => 84,
            Self::Keys => 85,
            Self::ForIn => 86,
            Self::IteratorNext => 87,
            Self::Spread => 88,
            Self::Destructure => 89,
            Self::CreateEnv => 90,
            Self::LoadName => 91,
            Self::StoreName => 92,
            Self::LoadClosure => 93,
            Self::NewThis => 94,
            Self::TypeofName => 95,
            Self::JmpEq => 96,
            Self::JmpNeq => 97,
            Self::JmpLt => 98,
            Self::JmpLte => 99,
            Self::LoopIncJmp => 100,
            Self::Switch => 101,
            Self::LoopHint => 102,
            Self::Ret => 103,
            Self::RetU => 104,
            Self::TailCall => 105,
            Self::Construct => 106,
            Self::CallVar => 107,
            Self::Enter => 108,
            Self::Leave => 109,
            Self::Yield => 110,
            Self::Await => 111,
            Self::Throw => 112,
            Self::Try => 113,
            Self::EndTry => 114,
            Self::Catch => 115,
            Self::Finally => 116,
            Self::CallIc => 128,
            Self::CallIcVar => 129,
            Self::ProfileType => 160,
            Self::ProfileCall => 161,
            Self::ProfileRet => 162,
            Self::CheckType => 163,
            Self::CheckStruct => 164,
            Self::CheckIc => 165,
            Self::IcInit => 166,
            Self::IcUpdate => 167,
            Self::IcMiss => 168,
            Self::OsrEntry => 169,
            Self::ProfileHotLoop => 170,
            Self::OsrExit => 171,
            Self::JitHint => 172,
            Self::SafetyCheck => 173,
            Self::GetPropIcCall => 200,
            Self::IncJmpFalseLoop => 201,
            Self::LoadKAddAcc => 202,
            Self::AddMov => 203,
            Self::EqJmpTrue => 204,
            Self::GetPropAccCall => 205,
            Self::LoadKMulAcc => 206,
            Self::LtJmp => 207,
            Self::GetPropIcMov => 208,
            Self::GetPropAddImmSetPropIc => 209,
            Self::AddAccImm8Mov => 210,
            Self::CallIcSuper => 211,
            Self::LoadThisCall => 212,
            Self::EqJmpFalse => 213,
            Self::LoadKSubAcc => 214,
            Self::GetLengthIcCall => 215,
            Self::AddStrAccMov => 216,
            Self::IncAccJmp => 217,
            Self::GetPropChainAcc => 218,
            Self::TestJmpTrue => 219,
            Self::LoadArgCall => 220,
            Self::MulAccMov => 221,
            Self::LteJmpLoop => 222,
            Self::NewObjInitProp => 223,
            Self::ProfileHotCall => 224,
            Self::AssertValue => 225,
            Self::AssertOk => 226,
            Self::AssertEqual => 227,
            Self::AssertNotEqual => 228,
            Self::AssertDeepEqual => 229,
            Self::AssertNotDeepEqual => 230,
            Self::AssertStrictEqual => 231,
            Self::AssertNotStrictEqual => 232,
            Self::AssertDeepStrictEqual => 233,
            Self::AssertNotDeepStrictEqual => 234,
            Self::AssertThrows => 235,
            Self::AssertDoesNotThrow => 236,
            Self::AssertRejects => 237,
            Self::AssertDoesNotReject => 238,
            Self::AssertFail => 239,
            Self::Call1SubI => 240,
            Self::JmpLteFalse => 241,
            Self::RetReg => 242,
            Self::AddI32 => 243,
            Self::AddF64 => 244,
            Self::SubI32 => 245,
            Self::SubF64 => 246,
            Self::MulI32 => 247,
            Self::MulF64 => 248,
            Self::Reserved(value) => value,
        }
    }
}

#[derive(Clone, Copy)]
struct VmValue {
    vm: *mut VM,
    value: JSValue,
}

impl VmValue {
    fn new(vm: *mut VM, value: JSValue) -> Self {
        Self { vm, value }
    }

    fn raw(self) -> JSValue {
        self.value
    }

    fn wrap(&self, value: JSValue) -> Self {
        Self { vm: self.vm, value }
    }

    fn vm(&self) -> &VM {
        unsafe { &*self.vm }
    }

    fn with_vm_mut<R>(&self, f: impl FnOnce(&mut VM) -> R) -> R {
        unsafe { f(&mut *self.vm) }
    }

    fn wrap_bool(&self, value: bool) -> Self {
        self.wrap(make_bool(value))
    }

    fn prop_key(&self, key: JSValue) -> PropertyKey {
        self.vm().property_key_from_value(key)
    }

    fn int32_value(&self, value: JSValue) -> i32 {
        let numeric = self.with_vm_mut(|vm| vm.number_value(value));
        to_i32(numeric).unwrap_or(0)
    }
}

impl ArithmeticOps for VmValue {
    fn add(&self, rhs: &Self) -> Self {
        self.wrap(self.with_vm_mut(|vm| vm.binary_add(self.value, rhs.value)))
    }

    fn sub(&self, rhs: &Self) -> Self {
        self.wrap(self.with_vm_mut(|vm| vm.binary_numeric_op(self.value, rhs.value, |x, y| x - y)))
    }

    fn mul(&self, rhs: &Self) -> Self {
        self.wrap(self.with_vm_mut(|vm| vm.binary_numeric_op(self.value, rhs.value, |x, y| x * y)))
    }

    fn div(&self, rhs: &Self) -> Self {
        self.wrap(self.with_vm_mut(|vm| vm.binary_numeric_op(self.value, rhs.value, |x, y| x / y)))
    }

    fn rem(&self, rhs: &Self) -> Self {
        self.wrap(self.with_vm_mut(|vm| vm.binary_numeric_op(self.value, rhs.value, |x, y| x % y)))
    }

    fn pow(&self, rhs: &Self) -> Self {
        self.wrap(
            self.with_vm_mut(|vm| vm.binary_numeric_op(self.value, rhs.value, |x, y| x.powf(y))),
        )
    }

    fn inc(&self) -> Self {
        self.wrap(
            self.with_vm_mut(|vm| vm.binary_numeric_op(self.value, make_number(1.0), |x, y| x + y)),
        )
    }

    fn dec(&self) -> Self {
        self.wrap(
            self.with_vm_mut(|vm| vm.binary_numeric_op(self.value, make_number(1.0), |x, y| x - y)),
        )
    }

    fn unary_plus(&self) -> Self {
        self.wrap(self.with_vm_mut(|vm| vm.number_value(self.value)))
    }

    fn unary_minus(&self) -> Self {
        self.wrap(
            self.with_vm_mut(|vm| vm.binary_numeric_op(make_number(0.0), self.value, |x, y| x - y)),
        )
    }
}

impl ComparisonOps for VmValue {
    fn eq(&self, rhs: &Self) -> Self {
        self.wrap_bool(self.with_vm_mut(|vm| vm.abstract_equal(self.value, rhs.value)))
    }

    fn ne(&self, rhs: &Self) -> Self {
        self.wrap_bool(!self.with_vm_mut(|vm| vm.abstract_equal(self.value, rhs.value)))
    }

    fn strict_eq(&self, rhs: &Self) -> Self {
        self.wrap_bool(self.vm().strict_equal(self.value, rhs.value))
    }

    fn strict_ne(&self, rhs: &Self) -> Self {
        self.wrap_bool(!self.vm().strict_equal(self.value, rhs.value))
    }

    fn gt(&self, rhs: &Self) -> Self {
        self.wrap_bool(self.with_vm_mut(|vm| vm.less_than(rhs.value, self.value)))
    }

    fn lt(&self, rhs: &Self) -> Self {
        self.wrap_bool(self.with_vm_mut(|vm| vm.less_than(self.value, rhs.value)))
    }

    fn ge(&self, rhs: &Self) -> Self {
        self.wrap_bool(self.with_vm_mut(|vm| vm.less_than_or_equal(rhs.value, self.value)))
    }

    fn le(&self, rhs: &Self) -> Self {
        self.wrap_bool(self.with_vm_mut(|vm| vm.less_than_or_equal(self.value, rhs.value)))
    }
}

impl LogicalOps for VmValue {
    fn logical_and(&self, rhs: &Self) -> Self {
        if self.vm().is_truthy_value(self.value) {
            *rhs
        } else {
            *self
        }
    }

    fn logical_or(&self, rhs: &Self) -> Self {
        if self.vm().is_truthy_value(self.value) {
            *self
        } else {
            *rhs
        }
    }

    fn logical_not(&self) -> Self {
        self.wrap_bool(!self.vm().is_truthy_value(self.value))
    }
}

impl BitwiseOps for VmValue {
    fn bit_and(&self, rhs: &Self) -> Self {
        self.wrap(make_int32(
            self.int32_value(self.value) & self.int32_value(rhs.value),
        ))
    }

    fn bit_or(&self, rhs: &Self) -> Self {
        self.wrap(make_int32(
            self.int32_value(self.value) | self.int32_value(rhs.value),
        ))
    }

    fn bit_xor(&self, rhs: &Self) -> Self {
        self.wrap(make_int32(
            self.int32_value(self.value) ^ self.int32_value(rhs.value),
        ))
    }

    fn bit_not(&self) -> Self {
        self.wrap(make_int32(!self.int32_value(self.value)))
    }

    fn shl(&self, rhs: &Self) -> Self {
        self.wrap(make_int32(
            self.int32_value(self.value) << (self.int32_value(rhs.value) & 31),
        ))
    }

    fn shr(&self, rhs: &Self) -> Self {
        self.wrap(make_int32(
            self.int32_value(self.value) >> (self.int32_value(rhs.value) & 31),
        ))
    }

    fn ushr(&self, rhs: &Self) -> Self {
        let lhs = self.int32_value(self.value) as u32;
        let shift = (self.int32_value(rhs.value) & 31) as u32;
        self.wrap(make_number((lhs >> shift) as f64))
    }
}

impl AssignmentOps for VmValue {
    fn assign(&mut self, rhs: Self) {
        self.value = rhs.value;
    }

    fn add_assign(&mut self, rhs: Self) {
        self.value = self.add(&rhs).raw();
    }

    fn sub_assign(&mut self, rhs: Self) {
        self.value = self.sub(&rhs).raw();
    }

    fn mul_assign(&mut self, rhs: Self) {
        self.value = self.mul(&rhs).raw();
    }

    fn div_assign(&mut self, rhs: Self) {
        self.value = self.div(&rhs).raw();
    }

    fn rem_assign(&mut self, rhs: Self) {
        self.value = self.rem(&rhs).raw();
    }

    fn pow_assign(&mut self, rhs: Self) {
        self.value = self.pow(&rhs).raw();
    }

    fn shl_assign(&mut self, rhs: Self) {
        self.value = self.shl(&rhs).raw();
    }

    fn shr_assign(&mut self, rhs: Self) {
        self.value = self.shr(&rhs).raw();
    }

    fn ushr_assign(&mut self, rhs: Self) {
        self.value = self.ushr(&rhs).raw();
    }

    fn bit_and_assign(&mut self, rhs: Self) {
        self.value = self.bit_and(&rhs).raw();
    }

    fn bit_or_assign(&mut self, rhs: Self) {
        self.value = self.bit_or(&rhs).raw();
    }

    fn bit_xor_assign(&mut self, rhs: Self) {
        self.value = self.bit_xor(&rhs).raw();
    }
}

impl LogicalAssignOps for VmValue {
    fn and_assign(&mut self, rhs: Self) {
        self.value = self.logical_and(&rhs).raw();
    }

    fn or_assign(&mut self, rhs: Self) {
        self.value = self.logical_or(&rhs).raw();
    }
}

impl NullishOps for VmValue {
    fn nullish_coalesce(&self, rhs: &Self) -> Self {
        if is_null(self.value) || is_undefined(self.value) {
            *rhs
        } else {
            *self
        }
    }

    fn nullish_assign(&mut self, rhs: Self) {
        self.value = self.nullish_coalesce(&rhs).raw();
    }
}

impl TypeOps for VmValue {
    fn typeof_(&self) -> Self {
        let ty = self.vm().type_of_name(self.value);
        self.wrap(self.with_vm_mut(|vm| vm.intern_string(ty)))
    }

    fn instanceof(&self, rhs: &Self) -> Self {
        let instance = if let Some(obj_ptr) = object_from_value(self.value) {
            unsafe {
                match &(*obj_ptr).kind {
                    ObjectKind::Instance(instance) => instance.class == rhs.value,
                    _ => false,
                }
            }
        } else {
            false
        };
        self.wrap_bool(instance)
    }

    fn in_(&self, rhs: &Self) -> Self {
        self.wrap_bool(self.vm().has_property(rhs.value, self.prop_key(self.value)))
    }

    fn delete(&self) -> Self {
        self.wrap(make_true())
    }
}

impl CoercionOps for VmValue {
    fn to_number(&self) -> Self {
        self.wrap(self.with_vm_mut(|vm| vm.number_value(self.value)))
    }

    fn to_string(&self) -> Self {
        self.wrap(self.with_vm_mut(|vm| vm.string_value(self.value)))
    }

    fn to_boolean(&self) -> Self {
        self.wrap_bool(self.vm().is_truthy_value(self.value))
    }

    fn to_primitive(&self) -> Self {
        self.wrap(self.with_vm_mut(|vm| vm.primitive_value(self.value)))
    }
}

impl PropertyOps for VmValue {
    fn get(&self, key: &Self) -> Self {
        self.wrap(self.vm().get_property(self.value, self.prop_key(key.value)))
    }

    fn set(&mut self, key: Self, value: Self) {
        let _ = self
            .with_vm_mut(|vm| vm.set_property(self.value, self.prop_key(key.value), value.value));
    }

    fn has(&self, key: &Self) -> Self {
        self.wrap_bool(self.vm().has_property(self.value, self.prop_key(key.value)))
    }

    fn delete_property(&mut self, key: &Self) -> Self {
        self.wrap_bool(
            self.with_vm_mut(|vm| vm.delete_property(self.value, self.prop_key(key.value))),
        )
    }
}

impl CallOps for VmValue {
    fn call(&self, this: &Self, args: &[Self]) -> Self {
        let args: Vec<_> = args.iter().map(|arg| arg.value).collect();
        self.wrap(self.with_vm_mut(|vm| vm.call_value(self.value, this.value, &args)))
    }

    fn construct(&self, args: &[Self]) -> Self {
        let args: Vec<_> = args.iter().map(|arg| arg.value).collect();
        self.wrap(self.with_vm_mut(|vm| vm.construct_value(self.value, &args)))
    }
}

impl Ternary for VmValue {
    fn ternary(cond: &Self, a: &Self, b: &Self) -> Self {
        if cond.vm().is_truthy_value(cond.value) {
            *a
        } else {
            *b
        }
    }
}

impl ValueOps for VmValue {}

impl VM {
    pub fn new(bytecode: Vec<u32>, const_pool: Vec<JSValue>, args: Vec<JSValue>) -> Self {
        let frame = Frame::fresh(args, make_undefined(), 0, 0, None, 0);

        Self {
            frame: FrameStack::new(frame),
            pc: 0,
            bytecode,
            const_pool,
            objects: Vec::new(),
            shapes: Vec::new(),
            strings: Vec::new(),
            global_object: HashMap::new(),
            scope_chain: Vec::new(),
            upvalues: Vec::new(),
            last_exception: make_undefined(),
            interned_strings: HashMap::new(),
            atoms: AtomTable::new(),
            feedback: RuntimeFeedback::default(),
            heap_shape: Rc::new(AtomShape::new()),
            next_shape_id: 1,
            last_ic_object: None,
        }
    }

    fn value_op(&mut self, value: JSValue) -> VmValue {
        VmValue::new(self as *mut VM, value)
    }

    fn value_pair(&mut self, lhs: JSValue, rhs: JSValue) -> (VmValue, VmValue) {
        let vm = self as *mut VM;
        (VmValue::new(vm, lhs), VmValue::new(vm, rhs))
    }

    fn decode_abx(insn: u32) -> usize {
        ((insn >> 16) & 0xFFFF) as usize
    }

    fn decode_asbx(insn: u32) -> i16 {
        ((insn >> 16) & 0xFFFF) as u16 as i16
    }

    fn property_key_from_immediate(id: u16) -> PropertyKey {
        PropertyKey::Id(id)
    }

    fn property_key_from_value(&self, value: JSValue) -> PropertyKey {
        if let Some(index) = self.array_index_from_value(value) {
            PropertyKey::Index(index as u32)
        } else if let Some(string_ptr) = string_from_value(value) {
            PropertyKey::Atom(unsafe { (*string_ptr).atom })
        } else {
            PropertyKey::Value(value)
        }
    }

    fn property_key_to_value(&mut self, key: PropertyKey) -> JSValue {
        match key {
            PropertyKey::Id(id) => make_number(id as f64),
            PropertyKey::Atom(atom) => {
                let text = self.atoms.resolve(atom).to_owned();
                self.intern_string(text)
            }
            PropertyKey::Index(index) => make_number(index as f64),
            PropertyKey::Value(value) => value,
        }
    }

    fn current_shape_id(&self, obj_ptr: *mut JSObject) -> u32 {
        unsafe {
            if (*obj_ptr).shape.is_null() {
                0
            } else {
                (*(*obj_ptr).shape).id
            }
        }
    }

    fn classify_value(&self, value: JSValue) -> ValueProfileKind {
        if is_undefined(value) {
            ValueProfileKind::Undefined
        } else if is_null(value) {
            ValueProfileKind::Null
        } else if bool_from_value(value).is_some() {
            ValueProfileKind::Boolean
        } else if is_string(value) {
            ValueProfileKind::String
        } else if is_object(value) {
            if let Some(obj_ptr) = object_from_value(value) {
                unsafe {
                    match (*obj_ptr).kind {
                        ObjectKind::Function(_)
                        | ObjectKind::Closure(_)
                        | ObjectKind::NativeFunction(_)
                        | ObjectKind::NativeClosure(_)
                        | ObjectKind::Class(_) => ValueProfileKind::Function,
                        _ => ValueProfileKind::Object,
                    }
                }
            } else {
                ValueProfileKind::Object
            }
        } else {
            ValueProfileKind::Number
        }
    }

    fn ensure_type_feedback_slot(&mut self, slot: usize) -> &mut TypeFeedbackSlot {
        if slot >= self.feedback.type_slots.len() {
            self.feedback
                .type_slots
                .resize(slot + 1, TypeFeedbackSlot::default());
        }
        &mut self.feedback.type_slots[slot]
    }

    fn ensure_call_feedback_slot(&mut self, slot: usize) -> &mut CallFeedbackSlot {
        if slot >= self.feedback.call_slots.len() {
            self.feedback
                .call_slots
                .resize(slot + 1, CallFeedbackSlot::default());
        }
        &mut self.feedback.call_slots[slot]
    }

    fn observe_type_feedback_slot(&mut self, slot: usize, value: JSValue) {
        let kind = self.classify_value(value);
        self.ensure_type_feedback_slot(slot).observe(kind);
    }

    fn observe_call_feedback_kind(&mut self, slot: usize, kind: ValueProfileKind) {
        self.ensure_call_feedback_slot(slot).observe(kind);
    }

    fn observe_return_value(&mut self, value: JSValue) {
        let kind = self.classify_value(value);
        self.feedback.return_slot.observe(kind);
    }

    fn record_deopt(&mut self, reason: DeoptReason) {
        self.feedback.deopt_count = self.feedback.deopt_count.saturating_add(1);
        self.feedback.last_deopt = Some(reason);
        self.feedback.osr_active = false;
    }

    fn restore_scope_depth(&mut self, depth: usize) {
        self.scope_chain.truncate(depth);
        self.frame.header.env = None;
    }

    fn switch_table_offset(value: JSValue) -> Option<i16> {
        let offset = to_i32(value)?;
        i16::try_from(offset).ok()
    }

    fn switch_jump_offset(&self, table_index: usize, value: JSValue) -> Option<i16> {
        let case_count = usize::try_from(to_i32(*self.const_pool.get(table_index)?)?).ok()?;
        let default_offset = Self::switch_table_offset(*self.const_pool.get(table_index + 1)?)?;
        let cases = &self.const_pool.get(table_index + 2..)?;

        for pair in cases.chunks_exact(2).take(case_count) {
            let case_value = pair[0];
            let case_offset = Self::switch_table_offset(pair[1])?;
            if self.strict_equal(value, case_value) {
                return Some(case_offset);
            }
        }

        Some(default_offset)
    }

    fn alloc_shape_with(
        &mut self,
        parent: Option<*mut Shape>,
        key: Option<PropertyKey>,
        property_count: u32,
        prototype: Option<*mut Shape>,
    ) -> *mut Shape {
        let offset = parent
            .map(|shape| unsafe { (*shape).property_count })
            .unwrap_or(0);
        let shape = Box::new(Shape {
            header: GCHeader::new(ObjType::Shape),
            id: self.next_shape_id,
            parent,
            key,
            offset,
            property_count,
            prototype,
            proto_cache_offset: 0,
            proto_cache_shape: None,
        });
        self.next_shape_id += 1;
        let shape_ptr = Box::into_raw(shape);
        self.shapes.push(shape_ptr);
        shape_ptr
    }

    pub fn alloc_shape(&mut self) -> *mut Shape {
        self.alloc_shape_with(None, None, 0, None)
    }

    fn alloc_object_with_kind(&mut self, kind: ObjectKind) -> JSValue {
        let shape = self.alloc_shape();
        let obj = Box::new(JSObject {
            header: GCHeader::new(ObjType::Object),
            shape,
            properties: HashMap::new(),
            kind,
        });
        let obj_ptr = Box::into_raw(obj);
        self.objects.push(obj_ptr);
        make_object(obj_ptr)
    }

    pub fn alloc_object(&mut self) -> JSValue {
        self.alloc_object_with_kind(ObjectKind::Ordinary(QObject::new(self.heap_shape.clone())))
    }

    fn alloc_array(&mut self, size_hint: usize) -> JSValue {
        let mut array = QArray::new(self.heap_shape.clone());
        array.elements = Vec::with_capacity(size_hint);
        self.alloc_object_with_kind(ObjectKind::Array(array))
    }

    fn alloc_iterator(&mut self, values: Vec<JSValue>) -> JSValue {
        self.alloc_object_with_kind(ObjectKind::Iterator { values, index: 0 })
    }

    fn alloc_function(&mut self, descriptor: JSValue) -> JSValue {
        self.alloc_object_with_kind(ObjectKind::Function(QFunction {
            name: None,
            params: Vec::new(),
            body: Vec::new(),
            prototype: None,
            descriptor,
        }))
    }

    fn alloc_class(&mut self, base: JSValue) -> JSValue {
        self.alloc_object_with_kind(ObjectKind::Class(QClass {
            name: None,
            prototype: None,
            constructor: None,
            static_props: HashMap::new(),
            base,
        }))
    }

    fn alloc_env(&mut self) -> JSValue {
        self.alloc_object_with_kind(ObjectKind::Env(QObject::new(self.heap_shape.clone())))
    }

    pub fn intern_string(&mut self, text: impl AsRef<str>) -> JSValue {
        let text = text.as_ref();
        match self.interned_strings.entry(text.to_owned()) {
            Entry::Occupied(entry) => *entry.get(),
            Entry::Vacant(entry) => {
                let atom = self.atoms.intern(entry.key());
                let string = Box::new(JSString::new(atom));
                let string_ptr = Box::into_raw(string);
                self.strings.push(string_ptr);
                let value = make_string(string_ptr);
                entry.insert(value);
                value
            }
        }
    }

    fn string_text(&self, value: JSValue) -> Option<&str> {
        if let Some(atom) = value.as_atom() {
            Some(self.atoms.resolve(atom))
        } else {
            string_from_value(value).map(|string_ptr| unsafe { (*string_ptr).text(&self.atoms) })
        }
    }

    fn string_equals(&self, value: JSValue, expected: &str) -> bool {
        self.string_text(value) == Some(expected)
    }

    fn is_truthy_value(&self, value: JSValue) -> bool {
        if let Some(text) = self.string_text(value) {
            !text.is_empty()
        } else {
            is_truthy(value)
        }
    }

    fn display_string(&mut self, value: JSValue) -> String {
        if is_undefined(value) {
            "undefined".to_owned()
        } else if is_null(value) {
            "null".to_owned()
        } else if let Some(boolean) = bool_from_value(value) {
            if boolean {
                "true".to_owned()
            } else {
                "false".to_owned()
            }
        } else if let Some(text) = self.string_text(value) {
            text.to_owned()
        } else if let Some(number) = to_f64(value) {
            if number.is_nan() {
                "NaN".to_owned()
            } else if number.is_infinite() && number.is_sign_positive() {
                "Infinity".to_owned()
            } else if number.is_infinite() {
                "-Infinity".to_owned()
            } else if number.fract() == 0.0 {
                format!("{number:.0}")
            } else {
                number.to_string()
            }
        } else if let Some(obj_ptr) = object_from_value(value) {
            unsafe {
                match &(*obj_ptr).kind {
                    ObjectKind::Array(array) => {
                        let items = array.elements.clone();
                        let mut rendered = Vec::with_capacity(items.len());
                        for item in items {
                            rendered.push(self.display_string(item));
                        }
                        rendered.join(",")
                    }
                    ObjectKind::Function(_)
                    | ObjectKind::Closure(_)
                    | ObjectKind::NativeFunction(_)
                    | ObjectKind::NativeClosure(_) => "function() { [bytecode] }".to_owned(),
                    ObjectKind::Class(_) => "class {}".to_owned(),
                    ObjectKind::Iterator { .. } => "[object Iterator]".to_owned(),
                    ObjectKind::Env(_) => "[object Env]".to_owned(),
                    ObjectKind::Module(_) => "[object Module]".to_owned(),
                    ObjectKind::Instance(_) => "[object Instance]".to_owned(),
                    ObjectKind::Symbol(_) => "[object Symbol]".to_owned(),
                    ObjectKind::BoolArray(_)
                    | ObjectKind::Uint8Array(_)
                    | ObjectKind::Int32Array(_)
                    | ObjectKind::Float64Array(_)
                    | ObjectKind::StringArray(_) => "[object Array]".to_owned(),
                    ObjectKind::Ordinary(_) => "[object Object]".to_owned(),
                }
            }
        } else {
            "unknown".to_owned()
        }
    }

    fn primitive_value(&mut self, value: JSValue) -> JSValue {
        if is_object(value) {
            let rendered = self.display_string(value);
            self.intern_string(rendered)
        } else {
            value
        }
    }

    fn number_value(&mut self, value: JSValue) -> JSValue {
        if let Some(number) = to_f64(value) {
            make_number(number)
        } else if is_undefined(value) {
            make_number(f64::NAN)
        } else if is_null(value) {
            make_number(0.0)
        } else if let Some(boolean) = bool_from_value(value) {
            make_number(if boolean { 1.0 } else { 0.0 })
        } else if let Some(text) = self.string_text(value) {
            make_number(text.trim().parse::<f64>().unwrap_or(f64::NAN))
        } else {
            let primitive = self.primitive_value(value);
            if primitive == value {
                make_number(f64::NAN)
            } else {
                self.number_value(primitive)
            }
        }
    }

    fn string_value(&mut self, value: JSValue) -> JSValue {
        let rendered = self.display_string(value);
        self.intern_string(rendered)
    }

    fn type_of_name(&self, value: JSValue) -> &'static str {
        if is_undefined(value) {
            "undefined"
        } else if is_null(value) {
            "object"
        } else if bool_from_value(value).is_some() {
            "boolean"
        } else if is_string(value) {
            "string"
        } else if is_object(value) {
            if let Some(obj_ptr) = object_from_value(value) {
                unsafe {
                    match (*obj_ptr).kind {
                        ObjectKind::Function(_)
                        | ObjectKind::Closure(_)
                        | ObjectKind::NativeFunction(_)
                        | ObjectKind::NativeClosure(_)
                        | ObjectKind::Class(_) => "function",
                        _ => "object",
                    }
                }
            } else {
                "object"
            }
        } else {
            "number"
        }
    }

    fn strict_equal(&self, lhs: JSValue, rhs: JSValue) -> bool {
        if is_string(lhs) && is_string(rhs) {
            return self.string_text(lhs) == self.string_text(rhs);
        }

        if is_object(lhs) && is_object(rhs) {
            return object_from_value(lhs) == object_from_value(rhs);
        }

        if let (Some(left), Some(right)) = (to_f64(lhs), to_f64(rhs)) {
            return !left.is_nan() && !right.is_nan() && left == right;
        }

        if let (Some(left), Some(right)) = (bool_from_value(lhs), bool_from_value(rhs)) {
            return left == right;
        }

        lhs == rhs
    }

    fn abstract_equal(&mut self, lhs: JSValue, rhs: JSValue) -> bool {
        if self.strict_equal(lhs, rhs) {
            return true;
        }

        if (is_null(lhs) && is_undefined(rhs)) || (is_undefined(lhs) && is_null(rhs)) {
            return true;
        }

        if bool_from_value(lhs).is_some() {
            let lhs = self.number_value(lhs);
            return self.abstract_equal(lhs, rhs);
        }

        if bool_from_value(rhs).is_some() {
            let rhs = self.number_value(rhs);
            return self.abstract_equal(lhs, rhs);
        }

        if (is_string(lhs) && to_f64(rhs).is_some()) || (to_f64(lhs).is_some() && is_string(rhs)) {
            let left = to_f64(self.number_value(lhs)).unwrap_or(f64::NAN);
            let right = to_f64(self.number_value(rhs)).unwrap_or(f64::NAN);
            return !left.is_nan() && !right.is_nan() && left == right;
        }

        if is_object(lhs) {
            let lhs = self.primitive_value(lhs);
            return self.abstract_equal(lhs, rhs);
        }

        if is_object(rhs) {
            let rhs = self.primitive_value(rhs);
            return self.abstract_equal(lhs, rhs);
        }

        false
    }

    fn less_than(&mut self, lhs: JSValue, rhs: JSValue) -> bool {
        if is_string(lhs) && is_string(rhs) {
            return self.string_text(lhs) < self.string_text(rhs);
        }

        let left = to_f64(self.number_value(lhs)).unwrap_or(f64::NAN);
        let right = to_f64(self.number_value(rhs)).unwrap_or(f64::NAN);
        !left.is_nan() && !right.is_nan() && left < right
    }

    fn less_than_or_equal(&mut self, lhs: JSValue, rhs: JSValue) -> bool {
        self.less_than(lhs, rhs) || self.strict_equal(lhs, rhs)
    }

    fn binary_add(&mut self, lhs: JSValue, rhs: JSValue) -> JSValue {
        if is_string(lhs) || is_string(rhs) {
            let result = format!("{}{}", self.display_string(lhs), self.display_string(rhs));
            self.intern_string(result)
        } else {
            let left = to_f64(self.number_value(lhs)).unwrap_or(f64::NAN);
            let right = to_f64(self.number_value(rhs)).unwrap_or(f64::NAN);
            make_number(left + right)
        }
    }

    fn binary_numeric_op<F>(&mut self, lhs: JSValue, rhs: JSValue, op: F) -> JSValue
    where
        F: FnOnce(f64, f64) -> f64,
    {
        let left = to_f64(self.number_value(lhs)).unwrap_or(f64::NAN);
        let right = to_f64(self.number_value(rhs)).unwrap_or(f64::NAN);
        make_number(op(left, right))
    }

    fn array_index_from_value(&self, value: JSValue) -> Option<usize> {
        let number = to_f64(value)?;
        if number.is_finite() && number >= 0.0 && number.fract() == 0.0 {
            Some(number as usize)
        } else {
            None
        }
    }

    fn property_is_length(&self, key: PropertyKey) -> bool {
        match key {
            PropertyKey::Atom(atom) => self.atoms.resolve(atom) == "length",
            PropertyKey::Value(value) => self.string_equals(value, "length"),
            _ => false,
        }
    }

    fn get_length_value(&self, value: JSValue) -> JSValue {
        if let Some(obj_ptr) = object_from_value(value) {
            unsafe {
                match &(*obj_ptr).kind {
                    ObjectKind::Array(array) => make_number(array.elements.len() as f64),
                    _ => make_number((*obj_ptr).properties.len() as f64),
                }
            }
        } else if let Some(text) = self.string_text(value) {
            make_number(text.chars().count() as f64)
        } else {
            make_number(0.0)
        }
    }

    fn transition_shape_if_needed(&mut self, obj_ptr: *mut JSObject, key: PropertyKey) {
        let is_named_key = !matches!(key, PropertyKey::Index(_));
        let should_transition =
            unsafe { is_named_key && !(*obj_ptr).properties.contains_key(&key) };
        if !should_transition {
            return;
        }

        let (parent, property_count, prototype) = unsafe {
            let parent = (*obj_ptr).shape;
            let next_property_count = if parent.is_null() {
                1
            } else {
                (*parent).property_count + 1
            };
            let prototype = if parent.is_null() {
                None
            } else {
                (*parent).prototype
            };
            (
                if parent.is_null() { None } else { Some(parent) },
                next_property_count,
                prototype,
            )
        };

        let new_shape = self.alloc_shape_with(parent, Some(key), property_count, prototype);
        unsafe {
            (*obj_ptr).shape = new_shape;
        }
    }

    pub fn obj_get_prop(&self, obj_val: JSValue, key_id: u16) -> JSValue {
        self.get_property(obj_val, PropertyKey::Id(key_id))
    }

    pub fn obj_set_prop(&mut self, obj_val: JSValue, key_id: u16, value: JSValue) {
        let _ = self.set_property(obj_val, PropertyKey::Id(key_id), value);
    }

    fn get_property(&self, obj_val: JSValue, key: PropertyKey) -> JSValue {
        let Some(obj_ptr) = object_from_value(obj_val) else {
            return make_undefined();
        };

        unsafe {
            match &(*obj_ptr).kind {
                ObjectKind::Array(array) => match key {
                    PropertyKey::Index(index) => array
                        .elements
                        .get(index as usize)
                        .copied()
                        .unwrap_or(make_undefined()),
                    _ if self.property_is_length(key) => make_number(array.elements.len() as f64),
                    _ => (*obj_ptr)
                        .properties
                        .get(&key)
                        .copied()
                        .unwrap_or(make_undefined()),
                },
                _ => (*obj_ptr)
                    .properties
                    .get(&key)
                    .copied()
                    .unwrap_or(make_undefined()),
            }
        }
    }

    fn set_property(&mut self, obj_val: JSValue, key: PropertyKey, value: JSValue) -> JSValue {
        let Some(obj_ptr) = object_from_value(obj_val) else {
            return make_undefined();
        };

        unsafe {
            if let ObjectKind::Array(array) = &mut (*obj_ptr).kind {
                match key {
                    PropertyKey::Index(index) => {
                        let index = index as usize;
                        if index >= array.elements.len() {
                            array.elements.resize(index + 1, make_undefined());
                        }
                        array.elements[index] = value;
                        return value;
                    }
                    _ if self.property_is_length(key) => {
                        let numeric_value = self.number_value(value);
                        let length = self.array_index_from_value(numeric_value).unwrap_or(0);
                        array.elements.resize(length, make_undefined());
                        return make_number(array.elements.len() as f64);
                    }
                    _ => {}
                }
            }
        }

        self.transition_shape_if_needed(obj_ptr, key);
        unsafe {
            (*obj_ptr).properties.insert(key, value);
        }
        value
    }

    fn delete_property(&mut self, obj_val: JSValue, key: PropertyKey) -> bool {
        let Some(obj_ptr) = object_from_value(obj_val) else {
            return false;
        };

        unsafe {
            match &mut (*obj_ptr).kind {
                ObjectKind::Array(array) => match key {
                    PropertyKey::Index(index) => {
                        let index = index as usize;
                        if let Some(slot) = array.elements.get_mut(index) {
                            *slot = make_undefined();
                            return true;
                        }
                        false
                    }
                    _ if self.property_is_length(key) => false,
                    _ => (*obj_ptr).properties.remove(&key).is_some(),
                },
                _ => (*obj_ptr).properties.remove(&key).is_some(),
            }
        }
    }

    fn has_property(&self, obj_val: JSValue, key: PropertyKey) -> bool {
        let Some(obj_ptr) = object_from_value(obj_val) else {
            return false;
        };

        unsafe {
            match &(*obj_ptr).kind {
                ObjectKind::Array(array) => match key {
                    PropertyKey::Index(index) => array.elements.get(index as usize).is_some(),
                    _ if self.property_is_length(key) => true,
                    _ => (*obj_ptr).properties.contains_key(&key),
                },
                _ => (*obj_ptr).properties.contains_key(&key),
            }
        }
    }

    fn get_keys(&self, obj_val: JSValue) -> Vec<PropertyKey> {
        let Some(obj_ptr) = object_from_value(obj_val) else {
            return Vec::new();
        };

        unsafe {
            match &(*obj_ptr).kind {
                ObjectKind::Array(array) => {
                    let mut keys =
                        Vec::with_capacity(array.elements.len() + (*obj_ptr).properties.len());
                    for index in 0..array.elements.len() {
                        keys.push(PropertyKey::Index(index as u32));
                    }
                    let mut named: Vec<_> = (*obj_ptr).properties.keys().copied().collect();
                    named.sort_by_key(PropertyKey::sort_key);
                    keys.extend(named);
                    keys
                }
                _ => {
                    let mut keys: Vec<_> = (*obj_ptr).properties.keys().copied().collect();
                    keys.sort_by_key(PropertyKey::sort_key);
                    keys
                }
            }
        }
    }

    fn array_push(&mut self, array_val: JSValue, value: JSValue) -> JSValue {
        let Some(obj_ptr) = object_from_value(array_val) else {
            return make_undefined();
        };

        unsafe {
            if let ObjectKind::Array(array) = &mut (*obj_ptr).kind {
                array.push(value);
                return make_number(array.elements.len() as f64);
            }
        }

        make_undefined()
    }

    fn array_values(&self, value: JSValue) -> Option<Vec<JSValue>> {
        let obj_ptr = object_from_value(value)?;
        unsafe {
            match &(*obj_ptr).kind {
                ObjectKind::Array(array) => Some(array.elements.clone()),
                ObjectKind::Iterator { values, .. } => Some(values.clone()),
                _ => None,
            }
        }
    }

    fn iterator_next_value(&mut self, iterator_val: JSValue) -> JSValue {
        let Some(obj_ptr) = object_from_value(iterator_val) else {
            return make_undefined();
        };

        unsafe {
            if let ObjectKind::Iterator { values, index } = &mut (*obj_ptr).kind
                && *index < values.len()
            {
                let value = values[*index];
                *index += 1;
                return value;
            }
        }

        make_undefined()
    }

    fn scope_at_depth(&self, depth: usize) -> Option<JSValue> {
        self.scope_chain
            .len()
            .checked_sub(depth + 1)
            .and_then(|index| self.scope_chain.get(index).copied())
    }

    fn set_scope_at_depth(&mut self, depth: usize, value: JSValue) {
        if let Some(index) = self.scope_chain.len().checked_sub(depth + 1)
            && index < self.scope_chain.len()
        {
            self.scope_chain[index] = value;
            return;
        }

        if depth == 0 {
            self.scope_chain.push(value);
        }
    }

    fn resolve_scope_value(&self, name: u16) -> Option<JSValue> {
        self.scope_chain
            .iter()
            .rev()
            .find(|&&scope| self.has_property(scope, PropertyKey::Id(name)))
            .copied()
    }

    fn load_name_value(&self, name: u16) -> JSValue {
        if let Some(scope) = self.resolve_scope_value(name) {
            self.get_property(scope, PropertyKey::Id(name))
        } else {
            self.global_object
                .get(&name)
                .copied()
                .unwrap_or(make_undefined())
        }
    }

    fn store_name_value(&mut self, name: u16, value: JSValue) {
        if let Some(scope) = self.resolve_scope_value(name) {
            let _ = self.set_property(scope, PropertyKey::Id(name), value);
        } else if let Some(&scope) = self.scope_chain.last() {
            let _ = self.set_property(scope, PropertyKey::Id(name), value);
        } else {
            self.global_object.insert(name, value);
        }
    }

    fn ensure_upvalue_slot(&mut self, slot: usize) {
        if slot >= self.upvalues.len() {
            self.upvalues.resize(slot + 1, make_undefined());
        }
    }

    fn collect_call_args(&self, start: usize, count: usize) -> Vec<JSValue> {
        match count {
            0 => Vec::new(),
            1 => vec![
                self.frame
                    .regs
                    .get(start)
                    .copied()
                    .unwrap_or(make_undefined()),
            ],
            _ => (0..count)
                .map(|index| {
                    self.frame
                        .regs
                        .get(start + index)
                        .copied()
                        .unwrap_or(make_undefined())
                })
                .collect(),
        }
    }

    fn function_entry_pc(&self, descriptor: JSValue) -> Option<usize> {
        let entry = to_f64(descriptor)?;
        if !entry.is_finite() || entry < 0.0 || entry.fract() != 0.0 {
            return None;
        }

        let entry = entry as usize;
        (entry < self.bytecode.len()).then_some(entry)
    }

    #[inline(always)]
    fn call_value(&mut self, callee: JSValue, this_value: JSValue, args: &[JSValue]) -> JSValue {
        let caller_depth = self.frame.depth();
        match self.dispatch_call_value(callee, this_value, args) {
            CallAction::Returned(result) => result,
            CallAction::EnteredFrame => {
                self.run_until_frame_depth(caller_depth);
                self.frame.regs[ACC]
            }
        }
    }

    #[inline(always)]
    fn construct_value(&mut self, callee: JSValue, args: &[JSValue]) -> JSValue {
        let caller_depth = self.frame.depth();
        match self.dispatch_construct(callee, args) {
            CallAction::Returned(result) => result,
            CallAction::EnteredFrame => {
                self.run_until_frame_depth(caller_depth);
                self.frame.regs[ACC]
            }
        }
    }

    #[inline(always)]
    fn invoke_call(&mut self, callee_reg: usize, arg_count: usize) -> CallAction {
        let callee = self.frame.regs[callee_reg];

        // 🔥 FAST PATH: 1 argument (common case for recursive fib)
        if arg_count == 1 {
            let arg0 = self.frame.regs[callee_reg + 1];
            return self.dispatch_call_value(callee, self.frame.regs[0], &[arg0]);
        }

        // Fallback (multi-arg, uncommon)
        let args = self.collect_call_args(callee_reg + 1, arg_count);
        self.dispatch_call_value(callee, self.frame.regs[0], &args)
    }

    fn invoke_method_call(
        &mut self,
        callee: JSValue,
        this_value: JSValue,
        arg_count: usize,
        arg_base: usize,
    ) -> CallAction {
        let args = self.collect_call_args(arg_base, arg_count);
        self.dispatch_call_value(callee, this_value, &args)
    }

    fn invoke_spread_call(&mut self, callee_reg: usize, array_reg: usize) -> CallAction {
        let callee = self.frame.regs[callee_reg];
        let args = self
            .array_values(
                self.frame
                    .regs
                    .get(array_reg)
                    .copied()
                    .unwrap_or(make_undefined()),
            )
            .unwrap_or_default();
        self.dispatch_call_value(callee, self.frame.regs[0], &args)
    }

    fn invoke_construct(&mut self, callee_reg: usize, arg_count: usize) -> CallAction {
        let callee = self.frame.regs[callee_reg];
        let args = self.collect_call_args(callee_reg + 1, arg_count);
        self.dispatch_construct(callee, &args)
    }

    #[inline(always)]
    fn enter_frame(
        &mut self,
        entry_pc: usize,
        this_value: JSValue,
        args: &[JSValue],
        construct_result: Option<JSValue>,
    ) {
        let return_pc = self.pc;
        let scope_depth = self.scope_chain.len();
        let callee_frame = self.frame.ensure_next_frame();
        callee_frame.reset(
            args,
            this_value,
            entry_pc,
            return_pc,
            construct_result,
            scope_depth,
        );
        self.pc = entry_pc;
    }

    #[inline(always)]
    fn exit_frame(&mut self, result: JSValue) -> bool {
        let result = match self.frame.header.construct_result {
            Some(instance) if !is_object(result) => instance,
            _ => result,
        };
        let return_pc = self.frame.header.return_pc;
        let scope_depth = self.frame.header.scope_depth;
        self.restore_scope_depth(scope_depth);

        if self.frame.pop_frame() {
            self.frame.regs[ACC] = result;
            self.pc = return_pc;
            true
        } else {
            self.frame.regs[ACC] = result;
            false
        }
    }

    fn dispatch_call_value(
        &mut self,
        callee: JSValue,
        this_value: JSValue,
        args: &[JSValue],
    ) -> CallAction {
        self.feedback.last_call_kind = Some(self.classify_value(callee));
        let Some(obj_ptr) = object_from_value(callee) else {
            return CallAction::Returned(make_undefined());
        };

        unsafe {
            match &(*obj_ptr).kind {
                ObjectKind::Function(function) => {
                    let descriptor = function.descriptor;
                    if let Some(entry_pc) = self.function_entry_pc(descriptor) {
                        self.enter_frame(entry_pc, this_value, args, None);
                        CallAction::EnteredFrame
                    } else if is_undefined(descriptor) {
                        CallAction::Returned(args.first().copied().unwrap_or(this_value))
                    } else {
                        CallAction::Returned(descriptor)
                    }
                }
                ObjectKind::Class(class) => {
                    let base = class.base;
                    let instance = self.alloc_object_with_kind(ObjectKind::Instance(QInstance {
                        class: callee,
                        object: QObject::new(self.heap_shape.clone()),
                    }));
                    let _ = self.set_property(instance, PropertyKey::Id(0), base);
                    CallAction::Returned(instance)
                }
                _ => CallAction::Returned(make_undefined()),
            }
        }
    }

    fn dispatch_construct(&mut self, callee: JSValue, args: &[JSValue]) -> CallAction {
        self.feedback.last_call_kind = Some(self.classify_value(callee));
        let Some(obj_ptr) = object_from_value(callee) else {
            return CallAction::Returned(self.alloc_object());
        };

        unsafe {
            match &(*obj_ptr).kind {
                ObjectKind::Function(function) => {
                    let descriptor = function.descriptor;
                    let instance = self.alloc_object();
                    if let Some(entry_pc) = self.function_entry_pc(descriptor) {
                        self.enter_frame(entry_pc, instance, args, Some(instance));
                        CallAction::EnteredFrame
                    } else {
                        CallAction::Returned(instance)
                    }
                }
                ObjectKind::Class(class) => {
                    let base = class.base;
                    let instance = self.alloc_object_with_kind(ObjectKind::Instance(QInstance {
                        class: callee,
                        object: QObject::new(self.heap_shape.clone()),
                    }));
                    let _ = self.set_property(instance, PropertyKey::Id(0), base);
                    CallAction::Returned(instance)
                }
                _ => CallAction::Returned(self.alloc_object()),
            }
        }
    }

    fn jump_by(&mut self, offset: i16) {
        let next_pc = (self.pc as isize + offset as isize).clamp(0, self.bytecode.len() as isize);
        self.pc = next_pc as usize;
    }

    fn ensure_ic_slot(&mut self, slot: usize) -> &mut InlineCache {
        if slot >= self.frame.ic_vector.len() {
            self.frame
                .ic_vector
                .resize(slot + 1, InlineCache::default());
        }
        &mut self.frame.ic_vector[slot]
    }

    fn ic_has_shape(ic: &InlineCache, shape_id: u32) -> bool {
        match ic.state {
            ICState::Uninit => false,
            ICState::Mono => ic.shape_id == shape_id,
            ICState::Poly => ic.shape_id == shape_id || ic.shapes.contains(&shape_id),
            ICState::Mega => false,
        }
    }

    fn check_ic_slot(&self, slot: usize, obj_ptr: *mut JSObject) -> bool {
        let Some(ic) = self.frame.ic_vector.get(slot) else {
            return false;
        };
        Self::ic_has_shape(ic, self.current_shape_id(obj_ptr))
    }

    fn cached_ic_hit(&self, slot: usize, obj_ptr: *mut JSObject, key: PropertyKey) -> bool {
        let Some(ic) = self.frame.ic_vector.get(slot) else {
            return false;
        };
        Self::ic_has_shape(ic, self.current_shape_id(obj_ptr)) && ic.key == Some(key)
    }

    fn init_ic_slot(&mut self, slot: usize, obj_ptr: *mut JSObject, key: Option<PropertyKey>) {
        let shape_id = self.current_shape_id(obj_ptr);
        let ic = self.ensure_ic_slot(slot);
        let preserved_key = ic.key;
        ic.state = ICState::Mono;
        ic.shape_id = shape_id;
        ic.offset = 0;
        ic.key = key.or(preserved_key);
        ic.shapes.clear();
    }

    fn update_ic_slot(&mut self, slot: usize, obj_ptr: *mut JSObject, key: PropertyKey) {
        let shape_id = self.current_shape_id(obj_ptr);
        let ic = self.ensure_ic_slot(slot);

        if ic.key != Some(key) {
            ic.state = ICState::Mono;
            ic.shape_id = shape_id;
            ic.offset = 0;
            ic.key = Some(key);
            ic.shapes.clear();
            return;
        }

        match ic.state {
            ICState::Uninit => {
                ic.state = ICState::Mono;
                ic.shape_id = shape_id;
                ic.offset = 0;
                ic.shapes.clear();
            }
            ICState::Mono => {
                if ic.shape_id != shape_id {
                    ic.state = ICState::Poly;
                    ic.shapes.clear();
                    ic.shapes.push(shape_id);
                }
            }
            ICState::Poly => {
                if ic.shape_id != shape_id && !ic.shapes.contains(&shape_id) {
                    if ic.shapes.len() < 3 {
                        ic.shapes.push(shape_id);
                    } else {
                        ic.state = ICState::Mega;
                        ic.shapes.clear();
                    }
                }
            }
            ICState::Mega => {}
        }
    }

    pub fn collect_garbage(&mut self) {
        gc::collect_garbage(self);
    }

    fn run_inner(&mut self, stop_at_depth: Option<usize>) {
        while self.pc < self.bytecode.len() {
            let insn = self.bytecode[self.pc];
            self.pc += 1;

            let opcode = Opcode::from((insn & 0xFF) as u8);
            let a = ((insn >> 8) & 0xFF) as usize;
            let b = ((insn >> 16) & 0xFF) as usize;
            let c = ((insn >> 24) & 0xFF) as usize;

            match opcode {
                Opcode::Mov => {
                    self.frame.regs[a] = self.frame.regs[b];
                }
                Opcode::LoadK => {
                    let index = Self::decode_abx(insn);
                    self.frame.regs[a] = self
                        .const_pool
                        .get(index)
                        .copied()
                        .unwrap_or(make_undefined());
                }
                Opcode::Add => {
                    let (lhs, rhs) = self.value_pair(self.frame.regs[b], self.frame.regs[c]);
                    self.frame.regs[ACC] = lhs.add(&rhs).raw();
                }
                Opcode::GetPropIc => {
                    let key = Self::property_key_from_immediate(c as u16);
                    let obj_val = self.frame.regs[b];
                    self.feedback.last_ic_slot = Some(c);
                    let result = if let Some(obj_ptr) = object_from_value(obj_val) {
                        self.last_ic_object = Some(obj_ptr);
                        let value = self.get_property(obj_val, key);
                        if !self.cached_ic_hit(c, obj_ptr, key) {
                            self.update_ic_slot(c, obj_ptr, key);
                        }
                        value
                    } else {
                        make_undefined()
                    };
                    self.frame.regs[a] = result;
                }
                Opcode::Call => match self.invoke_call(a, b) {
                    CallAction::Returned(result) => self.frame.regs[ACC] = result,
                    CallAction::EnteredFrame => continue,
                },
                Opcode::Call1SubI => {
                    let callee = self.frame.regs[a];
                    let arg = self.binary_numeric_op(
                        self.frame.regs[b],
                        make_int32(c as i8 as i32),
                        |x, y| x - y,
                    );
                    match self.dispatch_call_value(callee, self.frame.regs[0], &[arg]) {
                        CallAction::Returned(result) => self.frame.regs[ACC] = result,
                        CallAction::EnteredFrame => continue,
                    }
                }
                Opcode::Jmp => {
                    self.jump_by(Self::decode_asbx(insn));
                }
                Opcode::LoadI => {
                    self.frame.regs[a] = make_int32(Self::decode_asbx(insn) as i32);
                }
                Opcode::JmpTrue => {
                    if self.is_truthy_value(self.frame.regs[a]) {
                        self.jump_by(Self::decode_asbx(insn));
                    }
                }
                Opcode::JmpFalse => {
                    if !self.is_truthy_value(self.frame.regs[a]) {
                        self.jump_by(Self::decode_asbx(insn));
                    }
                }
                Opcode::SetPropIc => {
                    let key = Self::property_key_from_immediate(c as u16);
                    let obj_val = self.frame.regs[b];
                    let value = self.frame.regs[a];
                    self.feedback.last_ic_slot = Some(c);
                    let result = if let Some(obj_ptr) = object_from_value(obj_val) {
                        self.last_ic_object = Some(obj_ptr);
                        let written = self.set_property(obj_val, key, value);
                        self.update_ic_slot(c, obj_ptr, key);
                        written
                    } else {
                        make_undefined()
                    };
                    self.frame.regs[ACC] = result;
                }
                Opcode::AddAccImm8 => {
                    let (lhs, rhs) =
                        self.value_pair(self.frame.regs[ACC], make_int32(b as i8 as i32));
                    self.frame.regs[ACC] = lhs.add(&rhs).raw();
                }
                Opcode::IncAcc => {
                    self.frame.regs[ACC] = self.value_op(self.frame.regs[ACC]).inc().raw();
                }
                Opcode::LoadThis => {
                    self.frame.regs[ACC] = self.frame.regs[0];
                }
                Opcode::Load0 => {
                    self.frame.regs[ACC] = make_int32(0);
                }
                Opcode::Load1 => {
                    self.frame.regs[ACC] = make_int32(1);
                }
                Opcode::Eq => {
                    let (lhs, rhs) = self.value_pair(self.frame.regs[b], self.frame.regs[c]);
                    self.frame.regs[ACC] = lhs.eq(&rhs).raw();
                }
                Opcode::Lt => {
                    let (lhs, rhs) = self.value_pair(self.frame.regs[b], self.frame.regs[c]);
                    self.frame.regs[ACC] = lhs.lt(&rhs).raw();
                }
                Opcode::Lte => {
                    let (lhs, rhs) = self.value_pair(self.frame.regs[b], self.frame.regs[c]);
                    self.frame.regs[ACC] = lhs.le(&rhs).raw();
                }
                Opcode::AddAcc => {
                    let (lhs, rhs) = self.value_pair(self.frame.regs[ACC], self.frame.regs[b]);
                    self.frame.regs[ACC] = lhs.add(&rhs).raw();
                }
                Opcode::SubAcc => {
                    let (lhs, rhs) = self.value_pair(self.frame.regs[ACC], self.frame.regs[b]);
                    self.frame.regs[ACC] = lhs.sub(&rhs).raw();
                }
                Opcode::MulAcc => {
                    let (lhs, rhs) = self.value_pair(self.frame.regs[ACC], self.frame.regs[b]);
                    self.frame.regs[ACC] = lhs.mul(&rhs).raw();
                }
                Opcode::DivAcc => {
                    let (lhs, rhs) = self.value_pair(self.frame.regs[ACC], self.frame.regs[b]);
                    self.frame.regs[ACC] = lhs.div(&rhs).raw();
                }
                Opcode::LoadNull => {
                    self.frame.regs[ACC] = make_null();
                }
                Opcode::LoadTrue => {
                    self.frame.regs[ACC] = make_true();
                }
                Opcode::LoadFalse => {
                    self.frame.regs[ACC] = make_false();
                }
                Opcode::LoadGlobalIc | Opcode::GetGlobal => {
                    let key = Self::decode_abx(insn) as u16;
                    self.frame.regs[a] = self
                        .global_object
                        .get(&key)
                        .copied()
                        .unwrap_or(make_undefined());
                }
                Opcode::SetGlobalIc | Opcode::SetGlobal => {
                    let key = Self::decode_abx(insn) as u16;
                    self.global_object.insert(key, self.frame.regs[a]);
                }
                Opcode::Typeof => {
                    self.frame.regs[a] = self.value_op(self.frame.regs[b]).typeof_().raw();
                }
                Opcode::ToNum => {
                    self.frame.regs[a] = self.value_op(self.frame.regs[b]).to_number().raw();
                }
                Opcode::ToStr => {
                    self.frame.regs[a] = self.value_op(self.frame.regs[b]).to_string().raw();
                }
                Opcode::IsUndef => {
                    self.frame.regs[a] = make_bool(is_undefined(self.frame.regs[b]));
                }
                Opcode::IsNull => {
                    self.frame.regs[a] = make_bool(is_null(self.frame.regs[b]));
                }
                Opcode::SubAccImm8 => {
                    let (lhs, rhs) =
                        self.value_pair(self.frame.regs[ACC], make_int32(b as i8 as i32));
                    self.frame.regs[ACC] = lhs.sub(&rhs).raw();
                }
                Opcode::MulAccImm8 => {
                    let (lhs, rhs) =
                        self.value_pair(self.frame.regs[ACC], make_int32(b as i8 as i32));
                    self.frame.regs[ACC] = lhs.mul(&rhs).raw();
                }
                Opcode::DivAccImm8 => {
                    let (lhs, rhs) =
                        self.value_pair(self.frame.regs[ACC], make_int32(b as i8 as i32));
                    self.frame.regs[ACC] = lhs.div(&rhs).raw();
                }
                Opcode::AddStrAcc => {
                    let result = format!(
                        "{}{}",
                        self.display_string(self.frame.regs[ACC]),
                        self.display_string(self.frame.regs[b])
                    );
                    self.frame.regs[ACC] = self.intern_string(result);
                }
                Opcode::AddI => {
                    let result = self.binary_numeric_op(
                        self.frame.regs[b],
                        make_int32(c as i8 as i32),
                        |x, y| x + y,
                    );
                    self.frame.regs[ACC] = result;
                    if a != ACC {
                        self.frame.regs[a] = result;
                    }
                }
                Opcode::SubI => {
                    let result = self.binary_numeric_op(
                        self.frame.regs[b],
                        make_int32(c as i8 as i32),
                        |x, y| x - y,
                    );
                    self.frame.regs[ACC] = result;
                    if a != ACC {
                        self.frame.regs[a] = result;
                    }
                }
                Opcode::MulI => {
                    let result = self.binary_numeric_op(
                        self.frame.regs[b],
                        make_int32(c as i8 as i32),
                        |x, y| x * y,
                    );
                    self.frame.regs[ACC] = result;
                    if a != ACC {
                        self.frame.regs[a] = result;
                    }
                }
                Opcode::DivI => {
                    let result = self.binary_numeric_op(
                        self.frame.regs[b],
                        make_int32(c as i8 as i32),
                        |x, y| x / y,
                    );
                    self.frame.regs[ACC] = result;
                    if a != ACC {
                        self.frame.regs[a] = result;
                    }
                }
                Opcode::ModI => {
                    let result = self.binary_numeric_op(
                        self.frame.regs[b],
                        make_int32(c as i8 as i32),
                        |x, y| x % y,
                    );
                    self.frame.regs[ACC] = result;
                    if a != ACC {
                        self.frame.regs[a] = result;
                    }
                }
                Opcode::Neg => {
                    self.frame.regs[ACC] = self.value_op(self.frame.regs[b]).unary_minus().raw();
                }
                Opcode::Inc => {
                    self.frame.regs[ACC] = self.value_op(self.frame.regs[b]).inc().raw();
                }
                Opcode::Dec => {
                    self.frame.regs[ACC] = self.value_op(self.frame.regs[b]).dec().raw();
                }
                Opcode::AddStr => {
                    let result = format!(
                        "{}{}",
                        self.display_string(self.frame.regs[b]),
                        self.display_string(self.frame.regs[c])
                    );
                    self.frame.regs[ACC] = self.intern_string(result);
                }
                Opcode::ToPrimitive => {
                    self.frame.regs[ACC] = self.value_op(self.frame.regs[b]).to_primitive().raw();
                }
                Opcode::GetPropAcc => {
                    let base = self.value_op(self.frame.regs[b]);
                    let key = self.value_op(self.frame.regs[c]);
                    self.frame.regs[ACC] = base.get(&key).raw();
                }
                Opcode::SetPropAcc => {
                    let key = self.property_key_from_value(self.frame.regs[c]);
                    self.frame.regs[ACC] =
                        self.set_property(self.frame.regs[b], key, self.frame.regs[ACC]);
                }
                Opcode::GetIdxFast | Opcode::GetIdxIc => {
                    let key = self.property_key_from_value(self.frame.regs[c]);
                    let base = self.value_op(self.frame.regs[b]);
                    let key_value = self.value_op(self.frame.regs[c]);
                    let result = base.get(&key_value).raw();
                    if matches!(opcode, Opcode::GetIdxIc)
                        && let Some(obj_ptr) = object_from_value(self.frame.regs[b])
                    {
                        self.feedback.last_ic_slot = Some(c);
                        self.last_ic_object = Some(obj_ptr);
                        self.update_ic_slot(c, obj_ptr, key);
                    }
                    self.frame.regs[a] = result;
                }
                Opcode::SetIdxFast | Opcode::SetIdxIc => {
                    let key = self.property_key_from_value(self.frame.regs[c]);
                    let result = self.set_property(self.frame.regs[b], key, self.frame.regs[a]);
                    if matches!(opcode, Opcode::SetIdxIc)
                        && let Some(obj_ptr) = object_from_value(self.frame.regs[b])
                    {
                        self.feedback.last_ic_slot = Some(c);
                        self.last_ic_object = Some(obj_ptr);
                        self.update_ic_slot(c, obj_ptr, key);
                    }
                    self.frame.regs[ACC] = result;
                }
                Opcode::LoadArg => {
                    self.frame.regs[a] = self.frame.arg(b);
                }
                Opcode::LoadAcc => {
                    self.frame.regs[ACC] = self.frame.regs[a];
                }
                Opcode::StrictEq => {
                    let (lhs, rhs) = self.value_pair(self.frame.regs[b], self.frame.regs[c]);
                    self.frame.regs[ACC] = lhs.strict_eq(&rhs).raw();
                }
                Opcode::StrictNeq => {
                    let (lhs, rhs) = self.value_pair(self.frame.regs[b], self.frame.regs[c]);
                    self.frame.regs[ACC] = lhs.strict_ne(&rhs).raw();
                }
                Opcode::BitAnd => {
                    let (lhs, rhs) = self.value_pair(self.frame.regs[b], self.frame.regs[c]);
                    self.frame.regs[ACC] = lhs.bit_and(&rhs).raw();
                }
                Opcode::BitOr => {
                    let (lhs, rhs) = self.value_pair(self.frame.regs[b], self.frame.regs[c]);
                    self.frame.regs[ACC] = lhs.bit_or(&rhs).raw();
                }
                Opcode::BitXor => {
                    let (lhs, rhs) = self.value_pair(self.frame.regs[b], self.frame.regs[c]);
                    self.frame.regs[ACC] = lhs.bit_xor(&rhs).raw();
                }
                Opcode::BitNot => {
                    self.frame.regs[ACC] = self.value_op(self.frame.regs[b]).bit_not().raw();
                }
                Opcode::Shl => {
                    let (lhs, rhs) = self.value_pair(self.frame.regs[b], self.frame.regs[c]);
                    self.frame.regs[ACC] = lhs.shl(&rhs).raw();
                }
                Opcode::Shr => {
                    let (lhs, rhs) = self.value_pair(self.frame.regs[b], self.frame.regs[c]);
                    self.frame.regs[ACC] = lhs.shr(&rhs).raw();
                }
                Opcode::Ushr => {
                    let (lhs, rhs) = self.value_pair(self.frame.regs[b], self.frame.regs[c]);
                    self.frame.regs[ACC] = lhs.ushr(&rhs).raw();
                }
                Opcode::Pow => {
                    let (lhs, rhs) = self.value_pair(self.frame.regs[b], self.frame.regs[c]);
                    self.frame.regs[ACC] = lhs.pow(&rhs).raw();
                }
                Opcode::LogicalAnd => {
                    let (lhs, rhs) = self.value_pair(self.frame.regs[b], self.frame.regs[c]);
                    self.frame.regs[ACC] = lhs.logical_and(&rhs).raw();
                }
                Opcode::LogicalOr => {
                    let (lhs, rhs) = self.value_pair(self.frame.regs[b], self.frame.regs[c]);
                    self.frame.regs[ACC] = lhs.logical_or(&rhs).raw();
                }
                Opcode::NullishCoalesce => {
                    let (lhs, rhs) = self.value_pair(self.frame.regs[b], self.frame.regs[c]);
                    self.frame.regs[ACC] = lhs.nullish_coalesce(&rhs).raw();
                }
                Opcode::In => {
                    let (lhs, rhs) = self.value_pair(self.frame.regs[b], self.frame.regs[c]);
                    self.frame.regs[ACC] = lhs.in_(&rhs).raw();
                }
                Opcode::Instanceof => {
                    let (lhs, rhs) = self.value_pair(self.frame.regs[b], self.frame.regs[c]);
                    self.frame.regs[ACC] = lhs.instanceof(&rhs).raw();
                }
                Opcode::GetLengthIc => {
                    let result = self.get_length_value(self.frame.regs[b]);
                    if let Some(obj_ptr) = object_from_value(self.frame.regs[b]) {
                        let length_key = self.intern_string("length");
                        self.feedback.last_ic_slot = Some(c);
                        self.last_ic_object = Some(obj_ptr);
                        self.update_ic_slot(c, obj_ptr, PropertyKey::Value(length_key));
                    }
                    self.frame.regs[a] = result;
                }
                Opcode::ArrayPushAcc => {
                    self.frame.regs[ACC] =
                        self.array_push(self.frame.regs[a], self.frame.regs[ACC]);
                }
                Opcode::NewObj => {
                    self.frame.regs[a] = self.alloc_object();
                }
                Opcode::NewArr => {
                    self.frame.regs[a] = self.alloc_array(b);
                }
                Opcode::NewFunc => {
                    let descriptor = self
                        .const_pool
                        .get(Self::decode_abx(insn))
                        .copied()
                        .unwrap_or(make_undefined());
                    self.frame.regs[a] = self.alloc_function(descriptor);
                }
                Opcode::NewClass => {
                    self.frame.regs[a] = self.alloc_class(self.frame.regs[b]);
                }
                Opcode::GetProp | Opcode::GetSuper => {
                    self.frame.regs[a] = self.get_property(
                        self.frame.regs[b],
                        Self::property_key_from_immediate(c as u16),
                    );
                }
                Opcode::SetProp | Opcode::SetSuper => {
                    self.frame.regs[ACC] = self.set_property(
                        self.frame.regs[b],
                        Self::property_key_from_immediate(c as u16),
                        self.frame.regs[a],
                    );
                }
                Opcode::GetUpval | Opcode::LoadClosure => {
                    self.frame.regs[a] = self.upvalues.get(b).copied().unwrap_or(make_undefined());
                }
                Opcode::SetUpval => {
                    self.ensure_upvalue_slot(b);
                    self.upvalues[b] = self.frame.regs[a];
                }
                Opcode::GetScope => {
                    self.frame.regs[a] = self.scope_at_depth(b).unwrap_or(make_undefined());
                }
                Opcode::SetScope => {
                    self.set_scope_at_depth(b, self.frame.regs[a]);
                }
                Opcode::ResolveScope => {
                    let name = Self::decode_abx(insn) as u16;
                    self.frame.regs[a] = self
                        .resolve_scope_value(name)
                        .or_else(|| self.scope_chain.last().copied())
                        .unwrap_or(make_undefined());
                }
                Opcode::DeleteProp => {
                    let deleted = self.delete_property(
                        self.frame.regs[b],
                        Self::property_key_from_immediate(c as u16),
                    );
                    self.frame.regs[a] = make_bool(deleted);
                }
                Opcode::HasProp => {
                    let has = self.has_property(
                        self.frame.regs[b],
                        Self::property_key_from_immediate(c as u16),
                    );
                    self.frame.regs[a] = make_bool(has);
                }
                Opcode::Keys => {
                    let keys = self.get_keys(self.frame.regs[b]);
                    let array = self.alloc_array(keys.len());
                    for key in keys {
                        let key_value = self.property_key_to_value(key);
                        let _ = self.array_push(array, key_value);
                    }
                    self.frame.regs[a] = array;
                }
                Opcode::ForIn => {
                    let keys = self
                        .get_keys(self.frame.regs[b])
                        .into_iter()
                        .map(|key| self.property_key_to_value(key))
                        .collect();
                    let iterator = self.alloc_iterator(keys);
                    self.frame.regs[a] = iterator;
                    self.frame.regs[ACC] = self.iterator_next_value(iterator);
                }
                Opcode::IteratorNext => {
                    self.frame.regs[ACC] = self.iterator_next_value(self.frame.regs[a]);
                }
                Opcode::Spread => {
                    let source_values = self.array_values(self.frame.regs[b]).unwrap_or_default();
                    for value in source_values {
                        let _ = self.array_push(self.frame.regs[a], value);
                    }
                }
                Opcode::Destructure => {
                    let source_values = self.array_values(self.frame.regs[b]).unwrap_or_default();
                    for (index, value) in source_values.into_iter().enumerate() {
                        let dst = a + index;
                        if dst < self.frame.regs.len() {
                            self.frame.regs[dst] = value;
                        }
                    }
                }
                Opcode::CreateEnv => {
                    let env = self.alloc_env();
                    self.scope_chain.push(env);
                    self.frame.header.env = Some(env);
                    self.frame.regs[a] = env;
                }
                Opcode::LoadName => {
                    self.frame.regs[ACC] = self.load_name_value(Self::decode_abx(insn) as u16);
                }
                Opcode::StoreName => {
                    self.store_name_value(Self::decode_abx(insn) as u16, self.frame.regs[a]);
                }
                Opcode::NewThis => {
                    self.frame.regs[a] = self.alloc_object();
                }
                Opcode::TypeofName => {
                    let value = self.load_name_value(Self::decode_abx(insn) as u16);
                    self.frame.regs[a] = self.value_op(value).typeof_().raw();
                }
                Opcode::JmpEq => {
                    let (lhs, rhs) = self.value_pair(self.frame.regs[a], self.frame.regs[b]);
                    if bool_from_value(lhs.eq(&rhs).raw()).unwrap_or(false) {
                        self.jump_by(c as i8 as i16);
                    }
                }
                Opcode::JmpNeq => {
                    let (lhs, rhs) = self.value_pair(self.frame.regs[a], self.frame.regs[b]);
                    if bool_from_value(lhs.ne(&rhs).raw()).unwrap_or(false) {
                        self.jump_by(c as i8 as i16);
                    }
                }
                Opcode::JmpLt => {
                    let (lhs, rhs) = self.value_pair(self.frame.regs[a], self.frame.regs[b]);
                    if bool_from_value(lhs.lt(&rhs).raw()).unwrap_or(false) {
                        self.jump_by(c as i8 as i16);
                    }
                }
                Opcode::JmpLte => {
                    let (lhs, rhs) = self.value_pair(self.frame.regs[a], self.frame.regs[b]);
                    if bool_from_value(lhs.le(&rhs).raw()).unwrap_or(false) {
                        self.jump_by(c as i8 as i16);
                    }
                }
                Opcode::JmpLteFalse => {
                    let (lhs, rhs) = self.value_pair(self.frame.regs[a], self.frame.regs[b]);
                    if !bool_from_value(lhs.le(&rhs).raw()).unwrap_or(false) {
                        self.jump_by(c as i8 as i16);
                    }
                }
                Opcode::LoopIncJmp => {
                    let current =
                        self.binary_numeric_op(self.frame.regs[a], make_number(1.0), |x, y| x + y);
                    self.frame.regs[a] = current;
                    if self.less_than(current, self.frame.regs[ACC]) {
                        self.jump_by(Self::decode_asbx(insn));
                    }
                }
                Opcode::Switch => {
                    if let Some(offset) = self.switch_jump_offset(b, self.frame.regs[a]) {
                        self.jump_by(offset);
                    }
                }
                Opcode::LoopHint => {
                    let pc = self.pc.saturating_sub(1);
                    self.feedback.last_loop_hint_pc = Some(pc);
                    *self.feedback.loop_hint_counts.entry(pc).or_default() += 1;
                }
                Opcode::Ret => {
                    if !self.exit_frame(self.frame.regs[ACC]) {
                        return;
                    }
                    if stop_at_depth == Some(self.frame.depth()) {
                        return;
                    }
                    continue;
                }
                Opcode::RetU => {
                    if !self.exit_frame(make_undefined()) {
                        return;
                    }
                    if stop_at_depth == Some(self.frame.depth()) {
                        return;
                    }
                    continue;
                }
                Opcode::RetReg => {
                    if !self.exit_frame(self.frame.regs[a]) {
                        return;
                    }
                    if stop_at_depth == Some(self.frame.depth()) {
                        return;
                    }
                    continue;
                }
                Opcode::TailCall | Opcode::CallIc => match self.invoke_call(a, b) {
                    CallAction::Returned(result) => self.frame.regs[ACC] = result,
                    CallAction::EnteredFrame => continue,
                },
                Opcode::Construct => match self.invoke_construct(a, b) {
                    CallAction::Returned(result) => self.frame.regs[ACC] = result,
                    CallAction::EnteredFrame => continue,
                },
                Opcode::CallVar | Opcode::CallIcVar => match self.invoke_spread_call(a, a + 1) {
                    CallAction::Returned(result) => self.frame.regs[ACC] = result,
                    CallAction::EnteredFrame => continue,
                },
                Opcode::Enter => {
                    let frame_size = Self::decode_abx(insn).min(256);
                    self.frame.header.frame_size = frame_size as u32;
                    self.frame.header.register_count = frame_size as u32;
                    self.frame.scope_stack.push(self.scope_chain.len());
                }
                Opcode::Leave => {
                    let scope_depth = self
                        .frame
                        .scope_stack
                        .pop()
                        .unwrap_or(self.frame.header.scope_depth);
                    self.restore_scope_depth(scope_depth);
                }
                Opcode::Yield | Opcode::Await => {
                    self.frame.regs[ACC] = self.frame.regs[a];
                }
                Opcode::Throw => {
                    let exception = self.frame.regs[a];
                    if let Some(catch_pc) = self.frame.try_stack.pop() {
                        self.last_exception = exception;
                        self.pc = catch_pc.min(self.bytecode.len());
                    } else {
                        if !self.exit_frame(exception) {
                            return;
                        }
                        if stop_at_depth == Some(self.frame.depth()) {
                            return;
                        }
                        continue;
                    }
                }
                Opcode::Try => {
                    let catch_pc = (self.pc as isize + Self::decode_asbx(insn) as isize)
                        .clamp(0, self.bytecode.len() as isize)
                        as usize;
                    self.frame.try_stack.push(catch_pc);
                }
                Opcode::EndTry => {
                    let _ = self.frame.try_stack.pop();
                }
                Opcode::Catch => {
                    self.frame.regs[a] = self.last_exception;
                    self.frame.regs[ACC] = self.last_exception;
                }
                Opcode::Finally => {
                    self.last_exception = make_undefined();
                }
                Opcode::ProfileType => {
                    let slot = if b != 0 || c != 0 { c } else { a };
                    let reg = if b != 0 || c != 0 { b } else { ACC };
                    self.observe_type_feedback_slot(slot, self.frame.regs[reg]);
                }
                Opcode::ProfileCall => {
                    let slot = if b != 0 || c != 0 { c } else { a };
                    let kind = if b != 0 || c != 0 {
                        self.classify_value(self.frame.regs[b])
                    } else {
                        self.feedback
                            .last_call_kind
                            .unwrap_or_else(|| self.classify_value(self.frame.regs[ACC]))
                    };
                    self.observe_call_feedback_kind(slot, kind);
                }
                Opcode::ProfileRet => {
                    self.observe_return_value(self.frame.regs[ACC]);
                }
                Opcode::CheckType => {
                    let expected_id = if b != 0 || c != 0 { c as u8 } else { a as u8 };
                    let reg = if b != 0 || c != 0 { b } else { ACC };
                    if let Some(expected) = ValueProfileKind::from_id(expected_id) {
                        let observed = self.classify_value(self.frame.regs[reg]);
                        if observed != expected {
                            self.record_deopt(DeoptReason::TypeMismatch { expected, observed });
                        }
                    }
                }
                Opcode::CheckStruct => {
                    let expected = if b != 0 || c != 0 { c as u32 } else { a as u32 };
                    let reg = if b != 0 || c != 0 { b } else { ACC };
                    let observed = object_from_value(self.frame.regs[reg])
                        .map(|obj_ptr| self.current_shape_id(obj_ptr))
                        .unwrap_or(0);
                    if observed != expected {
                        self.record_deopt(DeoptReason::StructMismatch { expected, observed });
                    }
                }
                Opcode::CheckIc => {
                    let slot = if c != 0 {
                        c
                    } else if a != 0 {
                        a
                    } else {
                        self.feedback.last_ic_slot.unwrap_or(0)
                    };
                    let reg = if b != 0 || c != 0 { b } else { ACC };
                    self.feedback.last_ic_slot = Some(slot);
                    let obj_ptr = if b != 0 || c != 0 {
                        object_from_value(self.frame.regs[reg])
                    } else {
                        object_from_value(self.frame.regs[reg]).or(self.last_ic_object)
                    };
                    let hit = obj_ptr.is_some_and(|obj_ptr| self.check_ic_slot(slot, obj_ptr));
                    self.last_ic_object = obj_ptr;
                    self.frame.regs[ACC] = make_bool(hit);
                }
                Opcode::IcInit => {
                    let slot = if c != 0 {
                        c
                    } else if a != 0 {
                        a
                    } else {
                        self.feedback.last_ic_slot.unwrap_or(0)
                    };
                    let reg = if b != 0 || c != 0 { b } else { ACC };
                    self.feedback.last_ic_slot = Some(slot);
                    let obj_ptr = if b != 0 || c != 0 {
                        object_from_value(self.frame.regs[reg])
                    } else {
                        object_from_value(self.frame.regs[reg]).or(self.last_ic_object)
                    };
                    if let Some(obj_ptr) = obj_ptr {
                        self.last_ic_object = Some(obj_ptr);
                        self.init_ic_slot(slot, obj_ptr, None);
                    }
                }
                Opcode::IcUpdate => {
                    let slot = if c != 0 {
                        c
                    } else if a != 0 {
                        a
                    } else {
                        self.feedback.last_ic_slot.unwrap_or(0)
                    };
                    let reg = if b != 0 || c != 0 { b } else { ACC };
                    self.feedback.last_ic_slot = Some(slot);
                    let obj_ptr = if b != 0 || c != 0 {
                        object_from_value(self.frame.regs[reg])
                    } else {
                        object_from_value(self.frame.regs[reg]).or(self.last_ic_object)
                    };
                    if let Some(obj_ptr) = obj_ptr {
                        self.last_ic_object = Some(obj_ptr);
                        let shape_id = self.current_shape_id(obj_ptr);
                        let ic = self.ensure_ic_slot(slot);
                        match ic.state {
                            ICState::Uninit => {
                                ic.state = ICState::Mono;
                                ic.shape_id = shape_id;
                                ic.offset = 0;
                                ic.shapes.clear();
                            }
                            ICState::Mono => {
                                if ic.shape_id != shape_id {
                                    ic.state = ICState::Poly;
                                    ic.shapes.clear();
                                    ic.shapes.push(shape_id);
                                }
                            }
                            ICState::Poly => {
                                if ic.shape_id != shape_id && !ic.shapes.contains(&shape_id) {
                                    if ic.shapes.len() < 3 {
                                        ic.shapes.push(shape_id);
                                    } else {
                                        ic.state = ICState::Mega;
                                        ic.shapes.clear();
                                    }
                                }
                            }
                            ICState::Mega => {}
                        }
                    }
                }
                Opcode::IcMiss => {
                    let slot = if a != 0 {
                        a
                    } else {
                        self.feedback.last_ic_slot.unwrap_or(0)
                    };
                    self.feedback.ic_misses = self.feedback.ic_misses.saturating_add(1);
                    self.feedback.last_ic_slot = Some(slot);
                    self.frame.regs[ACC] = make_false();
                }
                Opcode::OsrEntry => {
                    self.feedback.osr_entries = self.feedback.osr_entries.saturating_add(1);
                    self.feedback.osr_active = true;
                }
                Opcode::ProfileHotLoop => {
                    let pc = self
                        .feedback
                        .last_loop_hint_pc
                        .unwrap_or_else(|| self.pc.saturating_sub(1));
                    *self.feedback.hot_loop_counts.entry(pc).or_default() += 1;
                }
                Opcode::OsrExit => {
                    self.feedback.osr_exits = self.feedback.osr_exits.saturating_add(1);
                    self.feedback.osr_active = false;
                }
                Opcode::JitHint => {
                    let key = if a != 0 { a } else { self.pc.saturating_sub(1) };
                    *self.feedback.jit_hints.entry(key).or_default() += 1;
                }
                Opcode::SafetyCheck => {
                    let reg = if a != 0 { a } else { ACC };
                    self.feedback.safety_checks = self.feedback.safety_checks.saturating_add(1);
                    let failed = reg >= self.frame.regs.len()
                        || self.frame.header.register_count as usize > self.frame.regs.len()
                        || self.pc > self.bytecode.len()
                        || self.frame.regs[reg].is_empty();
                    if failed {
                        self.feedback.failed_safety_checks =
                            self.feedback.failed_safety_checks.saturating_add(1);
                        self.record_deopt(DeoptReason::SafetyCheck { register: reg });
                    }
                }
                Opcode::GetPropIcCall => {
                    let key = Self::property_key_from_immediate(c as u16);
                    let this_value = self.frame.regs[b];
                    let callee = self.get_property(this_value, key);
                    self.frame.regs[a] = callee;
                    match self.invoke_method_call(callee, this_value, 0, a + 1) {
                        CallAction::Returned(result) => self.frame.regs[ACC] = result,
                        CallAction::EnteredFrame => continue,
                    }
                }
                Opcode::IncJmpFalseLoop => {
                    self.frame.regs[ACC] =
                        self.binary_numeric_op(self.frame.regs[ACC], make_number(1.0), |x, y| {
                            x + y
                        });
                    if !self.is_truthy_value(self.frame.regs[a]) {
                        self.jump_by(Self::decode_asbx(insn));
                    }
                }
                Opcode::LoadKAddAcc => {
                    let constant = self
                        .const_pool
                        .get(Self::decode_abx(insn))
                        .copied()
                        .unwrap_or(make_undefined());
                    self.frame.regs[ACC] = self.binary_add(constant, self.frame.regs[ACC]);
                }
                Opcode::AddMov => {
                    let result = self.binary_add(self.frame.regs[b], self.frame.regs[c]);
                    self.frame.regs[ACC] = result;
                    self.frame.regs[a] = result;
                }
                Opcode::EqJmpTrue => {
                    if self.abstract_equal(self.frame.regs[b], self.frame.regs[c]) {
                        self.jump_by(a as i8 as i16);
                    }
                }
                Opcode::GetPropAccCall => {
                    let this_value = self.frame.regs[b];
                    let key = self.property_key_from_value(self.frame.regs[c]);
                    let callee = self.get_property(this_value, key);
                    match self.dispatch_call_value(callee, this_value, &[]) {
                        CallAction::Returned(result) => self.frame.regs[ACC] = result,
                        CallAction::EnteredFrame => continue,
                    }
                }
                Opcode::LoadKMulAcc => {
                    let constant = self
                        .const_pool
                        .get(Self::decode_abx(insn))
                        .copied()
                        .unwrap_or(make_undefined());
                    self.frame.regs[ACC] =
                        self.binary_numeric_op(constant, self.frame.regs[ACC], |x, y| x * y);
                }
                Opcode::LtJmp => {
                    if self.less_than(self.frame.regs[b], self.frame.regs[c]) {
                        self.jump_by(a as i8 as i16);
                    }
                }
                Opcode::GetPropIcMov => {
                    let key = Self::property_key_from_immediate(c as u16);
                    self.frame.regs[a] = self.get_property(self.frame.regs[b], key);
                }
                Opcode::GetPropAddImmSetPropIc => {
                    let key = Self::property_key_from_immediate(c as u16);
                    let current = self.get_property(self.frame.regs[b], key);
                    let next =
                        self.binary_numeric_op(current, make_int32(a as i8 as i32), |x, y| x + y);
                    self.frame.regs[ACC] = self.set_property(self.frame.regs[b], key, next);
                }
                Opcode::AddAccImm8Mov => {
                    self.frame.regs[ACC] = self.binary_numeric_op(
                        self.frame.regs[ACC],
                        make_int32(b as i8 as i32),
                        |x, y| x + y,
                    );
                    self.frame.regs[a] = self.frame.regs[ACC];
                }
                Opcode::CallIcSuper => match self.invoke_call(a, b) {
                    CallAction::Returned(result) => self.frame.regs[ACC] = result,
                    CallAction::EnteredFrame => continue,
                },
                Opcode::LoadThisCall => {
                    match self.dispatch_call_value(self.frame.regs[0], self.frame.regs[0], &[]) {
                        CallAction::Returned(result) => self.frame.regs[ACC] = result,
                        CallAction::EnteredFrame => continue,
                    }
                }
                Opcode::EqJmpFalse => {
                    if !self.abstract_equal(self.frame.regs[b], self.frame.regs[c]) {
                        self.jump_by(a as i8 as i16);
                    }
                }
                Opcode::LoadKSubAcc => {
                    let constant = self
                        .const_pool
                        .get(Self::decode_abx(insn))
                        .copied()
                        .unwrap_or(make_undefined());
                    self.frame.regs[ACC] =
                        self.binary_numeric_op(constant, self.frame.regs[ACC], |x, y| x - y);
                }
                Opcode::GetLengthIcCall => {
                    self.frame.regs[ACC] = self.get_length_value(self.frame.regs[b]);
                }
                Opcode::AddStrAccMov => {
                    let result = format!(
                        "{}{}",
                        self.display_string(self.frame.regs[ACC]),
                        self.display_string(self.frame.regs[b])
                    );
                    self.frame.regs[ACC] = self.intern_string(result);
                    self.frame.regs[a] = self.frame.regs[ACC];
                }
                Opcode::IncAccJmp => {
                    self.frame.regs[ACC] =
                        self.binary_numeric_op(self.frame.regs[ACC], make_number(1.0), |x, y| {
                            x + y
                        });
                    self.jump_by(Self::decode_asbx(insn));
                }
                Opcode::GetPropChainAcc => {
                    let inner_reg = self.array_index_from_value(self.frame.regs[b]).unwrap_or(0);
                    let base = self
                        .frame
                        .regs
                        .get(inner_reg)
                        .copied()
                        .unwrap_or(make_undefined());
                    self.frame.regs[ACC] =
                        self.get_property(base, Self::property_key_from_immediate(c as u16));
                }
                Opcode::TestJmpTrue => {
                    if self.is_truthy_value(self.frame.regs[a]) {
                        self.jump_by(Self::decode_asbx(insn));
                    }
                }
                Opcode::LoadArgCall => {
                    self.frame.regs[a] = self.frame.arg(b);
                    match self.dispatch_call_value(self.frame.regs[a], self.frame.regs[0], &[]) {
                        CallAction::Returned(result) => self.frame.regs[ACC] = result,
                        CallAction::EnteredFrame => continue,
                    }
                }
                Opcode::MulAccMov => {
                    self.frame.regs[ACC] =
                        self.binary_numeric_op(self.frame.regs[ACC], self.frame.regs[b], |x, y| {
                            x * y
                        });
                    self.frame.regs[a] = self.frame.regs[ACC];
                }
                Opcode::LteJmpLoop => {
                    if self.less_than_or_equal(self.frame.regs[b], self.frame.regs[c]) {
                        self.jump_by(a as i8 as i16);
                    }
                }
                Opcode::NewObjInitProp => {
                    let object = self.alloc_object();
                    let _ = self.set_property(
                        object,
                        Self::property_key_from_immediate(c as u16),
                        self.frame.regs[b],
                    );
                    self.frame.regs[a] = object;
                }
                Opcode::ProfileHotCall => match self.invoke_call(b, c) {
                    CallAction::Returned(result) => self.frame.regs[ACC] = result,
                    CallAction::EnteredFrame => continue,
                },
                Opcode::AssertValue => {
                    // assert value - check if value is truthy
                    if !self.is_truthy_value(self.frame.regs[a]) {
                        panic!("Assertion failed: value is not truthy");
                    }
                }
                Opcode::AssertOk => {
                    // assert_ok value - check if value is not an error/exception
                    if is_undefined(self.frame.regs[a]) || is_null(self.frame.regs[a]) {
                        panic!("Assertion failed: value is not ok");
                    }
                }
                Opcode::AssertEqual => {
                    // assert_equal a, b - check if values are equal (abstract equality)
                    if !self.abstract_equal(self.frame.regs[b], self.frame.regs[c]) {
                        panic!("Assertion failed: values are not equal");
                    }
                }
                Opcode::AssertNotEqual => {
                    // assert_notEqual a, b - check if values are not equal
                    if self.abstract_equal(self.frame.regs[b], self.frame.regs[c]) {
                        panic!("Assertion failed: values are equal");
                    }
                }
                Opcode::AssertDeepEqual => {
                    // assert_deepEqual a, b - check deep equality (simplified to abstract equality for now)
                    if !self.abstract_equal(self.frame.regs[b], self.frame.regs[c]) {
                        panic!("Assertion failed: values are not deep equal");
                    }
                }
                Opcode::AssertNotDeepEqual => {
                    // assert_notDeepEqual a, b
                    if self.abstract_equal(self.frame.regs[b], self.frame.regs[c]) {
                        panic!("Assertion failed: values are deep equal");
                    }
                }
                Opcode::AssertStrictEqual => {
                    // assert_strictEqual a, b
                    if !self.strict_equal(self.frame.regs[b], self.frame.regs[c]) {
                        panic!("Assertion failed: values are not strictly equal");
                    }
                }
                Opcode::AssertNotStrictEqual => {
                    // assert_notStrictEqual a, b
                    if self.strict_equal(self.frame.regs[b], self.frame.regs[c]) {
                        panic!("Assertion failed: values are strictly equal");
                    }
                }
                Opcode::AssertDeepStrictEqual => {
                    // assert_deepStrictEqual a, b (same as strict equal for now)
                    if !self.strict_equal(self.frame.regs[b], self.frame.regs[c]) {
                        panic!("Assertion failed: values are not deep strictly equal");
                    }
                }
                Opcode::AssertNotDeepStrictEqual => {
                    // assert_notDeepStrictEqual a, b
                    if self.strict_equal(self.frame.regs[b], self.frame.regs[c]) {
                        panic!("Assertion failed: values are deep strictly equal");
                    }
                }
                Opcode::AssertThrows => {
                    // assert_throws fn - check if function throws
                    // For now, we'll just check if it's a function
                    if !is_object(self.frame.regs[a]) {
                        panic!("Assertion failed: not a function");
                    }
                }
                Opcode::AssertDoesNotThrow => {
                    // assert_doesNotThrow fn - check if function doesn't throw
                    // For now, we'll just check if it's a function
                    if !is_object(self.frame.regs[a]) {
                        panic!("Assertion failed: not a function");
                    }
                }
                Opcode::AssertRejects => {
                    // assert_rejects promise - check if promise rejects
                    // Not implemented yet
                }
                Opcode::AssertDoesNotReject => {
                    // assert_doesNotReject promise - check if promise doesn't reject
                    // Not implemented yet
                }
                Opcode::AssertFail => {
                    // assert_fail - always fail
                    panic!("Assertion failed: explicit fail");
                }
                Opcode::AddI32 => {
                    // Fast path: int32 + int32
                    let lhs = self.frame.regs[b];
                    let rhs = self.frame.regs[c];
                    
                    // Check if both are ints
                    if lhs.is_int() && rhs.is_int() {
                        let a_int = lhs.int_payload_unchecked();
                        let b_int = rhs.int_payload_unchecked();
                        if let Some(result) = a_int.checked_add(b_int) {
                            self.frame.regs[ACC] = make_int32(result);
                            if a != ACC {
                                self.frame.regs[a] = make_int32(result);
                            }
                            continue;
                        }
                    }
                    // Fall back to slow path
                    let (lhs, rhs) = self.value_pair(lhs, rhs);
                    self.frame.regs[ACC] = lhs.add(&rhs).raw();
                    if a != ACC {
                        self.frame.regs[a] = self.frame.regs[ACC];
                    }
                }
                Opcode::AddF64 => {
                    // Fast path: f64 + f64
                    let lhs = self.frame.regs[b];
                    let rhs = self.frame.regs[c];
                    
                    // Check if both are f64
                    if lhs.is_f64() && rhs.is_f64() {
                        let a_f64 = lhs.f64_payload_unchecked();
                        let b_f64 = rhs.f64_payload_unchecked();
                        self.frame.regs[ACC] = make_number(a_f64 + b_f64);
                        if a != ACC {
                            self.frame.regs[a] = self.frame.regs[ACC];
                        }
                        continue;
                    }
                    // Fall back to slow path
                    let (lhs, rhs) = self.value_pair(lhs, rhs);
                    self.frame.regs[ACC] = lhs.add(&rhs).raw();
                    if a != ACC {
                        self.frame.regs[a] = self.frame.regs[ACC];
                    }
                }
                Opcode::SubI32 => {
                    // Fast path: int32 - int32
                    let lhs = self.frame.regs[b];
                    let rhs = self.frame.regs[c];
                    
                    if lhs.is_int() && rhs.is_int() {
                        let a_int = lhs.int_payload_unchecked();
                        let b_int = rhs.int_payload_unchecked();
                        if let Some(result) = a_int.checked_sub(b_int) {
                            self.frame.regs[ACC] = make_int32(result);
                            if a != ACC {
                                self.frame.regs[a] = make_int32(result);
                            }
                            continue;
                        }
                    }
                    // Fall back to slow path
                    let (lhs, rhs) = self.value_pair(lhs, rhs);
                    self.frame.regs[ACC] = lhs.sub(&rhs).raw();
                    if a != ACC {
                        self.frame.regs[a] = self.frame.regs[ACC];
                    }
                }
                Opcode::SubF64 => {
                    // Fast path: f64 - f64
                    let lhs = self.frame.regs[b];
                    let rhs = self.frame.regs[c];
                    
                    if lhs.is_f64() && rhs.is_f64() {
                        let a_f64 = lhs.f64_payload_unchecked();
                        let b_f64 = rhs.f64_payload_unchecked();
                        self.frame.regs[ACC] = make_number(a_f64 - b_f64);
                        if a != ACC {
                            self.frame.regs[a] = self.frame.regs[ACC];
                        }
                        continue;
                    }
                    // Fall back to slow path
                    let (lhs, rhs) = self.value_pair(lhs, rhs);
                    self.frame.regs[ACC] = lhs.sub(&rhs).raw();
                    if a != ACC {
                        self.frame.regs[a] = self.frame.regs[ACC];
                    }
                }
                Opcode::MulI32 => {
                    // Fast path: int32 * int32
                    let lhs = self.frame.regs[b];
                    let rhs = self.frame.regs[c];
                    
                    if lhs.is_int() && rhs.is_int() {
                        let a_int = lhs.int_payload_unchecked();
                        let b_int = rhs.int_payload_unchecked();
                        if let Some(result) = a_int.checked_mul(b_int) {
                            self.frame.regs[ACC] = make_int32(result);
                            if a != ACC {
                                self.frame.regs[a] = make_int32(result);
                            }
                            continue;
                        }
                    }
                    // Fall back to slow path
                    let (lhs, rhs) = self.value_pair(lhs, rhs);
                    self.frame.regs[ACC] = lhs.mul(&rhs).raw();
                    if a != ACC {
                        self.frame.regs[a] = self.frame.regs[ACC];
                    }
                }
                Opcode::MulF64 => {
                    // Fast path: f64 * f64
                    let lhs = self.frame.regs[b];
                    let rhs = self.frame.regs[c];
                    
                    if lhs.is_f64() && rhs.is_f64() {
                        let a_f64 = lhs.f64_payload_unchecked();
                        let b_f64 = rhs.f64_payload_unchecked();
                        self.frame.regs[ACC] = make_number(a_f64 * b_f64);
                        if a != ACC {
                            self.frame.regs[a] = self.frame.regs[ACC];
                        }
                        continue;
                    }
                    // Fall back to slow path
                    let (lhs, rhs) = self.value_pair(lhs, rhs);
                    self.frame.regs[ACC] = lhs.mul(&rhs).raw();
                    if a != ACC {
                        self.frame.regs[a] = self.frame.regs[ACC];
                    }
                }
                Opcode::Reserved(_) => {}
            }
        }
    }

    fn run_until_frame_depth(&mut self, depth: usize) {
        self.run_inner(Some(depth));
    }
    pub fn optimize(&mut self) {
        let bytecode = std::mem::take(&mut self.bytecode);
        let const_pool = std::mem::take(&mut self.const_pool);
        let (bytecode, const_pool) = optimization::optimize_bytecode(bytecode, const_pool);
        self.bytecode = bytecode;
        self.const_pool = const_pool;
    }

    pub fn run(&mut self, optimization: bool) {
        if optimization && self.pc == 0 && self.frame.depth() == 0 {
            let bytecode = std::mem::take(&mut self.bytecode);
            let const_pool = std::mem::take(&mut self.const_pool);
            let (bytecode, const_pool) = optimization::optimize_bytecode(bytecode, const_pool);
            self.bytecode = bytecode;
            self.const_pool = const_pool;
        }
        self.run_inner(None);
    }
}

impl Drop for VM {
    fn drop(&mut self) {
        for obj_ptr in self.objects.drain(..) {
            unsafe {
                drop(Box::from_raw(obj_ptr));
            }
        }

        for shape_ptr in self.shapes.drain(..) {
            unsafe {
                drop(Box::from_raw(shape_ptr));
            }
        }

        for string_ptr in self.strings.drain(..) {
            unsafe {
                drop(Box::from_raw(string_ptr));
            }
        }
    }
}
