use std::collections::HashMap;
use std::rc::Rc;

use crate::atoms::Atom;
use crate::gc::{Gc, *};
use crate::heap::*;
use crate::runtime::Context;
use crate::runtime_trait::*;

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct Value(pub(crate) u64);

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Null;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Undefined;

pub const NULL: Null = Null;
pub const UNDEFINED: Undefined = Undefined;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ValueTypeError {
    expected: &'static str,
    actual: &'static str,
}

impl ValueTypeError {
    pub(crate) fn new(expected: &'static str, actual: &'static str) -> Self {
        Self { expected, actual }
    }

    pub fn expected(&self) -> &'static str {
        self.expected
    }

    pub fn actual(&self) -> &'static str {
        self.actual
    }
}

impl std::fmt::Display for ValueTypeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "expected {}, got {}", self.expected, self.actual)
    }
}

impl std::error::Error for ValueTypeError {}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct JsonValueError {
    message: String,
}

impl JsonValueError {
    #[allow(dead_code)]
    fn unsupported(type_name: &'static str) -> Self {
        Self {
            message: format!("{type_name} cannot be converted to JSON"),
        }
    }

    #[allow(dead_code)]
    fn cyclic(type_name: &'static str) -> Self {
        Self {
            message: format!("cyclic {type_name} cannot be converted to JSON"),
        }
    }

    #[allow(dead_code)]
    fn invalid_number(value: f64) -> Self {
        Self {
            message: format!("non-finite number {value} cannot be converted to JSON"),
        }
    }

    fn parse(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }

    pub fn message(&self) -> &str {
        &self.message
    }
}

impl std::fmt::Display for JsonValueError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for JsonValueError {}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct YamlValueError {
    message: String,
}

impl YamlValueError {
    #[allow(dead_code)]
    fn unsupported(type_name: &'static str) -> Self {
        Self {
            message: format!("{type_name} cannot be converted to YAML"),
        }
    }

    #[allow(dead_code)]
    fn cyclic(type_name: &'static str) -> Self {
        Self {
            message: format!("cyclic {type_name} cannot be converted to YAML"),
        }
    }

    #[allow(dead_code)]
    fn invalid_number(value: f64) -> Self {
        Self {
            message: format!("non-finite number {value} cannot be converted to YAML"),
        }
    }

    fn parse(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }

    pub fn message(&self) -> &str {
        &self.message
    }
}

impl std::fmt::Display for YamlValueError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for YamlValueError {}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MsgpackValueError {
    message: String,
}

impl MsgpackValueError {
    #[allow(dead_code)]
    fn unsupported(type_name: &'static str) -> Self {
        Self {
            message: format!("{type_name} cannot be converted to MsgPack"),
        }
    }

    #[allow(dead_code)]
    fn cyclic(type_name: &'static str) -> Self {
        Self {
            message: format!("cyclic {type_name} cannot be converted to MsgPack"),
        }
    }

    #[allow(dead_code)]
    fn invalid_number(value: f64) -> Self {
        Self {
            message: format!("non-finite number {value} cannot be converted to MsgPack"),
        }
    }

    fn parse(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }

    pub fn message(&self) -> &str {
        &self.message
    }
}

impl std::fmt::Display for MsgpackValueError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for MsgpackValueError {}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SerdeValueError {
    message: String,
}

impl SerdeValueError {
    fn unsupported(type_name: &'static str) -> Self {
        Self {
            message: format!("{type_name} cannot be converted through serde"),
        }
    }

    #[allow(dead_code)]
    fn cyclic(type_name: &'static str) -> Self {
        Self {
            message: format!("cyclic {type_name} cannot be converted through serde"),
        }
    }

    #[allow(dead_code)]
    fn invalid_number(value: f64) -> Self {
        Self {
            message: format!("non-finite number {value} cannot be converted through serde"),
        }
    }

    #[allow(dead_code)]
    fn serde_json(error: impl ToString) -> Self {
        Self {
            message: error.to_string(),
        }
    }

    pub fn message(&self) -> &str {
        &self.message
    }
}

impl std::fmt::Display for SerdeValueError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for SerdeValueError {}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ValueError {
    message: String,
}

impl ValueError {
    fn unsupported(feature: &'static str) -> Self {
        Self {
            message: format!("{feature} support is not available in this register-vm build"),
        }
    }

    pub fn message(&self) -> &str {
        &self.message
    }
}

impl std::fmt::Display for ValueError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for ValueError {}

const QNAN: u64 = 0x7ffc_0000_0000_0000;
const TAG_MASK: u64 = 0xf;
const PAYLOAD_MASK: u64 = 0x0000_ffff_ffff_fff0;

const TAG_INT: u64 = 1;
const TAG_BOOL: u64 = 2;
const TAG_NULL: u64 = 3;
const TAG_UNDEF: u64 = 4;
const TAG_HEAP: u64 = 5;
const TAG_ATOM: u64 = 6;
const TAG_EMPTY: u64 = 7;

// Small integer string cache - memoizes -100..100
// This reduces allocations for the most common number conversions
static SMALL_INT_STRING_CACHE: [&str; 201] = [
    "-100", "-99", "-98", "-97", "-96", "-95", "-94", "-93", "-92", "-91", "-90", "-89", "-88",
    "-87", "-86", "-85", "-84", "-83", "-82", "-81", "-80", "-79", "-78", "-77", "-76", "-75",
    "-74", "-73", "-72", "-71", "-70", "-69", "-68", "-67", "-66", "-65", "-64", "-63", "-62",
    "-61", "-60", "-59", "-58", "-57", "-56", "-55", "-54", "-53", "-52", "-51", "-50", "-49",
    "-48", "-47", "-46", "-45", "-44", "-43", "-42", "-41", "-40", "-39", "-38", "-37", "-36",
    "-35", "-34", "-33", "-32", "-31", "-30", "-29", "-28", "-27", "-26", "-25", "-24", "-23",
    "-22", "-21", "-20", "-19", "-18", "-17", "-16", "-15", "-14", "-13", "-12", "-11", "-10",
    "-9", "-8", "-7", "-6", "-5", "-4", "-3", "-2", "-1", "0", "1", "2", "3", "4", "5", "6", "7",
    "8", "9", "10", "11", "12", "13", "14", "15", "16", "17", "18", "19", "20", "21", "22", "23",
    "24", "25", "26", "27", "28", "29", "30", "31", "32", "33", "34", "35", "36", "37", "38", "39",
    "40", "41", "42", "43", "44", "45", "46", "47", "48", "49", "50", "51", "52", "53", "54", "55",
    "56", "57", "58", "59", "60", "61", "62", "63", "64", "65", "66", "67", "68", "69", "70", "71",
    "72", "73", "74", "75", "76", "77", "78", "79", "80", "81", "82", "83", "84", "85", "86", "87",
    "88", "89", "90", "91", "92", "93", "94", "95", "96", "97", "98", "99", "100",
];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HeapKind {
    Object,
    Array,
    BoolArray,
    Uint8Array,
    Int32Array,
    Float64Array,
    StringArray,
    String,
    Symbol,
    Function,
    Closure,
    NativeFunction,
    NativeClosure,
    Class,
    Module,
    Instance,
}

impl HeapKind {
    pub fn type_name(self) -> &'static str {
        match self {
            HeapKind::Object => "object",
            HeapKind::Array => "array",
            HeapKind::BoolArray => "bool array",
            HeapKind::Uint8Array => "uint8 array",
            HeapKind::Int32Array => "int32 array",
            HeapKind::Float64Array => "float64 array",
            HeapKind::StringArray => "string array",
            HeapKind::String => "string object",
            HeapKind::Symbol => "symbol object",
            HeapKind::Function => "function",
            HeapKind::Closure => "closure",
            HeapKind::NativeFunction => "native function",
            HeapKind::NativeClosure => "native closure",
            HeapKind::Class => "class",
            HeapKind::Module => "module",
            HeapKind::Instance => "instance",
        }
    }
}

impl Value {
    #[inline(always)]
    pub fn bits(self) -> u64 {
        self.0
    }

    #[inline(always)]
    fn is_tagged_int_bits(bits: u64) -> bool {
        (bits & QNAN) == QNAN && (bits & TAG_MASK) == TAG_INT
    }

    #[inline(always)]
    fn tagged(tag: u64, payload: u64) -> Self {
        Self(QNAN | (payload & PAYLOAD_MASK) | tag)
    }

    #[inline(always)]
    fn is_tagged(self) -> bool {
        // Optimized: bitwise AND is faster than shift on modern CPUs
        // Eliminates instruction (no shift), better pipelining
        (self.0 & QNAN) == QNAN
    }

    #[inline(always)]
    fn tag(self) -> u64 {
        self.0 & TAG_MASK
    }

    // Internal helpers: avoid Option overhead in hot paths
    #[inline(always)]
    fn int_payload(bits: u64) -> i32 {
        (bits >> 4) as i32
    }

    // Small integer string cache lookup
    #[inline]
    fn get_small_int_string(i: i32) -> Option<&'static str> {
        if (-100..=100).contains(&i) {
            let idx = (i + 100) as usize;
            Some(SMALL_INT_STRING_CACHE[idx])
        } else {
            None
        }
    }

    #[inline(always)]
    pub(crate) fn heap(ptr: *const GCHeader) -> Self {
        // Ensure pointer is properly aligned - pointer tagging requires lower bits to be zero
        debug_assert_eq!(
            (ptr as usize) & 0xf,
            0,
            "Heap pointer must be 16-byte aligned for safe tagging"
        );
        debug_assert_eq!((ptr as usize) & TAG_MASK as usize, 0);
        Self::tagged(TAG_HEAP, (ptr as usize as u64) & PAYLOAD_MASK)
    }

    #[inline(always)]
    pub(crate) fn as_heap_ptr(self) -> Option<*const GCHeader> {
        if !(self.is_tagged() && self.tag() == TAG_HEAP) {
            return None;
        }
        let ptr = (self.0 & PAYLOAD_MASK) as *const GCHeader;
        #[cfg(debug_assertions)]
        {
            let is_valid =
                Context::with_current_opt(|ctx| ctx.is_none_or(|ctx| ctx.is_valid_heap_ptr(ptr)));
            debug_assert!(is_valid, "invalid heap pointer tag: {ptr:p}");
            if !is_valid {
                return None;
            }
        }
        Some(ptr)
    }

    #[inline(always)]
    pub fn i32(v: i32) -> Self {
        Self::tagged(TAG_INT, ((v as u32) as u64) << 4)
    }

    #[inline(always)]
    pub fn bool(v: bool) -> Self {
        Self::tagged(TAG_BOOL, (u64::from(v)) << 4)
    }

    #[inline(always)]
    pub fn f64(v: f64) -> Self {
        let bits = v.to_bits();

        if bits & QNAN == QNAN {
            Self(f64::NAN.to_bits())
        } else {
            Self(bits)
        }
    }

    #[inline(always)]
    pub fn null() -> Self {
        Self::tagged(TAG_NULL, 0)
    }

    #[inline(always)]
    pub fn undefined() -> Self {
        Self::tagged(TAG_UNDEF, 0)
    }

    #[inline(always)]
    pub fn empty() -> Self {
        Self::tagged(TAG_EMPTY, 0)
    }

    #[inline(always)]
    pub fn atom(atom: Atom) -> Self {
        Self::tagged(TAG_ATOM, (atom.0 as u64) << 4)
    }

    #[inline(always)]
    pub fn as_i32(self) -> Option<i32> {
        (self.is_tagged() && self.tag() == TAG_INT).then_some(((self.0 >> 4) as u32) as i32)
    }

    #[inline(always)]
    pub fn as_bool(self) -> Option<bool> {
        (self.is_tagged() && self.tag() == TAG_BOOL).then_some(((self.0 >> 4) & 1) != 0)
    }

    #[inline(always)]
    pub fn as_f64(self) -> Option<f64> {
        (!self.is_tagged()).then(|| f64::from_bits(self.0))
    }

    #[inline(always)]
    pub fn is_int(self) -> bool {
        self.is_tagged() && self.tag() == TAG_INT
    }

    #[inline(always)]
    pub fn int_payload_unchecked(self) -> i32 {
        Self::int_payload(self.0)
    }

    #[inline(always)]
    pub fn is_f64(self) -> bool {
        !self.is_tagged()
    }

    #[inline(always)]
    pub fn f64_payload_unchecked(self) -> f64 {
        f64::from_bits(self.0)
    }

    #[inline(always)]
    pub fn as_atom(self) -> Option<Atom> {
        (self.is_tagged() && self.tag() == TAG_ATOM).then_some(Atom((self.0 >> 4) as u32))
    }

    #[inline]
    pub fn as_obj(self) -> Option<*const GcBox<QObject>> {
        (self.heap_kind() == Some(HeapKind::Object))
            .then(|| self.as_heap_ptr().map(|ptr| ptr as *const GcBox<QObject>))
            .flatten()
    }

    #[inline]
    pub fn heap_kind(self) -> Option<HeapKind> {
        self.as_heap_ptr().map(|ptr| unsafe { (*ptr).kind })
    }

    #[inline]
    pub fn type_name(self) -> &'static str {
        let bits = self.0;

        // Optimized: direct bitwise dispatch instead of multiple function calls
        // This removes 5+ function calls from the hot path
        if (bits & QNAN) != QNAN {
            // Untagged float
            return "number";
        }

        // Tagged value - dispatch on tag
        match bits & TAG_MASK {
            TAG_INT => "number",
            TAG_BOOL => "bool",
            TAG_NULL => "null",
            TAG_UNDEF | TAG_EMPTY => "undefined",
            TAG_ATOM => "string",
            TAG_HEAP => {
                // For heap objects, we still need to look at the header
                if let Some(kind) = self.heap_kind() {
                    return kind.type_name();
                }
                "object"
            }
            _ => "unknown",
        }
    }

    #[inline]
    fn integer_value(self) -> Option<i128> {
        if let Some(value) = self.as_i32() {
            return Some(value as i128);
        }
        let value = self.as_f64()?;
        if !value.is_finite() || value.fract() != 0.0 {
            return None;
        }
        Some(value as i128)
    }

    #[inline]
    fn unsigned_integer_value(self) -> Option<u128> {
        self.integer_value()
            .and_then(|value| u128::try_from(value).ok())
    }

    #[inline(always)]
    pub fn is_null(self) -> bool {
        self.is_tagged() && self.tag() == TAG_NULL
    }

    #[inline(always)]
    pub fn is_undefined(self) -> bool {
        self.is_tagged() && matches!(self.tag(), TAG_UNDEF | TAG_EMPTY)
    }

    #[inline(always)]
    pub fn is_empty(self) -> bool {
        self.is_tagged() && self.tag() == TAG_EMPTY
    }

    #[inline]
    pub fn is_truthy(self) -> bool {
        let bits = self.0;

        // Optimized: direct bitwise inspection instead of multiple function calls
        // Fast path: untagged float
        if (bits & QNAN) != QNAN {
            let f = f64::from_bits(bits);
            return f != 0.0 && !f.is_nan();
        }

        // Tagged value - dispatch on tag
        match bits & TAG_MASK {
            TAG_BOOL => ((bits >> 4) & 1) != 0,
            TAG_INT => Self::int_payload(bits) != 0,
            TAG_NULL | TAG_UNDEF | TAG_EMPTY => false,
            TAG_HEAP => !self.is_html_dda(),
            _ => true, // Objects, strings, etc. are truthy
        }
    }

    #[inline]
    fn is_html_dda(self) -> bool {
        if self.heap_kind() != Some(HeapKind::Object) {
            return false;
        }
        if let Ok(obj) = Gc::<QObject>::try_from(self) {
            return obj.borrow().is_html_dda;
        }
        false
    }

    #[inline(always)]
    #[allow(clippy::wrong_self_convention)]
    pub(crate) fn to_number_ecma(&self) -> f64 {
        let bits = self.0;

        // Optimized: bitwise AND check instead of shift (faster on modern CPUs)
        if (bits & QNAN) != QNAN {
            return f64::from_bits(bits);
        }

        // Tagged value - extract tag and dispatch
        let tag_val = bits & TAG_MASK;
        match tag_val {
            TAG_INT => Self::int_payload(bits) as f64,
            TAG_BOOL => {
                if ((bits >> 4) & 1) != 0 {
                    1.0
                } else {
                    0.0
                }
            }
            TAG_NULL => 0.0,
            TAG_UNDEF | TAG_EMPTY => f64::NAN,
            _ => f64::NAN,
        }
    }

    #[inline(always)]
    #[allow(clippy::wrong_self_convention)]
    pub(crate) fn to_i32(&self) -> i32 {
        let bits = self.0;

        // Optimized: bitwise AND instead of shift
        if (bits & QNAN) != QNAN {
            let f = f64::from_bits(bits);
            if f.is_nan() || f.is_infinite() {
                return 0;
            }
            return f as i32;
        }

        // Tagged: extract tag and dispatch
        let tag_val = bits & TAG_MASK;
        match tag_val {
            TAG_INT => Self::int_payload(bits),
            TAG_BOOL => {
                if ((bits >> 4) & 1) != 0 {
                    1
                } else {
                    0
                }
            }
            TAG_NULL => 0,
            TAG_UNDEF | TAG_EMPTY => 0,
            _ => 0,
        }
    }

    pub fn from_json(_ctx: &Context, _json: &str) -> Result<Self, JsonValueError> {
        Err(JsonValueError::parse(
            "JSON support is not available in this register-vm build",
        ))
    }

    pub fn from_serde<T>(_ctx: &Context, _value: &T) -> Result<Self, SerdeValueError> {
        Err(SerdeValueError::unsupported("serde"))
    }

    pub fn from_serde_json<T>(_ctx: &Context, _value: &T) -> Self {
        Self::undefined()
    }

    pub fn to_json(self, _ctx: &Context) -> Result<String, JsonValueError> {
        let _ = self;
        Err(JsonValueError::parse(
            "JSON support is not available in this register-vm build",
        ))
    }

    pub fn to_serde<T>(self, _ctx: &Context) -> Result<T, SerdeValueError> {
        let _ = self;
        Err(SerdeValueError::unsupported("serde"))
    }

    pub fn to_serde_json(self, _ctx: &Context) -> Result<String, SerdeValueError> {
        let _ = self;
        Err(SerdeValueError::unsupported("serde_json"))
    }

    pub fn to_pretty_json(self, _ctx: &Context) -> Result<String, JsonValueError> {
        let _ = self;
        Err(JsonValueError::parse(
            "JSON support is not available in this register-vm build",
        ))
    }

    pub fn from_yaml(_ctx: &Context, _yaml: &str) -> Result<Self, YamlValueError> {
        Err(YamlValueError::parse(
            "YAML support is not available in this register-vm build",
        ))
    }

    pub fn to_yaml(self, _ctx: &Context) -> Result<String, YamlValueError> {
        let _ = self;
        Err(YamlValueError::parse(
            "YAML support is not available in this register-vm build",
        ))
    }

    pub fn from_msgpack(_ctx: &Context, _bytes: &[u8]) -> Result<Self, MsgpackValueError> {
        Err(MsgpackValueError::parse(
            "MsgPack support is not available in this register-vm build",
        ))
    }

    pub fn to_msgpack(self, _ctx: &Context) -> Result<Vec<u8>, MsgpackValueError> {
        let _ = self;
        Err(MsgpackValueError::parse(
            "MsgPack support is not available in this register-vm build",
        ))
    }

    pub fn from_arena_buffer(_ctx: &Context, _bytes: &[u8]) -> Result<Self, ValueError> {
        Err(ValueError::unsupported("arena buffer"))
    }

    pub fn to_arena_buffer(self, _ctx: &Context) -> Result<Vec<u8>, ValueError> {
        let _ = self;
        Err(ValueError::unsupported("arena buffer"))
    }
}

impl std::fmt::Debug for Value {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(v) = self.as_i32() {
            return f.debug_tuple("Int").field(&v).finish();
        }
        if let Some(v) = self.as_bool() {
            return f.debug_tuple("Bool").field(&v).finish();
        }
        if let Some(v) = self.as_f64() {
            return f.debug_tuple("Float").field(&v).finish();
        }
        if let Some(v) = self.as_atom() {
            return f.debug_tuple("Atom").field(&v.0).finish();
        }
        if let Some(kind) = self.heap_kind() {
            return f.debug_tuple("Heap").field(&kind).finish();
        }
        if self.is_null() {
            return f.write_str("Null");
        }
        if self.is_undefined() {
            return f.write_str("Undefined");
        }
        f.write_str("Value(?)")
    }
}

impl From<Null> for Value {
    #[inline(always)]
    fn from(_: Null) -> Self {
        Self::null()
    }
}

impl From<Undefined> for Value {
    #[inline(always)]
    fn from(_: Undefined) -> Self {
        Self::undefined()
    }
}

impl From<()> for Value {
    #[inline(always)]
    fn from(_: ()) -> Self {
        Self::null()
    }
}

impl From<bool> for Value {
    #[inline(always)]
    fn from(value: bool) -> Self {
        Self::bool(value)
    }
}

macro_rules! impl_exact_int_value_from {
    ($($ty:ty),+ $(,)?) => {
        $(
            impl From<$ty> for Value {
                #[inline(always)]
                fn from(value: $ty) -> Self {
                    Self::i32(i32::from(value))
                }
            }
        )+
    };
}

macro_rules! impl_fallible_int_value_from {
    ($($ty:ty),+ $(,)?) => {
        $(
            impl From<$ty> for Value {
                #[inline]
                fn from(value: $ty) -> Self {
                    match i32::try_from(value) {
                        Ok(value) => Self::i32(value),
                        Err(_) => Self::f64(value as f64),
                    }
                }
            }
        )+
    };
}

macro_rules! impl_float_value_from {
    ($($ty:ty),+ $(,)?) => {
        $(
            impl From<$ty> for Value {
                #[inline(always)]
                fn from(value: $ty) -> Self {
                    Self::f64(value as f64)
                }
            }
        )+
    };
}

impl_exact_int_value_from!(i8, i16, u8, u16);
impl_fallible_int_value_from!(i32, isize, i64, i128, u32, usize, u64, u128);
impl_float_value_from!(f32, f64);

impl From<Atom> for Value {
    #[inline(always)]
    fn from(value: Atom) -> Self {
        Self::atom(value)
    }
}

impl From<&Atom> for Value {
    #[inline(always)]
    fn from(value: &Atom) -> Self {
        Self::atom(*value)
    }
}

impl TryFrom<Value> for Null {
    type Error = ValueTypeError;

    fn try_from(value: Value) -> Result<Self, Self::Error> {
        value
            .is_null()
            .then_some(Null)
            .ok_or_else(|| ValueTypeError::new("null", value.type_name()))
    }
}

impl TryFrom<Value> for Undefined {
    type Error = ValueTypeError;

    fn try_from(value: Value) -> Result<Self, Self::Error> {
        value
            .is_undefined()
            .then_some(Undefined)
            .ok_or_else(|| ValueTypeError::new("undefined", value.type_name()))
    }
}

impl TryFrom<Value> for bool {
    type Error = ValueTypeError;

    #[inline]
    fn try_from(value: Value) -> Result<Self, Self::Error> {
        value
            .as_bool()
            .ok_or_else(|| ValueTypeError::new("bool", value.type_name()))
    }
}

macro_rules! impl_try_from_signed_int {
    ($($ty:ty),+ $(,)?) => {
        $(
            impl TryFrom<Value> for $ty {
                type Error = ValueTypeError;

                #[inline]
                fn try_from(value: Value) -> Result<Self, Self::Error> {
                    let integer = value
                        .integer_value()
                        .ok_or_else(|| ValueTypeError::new("number", value.type_name()))?;

                    <$ty>::try_from(integer)
                        .map_err(|_| ValueTypeError::new(stringify!($ty), value.type_name()))
                }
            }
        )+
    };
}

macro_rules! impl_try_from_unsigned_int {
    ($($ty:ty),+ $(,)?) => {
        $(
            impl TryFrom<Value> for $ty {
                type Error = ValueTypeError;

                #[inline]
                fn try_from(value: Value) -> Result<Self, Self::Error> {
                    let integer = value
                        .unsigned_integer_value()
                        .ok_or_else(|| ValueTypeError::new("number", value.type_name()))?;

                    <$ty>::try_from(integer)
                        .map_err(|_| ValueTypeError::new(stringify!($ty), value.type_name()))
                }
            }
        )+
    };
}

macro_rules! impl_try_from_float {
    ($($ty:ty),+ $(,)?) => {
        $(
            impl TryFrom<Value> for $ty {
                type Error = ValueTypeError;

                #[inline]
                fn try_from(value: Value) -> Result<Self, Self::Error> {
                    if let Some(integer) = value.as_i32() {
                        return Ok(integer as $ty);
                    }

                    value
                        .as_f64()
                        .map(|v| v as $ty)
                        .ok_or_else(|| ValueTypeError::new("number", value.type_name()))
                }
            }
        )+
    };
}

impl_try_from_signed_int!(i8, i16, i32, i64, i128, isize);
impl_try_from_unsigned_int!(u8, u16, u32, u64, u128, usize);
impl_try_from_float!(f32, f64);

impl TryFrom<Value> for Atom {
    type Error = ValueTypeError;

    fn try_from(value: Value) -> Result<Self, Self::Error> {
        value
            .as_atom()
            .ok_or_else(|| ValueTypeError::new("string", value.type_name()))
    }
}

impl<T: Trace + AtomTrace + HeapTyped + 'static> TryFrom<Value> for Gc<T> {
    type Error = ValueTypeError;

    fn try_from(value: Value) -> Result<Self, Self::Error> {
        let ptr = value
            .as_heap_ptr()
            .ok_or_else(|| ValueTypeError::new(T::KIND.type_name(), value.type_name()))?;

        let kind = unsafe { (*ptr).kind };
        if kind != T::KIND {
            return Err(ValueTypeError::new(T::KIND.type_name(), kind.type_name()));
        }

        let ptr = ptr as *const GcBox<T>;

        unsafe {
            Rc::increment_strong_count(ptr);
            Ok(Gc {
                inner: Rc::from_raw(ptr),
            })
        }
    }
}

impl<T: Trace + AtomTrace + HeapTyped + 'static> From<&Gc<T>> for Value {
    fn from(value: &Gc<T>) -> Self {
        Self::heap(value.as_ptr() as *const GCHeader)
    }
}

impl<T: Trace + AtomTrace + HeapTyped + 'static> From<Gc<T>> for Value {
    fn from(value: Gc<T>) -> Self {
        Self::from(&value)
    }
}

// ============================================================================
// impl From for Value - String types
// ============================================================================

impl From<String> for Value {
    fn from(value: String) -> Self {
        Context::with_current(|ctx| {
            let string = ctx.new_string(&value);
            Self::from(&string)
        })
    }
}

impl From<&String> for Value {
    fn from(value: &String) -> Self {
        Context::with_current(|ctx| {
            let string = ctx.new_string(value);
            Self::from(&string)
        })
    }
}

impl From<&str> for Value {
    fn from(value: &str) -> Self {
        Context::with_current(|ctx| {
            let string = ctx.new_string(value);
            Self::from(&string)
        })
    }
}

// UTF-16 to QString
impl From<Vec<u16>> for Value {
    fn from(value: Vec<u16>) -> Self {
        Context::with_current(|ctx| {
            let s = String::from_utf16_lossy(&value).to_string();
            let string = ctx.new_string(&s);
            Self::from(&string)
        })
    }
}

impl From<&[u16]> for Value {
    fn from(value: &[u16]) -> Self {
        Context::with_current(|ctx| {
            let s = String::from_utf16_lossy(value).to_string();
            let string = ctx.new_string(&s);
            Self::from(&string)
        })
    }
}

// ============================================================================
// impl From for Value - Collections: Arrays
// ============================================================================

impl From<Vec<Value>> for Value {
    fn from(value: Vec<Value>) -> Self {
        Context::with_current(|ctx| {
            let array = ctx.new_array();
            {
                let mut array_mut = array.borrow_mut();
                for v in value {
                    array_mut.push(v);
                }
            }
            Self::from(&array)
        })
    }
}

impl From<&[Value]> for Value {
    fn from(value: &[Value]) -> Self {
        Context::with_current(|ctx| {
            let array = ctx.new_array();
            {
                let mut array_mut = array.borrow_mut();
                for &v in value {
                    array_mut.push(v);
                }
            }
            Self::from(&array)
        })
    }
}

impl From<Vec<bool>> for Value {
    fn from(value: Vec<bool>) -> Self {
        Context::with_current(|ctx| {
            let array = ctx.new_bool_array();
            {
                let mut array_mut = array.borrow_mut();
                for v in value {
                    array_mut.push(v);
                }
            }
            Self::from(&array)
        })
    }
}

impl From<&[bool]> for Value {
    fn from(value: &[bool]) -> Self {
        Context::with_current(|ctx| {
            let array = ctx.new_bool_array();
            {
                let mut array_mut = array.borrow_mut();
                for &v in value {
                    array_mut.push(v);
                }
            }
            Self::from(&array)
        })
    }
}

impl From<Vec<u8>> for Value {
    fn from(value: Vec<u8>) -> Self {
        Context::with_current(|ctx| {
            let array = ctx.new_uint8_array();
            {
                let mut array_mut = array.borrow_mut();
                for v in value {
                    array_mut.push(v);
                }
            }
            Self::from(&array)
        })
    }
}

impl From<&[u8]> for Value {
    fn from(value: &[u8]) -> Self {
        Context::with_current(|ctx| {
            let array = ctx.new_uint8_array();
            {
                let mut array_mut = array.borrow_mut();
                for &v in value {
                    array_mut.push(v);
                }
            }
            Self::from(&array)
        })
    }
}

impl From<Vec<i32>> for Value {
    fn from(value: Vec<i32>) -> Self {
        Context::with_current(|ctx| {
            let array = ctx.new_int32_array();
            {
                let mut array_mut = array.borrow_mut();
                for v in value {
                    array_mut.push(v);
                }
            }
            Self::from(&array)
        })
    }
}

impl From<&[i32]> for Value {
    fn from(value: &[i32]) -> Self {
        Context::with_current(|ctx| {
            let array = ctx.new_int32_array();
            {
                let mut array_mut = array.borrow_mut();
                for &v in value {
                    array_mut.push(v);
                }
            }
            Self::from(&array)
        })
    }
}

impl From<Vec<f64>> for Value {
    fn from(value: Vec<f64>) -> Self {
        Context::with_current(|ctx| {
            let array = ctx.new_float64_array();
            {
                let mut array_mut = array.borrow_mut();
                for v in value {
                    array_mut.push(v);
                }
            }
            Self::from(&array)
        })
    }
}

impl From<&[f64]> for Value {
    fn from(value: &[f64]) -> Self {
        Context::with_current(|ctx| {
            let array = ctx.new_float64_array();
            {
                let mut array_mut = array.borrow_mut();
                for &v in value {
                    array_mut.push(v);
                }
            }
            Self::from(&array)
        })
    }
}

// ============================================================================
// impl From for Value - String Arrays
// ============================================================================

impl From<Vec<String>> for Value {
    fn from(value: Vec<String>) -> Self {
        Context::with_current(|ctx| {
            let array = ctx.new_string_array();
            {
                let mut array_mut = array.borrow_mut();
                for s in value {
                    let string = ctx.new_string(&s);
                    array_mut.push(string.into());
                }
            }
            Self::from(&array)
        })
    }
}

impl From<&[String]> for Value {
    fn from(value: &[String]) -> Self {
        Context::with_current(|ctx| {
            let array = ctx.new_string_array();
            {
                let mut array_mut = array.borrow_mut();
                for s in value {
                    let string = ctx.new_string(s);
                    array_mut.push(string.into());
                }
            }
            Self::from(&array)
        })
    }
}

impl From<Vec<Atom>> for Value {
    fn from(value: Vec<Atom>) -> Self {
        Context::with_current(|ctx| {
            let array = ctx.new_string_array();
            {
                let mut array_mut = array.borrow_mut();
                for atom in value {
                    let string = ctx.new_string(&ctx.resolve(atom));
                    array_mut.push(string.into());
                }
            }
            Self::from(&array)
        })
    }
}

impl From<&[Atom]> for Value {
    fn from(value: &[Atom]) -> Self {
        Context::with_current(|ctx| {
            let array = ctx.new_string_array();
            {
                let mut array_mut = array.borrow_mut();
                for &atom in value {
                    let string = ctx.new_string(&ctx.resolve(atom));
                    array_mut.push(string.into());
                }
            }
            Self::from(&array)
        })
    }
}

// UTF-16 String Arrays
impl From<Vec<Vec<u16>>> for Value {
    fn from(value: Vec<Vec<u16>>) -> Self {
        Context::with_current(|ctx| {
            let array = ctx.new_string_array();
            {
                let mut array_mut = array.borrow_mut();
                for utf16_string in value {
                    let s = String::from_utf16_lossy(&utf16_string).to_string();
                    let string = ctx.new_string(&s);
                    array_mut.push(string.into());
                }
            }
            Self::from(&array)
        })
    }
}

impl From<&[Vec<u16>]> for Value {
    fn from(value: &[Vec<u16>]) -> Self {
        Context::with_current(|ctx| {
            let array = ctx.new_string_array();
            {
                let mut array_mut = array.borrow_mut();
                for utf16_string in value {
                    let s = String::from_utf16_lossy(utf16_string).to_string();
                    let string = ctx.new_string(&s);
                    array_mut.push(string.into());
                }
            }
            Self::from(&array)
        })
    }
}

// ============================================================================
// impl From for Value - Objects (HashMap)
// ============================================================================

impl From<HashMap<String, Value>> for Value {
    fn from(value: HashMap<String, Value>) -> Self {
        Context::with_current(|ctx| {
            let object = ctx.new_object();
            {
                let mut object_mut = object.borrow_mut();
                for (key, val) in value {
                    let atom = ctx.intern(&key);
                    object_mut.set(atom, val);
                }
            }
            Self::from(&object)
        })
    }
}

impl From<&HashMap<String, Value>> for Value {
    fn from(value: &HashMap<String, Value>) -> Self {
        Context::with_current(|ctx| {
            let object = ctx.new_object();
            {
                let mut object_mut = object.borrow_mut();
                for (key, val) in value {
                    let atom = ctx.intern(key);
                    object_mut.set(atom, *val);
                }
            }
            Self::from(&object)
        })
    }
}

impl From<HashMap<Atom, Value>> for Value {
    fn from(value: HashMap<Atom, Value>) -> Self {
        Context::with_current(|ctx| {
            let object = ctx.new_object();
            {
                let mut object_mut = object.borrow_mut();
                for (key, val) in value {
                    object_mut.set(key, val);
                }
            }
            Self::from(&object)
        })
    }
}

impl From<&HashMap<Atom, Value>> for Value {
    fn from(value: &HashMap<Atom, Value>) -> Self {
        Context::with_current(|ctx| {
            let object = ctx.new_object();
            {
                let mut object_mut = object.borrow_mut();
                for (key, val) in value {
                    object_mut.set(*key, *val);
                }
            }
            Self::from(&object)
        })
    }
}
// This module contains trait implementations for Value
// It will be included in value.rs

impl ArithmeticOps for Value {
    #[inline(always)]
    fn add(&self, rhs: &Self) -> Self {
        let a = self.0;
        let b = rhs.0;

        // String concatenation needs the current Context (atom resolve/intern).
        // Keep numeric loops free of TLS/closure overhead.
        let tag_a = a & TAG_MASK;
        let tag_b = b & TAG_MASK;
        if (a & QNAN) == QNAN && (tag_a == TAG_ATOM || tag_b == TAG_ATOM) {
            return Context::with_current(|ctx| Self::add_with_context(ctx, self, rhs));
        }

        // FAST FLOAT PATH (OR-tag dispatch)
        if ((a | b) & QNAN) != QNAN {
            return Value::f64(f64::from_bits(a) + f64::from_bits(b));
        }

        // FAST INT PATH
        if Self::is_tagged_int_bits(a) && Self::is_tagged_int_bits(b) {
            let a_int = Self::int_payload(a);
            let b_int = Self::int_payload(b);
            if let Some(result) = a_int.checked_add(b_int) {
                return Value::i32(result);
            }
            return Value::f64((a_int as f64) + (b_int as f64));
        }

        // SLOW PATH (numeric coercion)
        Value::f64(self.to_number_ecma() + rhs.to_number_ecma())
    }
    #[inline(always)]
    fn sub(&self, rhs: &Self) -> Self {
        let a = self.0;
        let b = rhs.0;

        // FAST FLOAT PATH (OR-tag dispatch)
        if ((a | b) & QNAN) != QNAN {
            return Value::f64(f64::from_bits(a) - f64::from_bits(b));
        }

        // FAST INT PATH
        if Self::is_tagged_int_bits(a) && Self::is_tagged_int_bits(b) {
            let a_int = Self::int_payload(a);
            let b_int = Self::int_payload(b);
            if let Some(result) = a_int.checked_sub(b_int) {
                return Value::i32(result);
            }
            return Value::f64((a_int as f64) - (b_int as f64));
        }

        Value::f64(self.to_number_ecma() - rhs.to_number_ecma())
    }
    #[inline(always)]
    fn mul(&self, rhs: &Self) -> Self {
        let a = self.0;
        let b = rhs.0;

        // FAST FLOAT PATH (OR-tag dispatch)
        if ((a | b) & QNAN) != QNAN {
            return Value::f64(f64::from_bits(a) * f64::from_bits(b));
        }

        // FAST INT PATH
        if Self::is_tagged_int_bits(a) && Self::is_tagged_int_bits(b) {
            let a_int = Self::int_payload(a);
            let b_int = Self::int_payload(b);
            if let Some(result) = a_int.checked_mul(b_int) {
                return Value::i32(result);
            }
            return Value::f64((a_int as f64) * (b_int as f64));
        }

        Value::f64(self.to_number_ecma() * rhs.to_number_ecma())
    }
    #[inline(always)]
    fn div(&self, rhs: &Self) -> Self {
        let a = self.0;
        let b = rhs.0;

        // FAST FLOAT PATH (OR-tag dispatch)
        if ((a | b) & QNAN) != QNAN {
            return Value::f64(f64::from_bits(a) / f64::from_bits(b));
        }

        // FAST INT PATH
        if Self::is_tagged_int_bits(a) && Self::is_tagged_int_bits(b) {
            let a_int = Self::int_payload(a);
            let b_int = Self::int_payload(b);
            // Let IEEE 754 handle it: 0/0 = NaN, x/0 = ±Infinity
            return Value::f64((a_int as f64) / (b_int as f64));
        }

        Value::f64(self.to_number_ecma() / rhs.to_number_ecma())
    }
    #[inline(always)]
    fn rem(&self, rhs: &Self) -> Self {
        let a = self.0;
        let b = rhs.0;

        // FAST FLOAT PATH (OR-tag dispatch)
        if ((a | b) & QNAN) != QNAN {
            return Value::f64(f64::from_bits(a) % f64::from_bits(b));
        }

        // FAST INT PATH
        if Self::is_tagged_int_bits(a) && Self::is_tagged_int_bits(b) {
            let a_int = Self::int_payload(a);
            let b_int = Self::int_payload(b);
            if b_int == 0 {
                return Value::f64(f64::NAN);
            }
            if let Some(result) = a_int.checked_rem(b_int) {
                return Value::i32(result);
            }
            return Value::f64((a_int as f64) % (b_int as f64));
        }

        Value::f64(self.to_number_ecma() % rhs.to_number_ecma())
    }
    #[inline(always)]
    fn pow(&self, rhs: &Self) -> Self {
        Value::f64(ecma_number_pow(self.to_number_ecma(), rhs.to_number_ecma()))
    }
    #[inline(always)]
    fn inc(&self) -> Self {
        self.add(&Value::i32(1))
    }
    #[inline(always)]
    fn dec(&self) -> Self {
        self.sub(&Value::i32(1))
    }
    #[inline(always)]
    fn unary_plus(&self) -> Self {
        self.to_number()
    }
    #[inline(always)]
    fn unary_minus(&self) -> Self {
        Value::f64(-self.to_number_ecma())
    }
}

impl ComparisonOps for Value {
    #[inline(always)]
    fn eq(&self, rhs: &Self) -> Self {
        // Fast path: identical bits
        if self.0 == rhs.0 {
            return Value::bool(true);
        }

        // null == undefined (ECMAScript spec)
        if (self.is_null() && rhs.is_undefined()) || (self.is_undefined() && rhs.is_null()) {
            return Value::bool(true);
        }

        // null/undefined are ONLY equal to each other (already handled above)
        if self.is_null() || self.is_undefined() || rhs.is_null() || rhs.is_undefined() {
            return Value::bool(false);
        }

        // Atom comparisons need Context (resolve/parse/coerce).
        let a = self.0;
        let b = rhs.0;
        if (a & QNAN) == QNAN && ((a & TAG_MASK) == TAG_ATOM || (b & TAG_MASK) == TAG_ATOM) {
            return Context::with_current(|ctx| Self::eq_with_context(ctx, self, rhs));
        }

        // Everything else: numeric coercion (no Context needed).
        Value::bool(self.to_number_ecma() == rhs.to_number_ecma())
    }
    #[inline(always)]
    fn ne(&self, rhs: &Self) -> Self {
        let eq_val = <Self as ComparisonOps>::eq(self, rhs);
        Value::bool(!eq_val.as_bool().unwrap())
    }
    #[inline(always)]
    fn strict_eq(&self, rhs: &Self) -> Self {
        Value::bool(self.0 == rhs.0)
    }
    #[inline(always)]
    fn strict_ne(&self, rhs: &Self) -> Self {
        Value::bool(self.0 != rhs.0)
    }
    #[inline(always)]
    fn gt(&self, rhs: &Self) -> Self {
        Value::bool(self.to_number_ecma() > rhs.to_number_ecma())
    }
    #[inline(always)]
    fn lt(&self, rhs: &Self) -> Self {
        Value::bool(self.to_number_ecma() < rhs.to_number_ecma())
    }
    #[inline(always)]
    fn ge(&self, rhs: &Self) -> Self {
        Value::bool(self.to_number_ecma() >= rhs.to_number_ecma())
    }
    #[inline(always)]
    fn le(&self, rhs: &Self) -> Self {
        Value::bool(self.to_number_ecma() <= rhs.to_number_ecma())
    }
}

impl LogicalOps for Value {
    #[inline(always)]
    fn logical_and(&self, rhs: &Self) -> Self {
        if self.is_truthy() { *rhs } else { *self }
    }

    #[inline(always)]
    fn logical_or(&self, rhs: &Self) -> Self {
        if self.is_truthy() { *self } else { *rhs }
    }

    #[inline(always)]
    fn logical_not(&self) -> Self {
        Value::bool(!self.is_truthy())
    }
}

impl BitwiseOps for Value {
    #[inline(always)]
    fn bit_and(&self, rhs: &Self) -> Self {
        let a = self.0;
        let b = rhs.0;

        // Fast path for both ints - bitwise AND optimization
        if (a & QNAN) == QNAN
            && (b & QNAN) == QNAN
            && (a & TAG_MASK) == TAG_INT
            && (b & TAG_MASK) == TAG_INT
        {
            return Value::i32(Self::int_payload(a) & Self::int_payload(b));
        }
        let a_int = self.to_i32() as u32;
        let b_int = rhs.to_i32() as u32;
        Value::i32((a_int & b_int) as i32)
    }

    #[inline(always)]
    fn bit_or(&self, rhs: &Self) -> Self {
        let a = self.0;
        let b = rhs.0;

        // Fast path for both ints - bitwise AND optimization
        if (a & QNAN) == QNAN
            && (b & QNAN) == QNAN
            && (a & TAG_MASK) == TAG_INT
            && (b & TAG_MASK) == TAG_INT
        {
            return Value::i32(Self::int_payload(a) | Self::int_payload(b));
        }
        let a_int = self.to_i32() as u32;
        let b_int = rhs.to_i32() as u32;
        Value::i32((a_int | b_int) as i32)
    }

    #[inline(always)]
    fn bit_xor(&self, rhs: &Self) -> Self {
        let a = self.0;
        let b = rhs.0;

        // Fast path for both ints - bitwise AND optimization
        if (a & QNAN) == QNAN
            && (b & QNAN) == QNAN
            && (a & TAG_MASK) == TAG_INT
            && (b & TAG_MASK) == TAG_INT
        {
            return Value::i32(Self::int_payload(a) ^ Self::int_payload(b));
        }
        let a_int = self.to_i32() as u32;
        let b_int = rhs.to_i32() as u32;
        Value::i32((a_int ^ b_int) as i32)
    }

    #[inline(always)]
    fn bit_not(&self) -> Self {
        let bits = self.0;

        // Fast path for int - bitwise AND optimization
        if (bits & QNAN) == QNAN && (bits & TAG_MASK) == TAG_INT {
            return Value::i32(!Self::int_payload(bits));
        }
        let a_int = self.to_i32() as u32;
        Value::i32((!a_int) as i32)
    }

    #[inline(always)]
    fn shl(&self, rhs: &Self) -> Self {
        let a = self.0;
        let b = rhs.0;

        // Fast path for both ints - bitwise AND optimization
        if (a & QNAN) == QNAN
            && (b & QNAN) == QNAN
            && (a & TAG_MASK) == TAG_INT
            && (b & TAG_MASK) == TAG_INT
        {
            return Value::i32(Self::int_payload(a) << (Self::int_payload(b) as u32 & 31));
        }
        let a_int = self.to_i32() as u32;
        let b_int = rhs.to_i32() as u32 & 31;
        Value::i32((a_int << b_int) as i32)
    }

    #[inline(always)]
    fn shr(&self, rhs: &Self) -> Self {
        let a = self.0;
        let b = rhs.0;

        // Fast path for both ints - bitwise AND optimization
        if (a & QNAN) == QNAN
            && (b & QNAN) == QNAN
            && (a & TAG_MASK) == TAG_INT
            && (b & TAG_MASK) == TAG_INT
        {
            return Value::i32(Self::int_payload(a) >> (Self::int_payload(b) as u32 & 31));
        }
        let a_int = self.to_i32();
        let b_int = rhs.to_i32() as u32 & 31;
        Value::i32(a_int >> b_int)
    }

    #[inline(always)]
    fn ushr(&self, rhs: &Self) -> Self {
        let a = self.0;
        let b = rhs.0;

        // Fast path for both ints - bitwise AND optimization
        if (a & QNAN) == QNAN
            && (b & QNAN) == QNAN
            && (a & TAG_MASK) == TAG_INT
            && (b & TAG_MASK) == TAG_INT
        {
            let a_int = Self::int_payload(a) as u32;
            let b_int = Self::int_payload(b) as u32 & 31;
            return Value::i32((a_int >> b_int) as i32);
        }
        let a_int = self.to_i32() as u32;
        let b_int = rhs.to_i32() as u32 & 31;
        Value::i32((a_int >> b_int) as i32)
    }
}

impl AssignmentOps for Value {
    #[inline(always)]
    fn assign(&mut self, rhs: Self) {
        *self = rhs;
    }

    #[inline(always)]
    fn add_assign(&mut self, rhs: Self) {
        *self = self.add(&rhs);
    }

    #[inline(always)]
    fn sub_assign(&mut self, rhs: Self) {
        *self = self.sub(&rhs);
    }

    #[inline(always)]
    fn mul_assign(&mut self, rhs: Self) {
        *self = self.mul(&rhs);
    }

    #[inline(always)]
    fn div_assign(&mut self, rhs: Self) {
        *self = self.div(&rhs);
    }

    #[inline(always)]
    fn rem_assign(&mut self, rhs: Self) {
        *self = self.rem(&rhs);
    }

    #[inline(always)]
    fn pow_assign(&mut self, rhs: Self) {
        *self = self.pow(&rhs);
    }

    #[inline(always)]
    fn shl_assign(&mut self, rhs: Self) {
        *self = self.shl(&rhs);
    }

    #[inline(always)]
    fn shr_assign(&mut self, rhs: Self) {
        *self = self.shr(&rhs);
    }

    #[inline(always)]
    fn ushr_assign(&mut self, rhs: Self) {
        *self = self.ushr(&rhs);
    }

    #[inline(always)]
    fn bit_and_assign(&mut self, rhs: Self) {
        *self = self.bit_and(&rhs);
    }

    #[inline(always)]
    fn bit_or_assign(&mut self, rhs: Self) {
        *self = self.bit_or(&rhs);
    }

    #[inline(always)]
    fn bit_xor_assign(&mut self, rhs: Self) {
        *self = self.bit_xor(&rhs);
    }
}

impl LogicalAssignOps for Value {
    #[inline(always)]
    fn and_assign(&mut self, rhs: Self) {
        if self.is_truthy() {
            *self = rhs;
        }
    }

    #[inline(always)]
    fn or_assign(&mut self, rhs: Self) {
        if !self.is_truthy() {
            *self = rhs;
        }
    }
}

impl NullishOps for Value {
    #[inline(always)]
    fn nullish_coalesce(&self, rhs: &Self) -> Self {
        if self.is_null() || self.is_undefined() {
            *rhs
        } else {
            *self
        }
    }

    #[inline(always)]
    fn nullish_assign(&mut self, rhs: Self) {
        if self.is_null() || self.is_undefined() {
            *self = rhs;
        }
    }
}

impl TypeOps for Value {
    #[inline(always)]
    fn typeof_(&self) -> Self {
        Context::with_current(|ctx| self.typeof_with_context(ctx))
    }
    #[inline(always)]
    fn instanceof(&self, _rhs: &Self) -> Self {
        Value::bool(false)
    }
    #[inline(always)]
    fn in_(&self, _rhs: &Self) -> Self {
        Value::bool(false)
    }
    #[inline(always)]
    fn delete(&self) -> Self {
        Value::bool(true)
    }
}

impl CoercionOps for Value {
    #[inline(always)]
    fn to_number(&self) -> Self {
        Value::f64(self.to_number_ecma())
    }
    #[inline]
    fn to_string(&self) -> Self {
        Context::with_current(|ctx| self.to_string_with_context(ctx))
    }
    #[inline(always)]
    fn to_boolean(&self) -> Self {
        Value::bool(self.is_truthy())
    }
    #[inline]
    fn to_primitive(&self) -> Self {
        *self
    }
}

impl PropertyOps for Value {
    #[inline(always)]
    fn get(&self, _key: &Self) -> Self {
        // Simplified: would need object support
        Value::undefined()
    }

    #[inline(always)]
    fn set(&mut self, _key: Self, _value: Self) {
        // Simplified: would need object support
    }

    #[inline(always)]
    fn has(&self, _key: &Self) -> Self {
        Value::bool(false)
    }

    #[inline(always)]
    fn delete_property(&mut self, _key: &Self) -> Self {
        Value::bool(true)
    }
}

impl CallOps for Value {
    #[inline(always)]
    fn call(&self, this_value: &Self, _args: &[Self]) -> Self {
        let bits = self.0;
        if (bits & QNAN) == QNAN
            && (bits & TAG_MASK) == TAG_HEAP
            && let Some(kind) = self.heap_kind()
        {
            match kind {
                HeapKind::NativeFunction => {
                    if let Ok(func) = Gc::<QNativeFunction>::try_from(*self) {
                        return Context::with_current(|ctx| {
                            (func.borrow().callback)(ctx, *this_value, _args)
                        });
                    }
                }
                HeapKind::NativeClosure => {
                    if let Ok(func) = Gc::<QNativeClosure>::try_from(*self) {
                        return Context::with_current(|ctx| {
                            (func.borrow().callback)(ctx, *this_value, _args)
                        });
                    }
                }
                HeapKind::Object => {
                    if let Ok(obj) = Gc::<QObject>::try_from(*self) {
                        return Context::with_current(|ctx| {
                            let atom_bound_target = ctx.intern("__vm_bound_target");
                            let atom_bound_this = ctx.intern("__vm_bound_this");
                            let atom_bound_args = ctx.intern("__vm_bound_args");
                            let atom_native = ctx.intern("__vm_native_fn");

                            let (bound_target, bound_this, bound_args_val, native_val) = {
                                let oref = obj.borrow();
                                (
                                    oref.get(atom_bound_target).unwrap_or(Value::undefined()),
                                    oref.get(atom_bound_this).unwrap_or(Value::undefined()),
                                    oref.get(atom_bound_args).unwrap_or(Value::undefined()),
                                    oref.get(atom_native).unwrap_or(Value::undefined()),
                                )
                            };

                            if bound_target.is_truthy() {
                                let merged = merge_bound_args(bound_args_val, _args);
                                return bound_target.call(&bound_this, &merged);
                            }

                            if let Ok(func) = Gc::<QNativeFunction>::try_from(native_val) {
                                return (func.borrow().callback)(ctx, *this_value, _args);
                            }

                            make_type_error_throw(ctx, "not a function")
                        });
                    }
                }
                _ => {}
            }
        }
        Context::with_current(|ctx| make_type_error_throw(ctx, "not a function"))
    }

    #[inline(always)]
    fn construct(&self, _args: &[Self]) -> Self {
        let bits = self.0;
        if (bits & QNAN) == QNAN
            && (bits & TAG_MASK) == TAG_HEAP
            && let Some(kind) = self.heap_kind()
            && kind == HeapKind::Object
            && let Ok(obj) = Gc::<QObject>::try_from(*self)
        {
            return Context::with_current(|ctx| {
                let atom_no_construct = ctx.intern("__vm_no_construct");
                let atom_bound_target = ctx.intern("__vm_bound_target");
                let atom_bound_args = ctx.intern("__vm_bound_args");
                let atom_native = ctx.intern("__vm_native_fn");
                let atom_proto = ctx.intern("prototype");

                let (no_construct, bound_target, bound_args_val, native_val, proto_val) = {
                    let oref = obj.borrow();
                    (
                        oref.get(atom_no_construct).unwrap_or(Value::undefined()),
                        oref.get(atom_bound_target).unwrap_or(Value::undefined()),
                        oref.get(atom_bound_args).unwrap_or(Value::undefined()),
                        oref.get(atom_native).unwrap_or(Value::undefined()),
                        oref.get(atom_proto).unwrap_or(Value::undefined()),
                    )
                };

                if no_construct.is_truthy() {
                    return make_type_error_throw(ctx, "not a constructor");
                }

                if bound_target.is_truthy() {
                    let merged = merge_bound_args(bound_args_val, _args);
                    return bound_target.construct(&merged);
                }

                let instance = ctx.new_object();
                if let Ok(proto) = Gc::<QObject>::try_from(proto_val) {
                    instance.borrow_mut().prototype = Some(Value::from(proto));
                }

                if let Ok(func) = Gc::<QNativeFunction>::try_from(native_val) {
                    let result =
                        (func.borrow().callback)(ctx, Value::from(instance.clone()), _args);
                    if result.heap_kind().is_some() {
                        return result;
                    }
                    return Value::from(instance);
                }

                make_type_error_throw(ctx, "not a constructor")
            });
        }
        Context::with_current(|ctx| make_type_error_throw(ctx, "not a constructor"))
    }
}

#[inline(always)]
#[allow(dead_code)]
fn prepend_callee_arg(callee: Value, args: &[Value]) -> Vec<Value> {
    let mut merged = Vec::with_capacity(args.len() + 1);
    merged.push(callee);
    merged.extend_from_slice(args);
    merged
}

#[inline(always)]
fn merge_bound_args(bound_args: Value, args: &[Value]) -> Vec<Value> {
    let mut merged = Vec::new();
    if let Ok(arr) = Gc::<QArray>::try_from(bound_args) {
        merged.extend(arr.borrow().elements.iter().copied());
    }
    merged.extend_from_slice(args);
    merged
}

fn make_type_error_throw(ctx: &Context, message: &str) -> Value {
    let error = ctx.new_object();
    let atom_name = ctx.intern("name");
    let atom_message = ctx.intern("message");
    let atom_ctor = ctx.intern("constructor");
    error
        .borrow_mut()
        .set(atom_name, Value::from_str_with_context(ctx, "TypeError"));
    error.borrow_mut().set(
        atom_message,
        Value::from_owned_string_with_context(ctx, message.to_string()),
    );
    if let Some(ctor) = ctx.globals.borrow().get(&ctx.intern("TypeError")).copied() {
        error.borrow_mut().set(atom_ctor, ctor);
    }

    let thrown = ctx.new_object();
    let atom_throw = ctx.intern("__vm_throw");
    thrown.borrow_mut().set(atom_throw, Value::from(error));
    Value::from(thrown)
}

impl Ternary for Value {
    #[inline(always)]
    fn ternary(cond: &Self, a: &Self, b: &Self) -> Self {
        if cond.is_truthy() { *a } else { *b }
    }
}

impl ValueOps for Value {}

#[inline(always)]
fn is_positive_zero(value: f64) -> bool {
    value == 0.0 && value.is_sign_positive()
}

#[inline(always)]
fn is_negative_zero(value: f64) -> bool {
    value == 0.0 && value.is_sign_negative()
}

#[inline(always)]
fn is_integral_number(value: f64) -> bool {
    value.is_finite() && value.fract() == 0.0
}

#[inline(always)]
fn is_odd_integer(value: f64) -> bool {
    is_integral_number(value) && (value % 2.0).abs() == 1.0
}

#[inline(always)]
fn ecma_number_pow(base: f64, exponent: f64) -> f64 {
    if exponent.is_nan() {
        return f64::NAN;
    }
    if exponent == 0.0 {
        return 1.0;
    }
    if base.is_nan() {
        return f64::NAN;
    }

    let abs_base = base.abs();
    if exponent.is_infinite() {
        if abs_base == 1.0 {
            return f64::NAN;
        }
        return if exponent.is_sign_positive() {
            if abs_base > 1.0 { f64::INFINITY } else { 0.0 }
        } else if abs_base > 1.0 {
            0.0
        } else {
            f64::INFINITY
        };
    }

    if base.is_infinite() {
        if base.is_sign_positive() {
            return if exponent > 0.0 { f64::INFINITY } else { 0.0 };
        }
        return if exponent > 0.0 {
            if is_odd_integer(exponent) {
                f64::NEG_INFINITY
            } else {
                f64::INFINITY
            }
        } else if is_odd_integer(exponent) {
            -0.0
        } else {
            0.0
        };
    }

    if is_positive_zero(base) {
        return if exponent > 0.0 { 0.0 } else { f64::INFINITY };
    }
    if is_negative_zero(base) {
        return if exponent > 0.0 {
            if is_odd_integer(exponent) { -0.0 } else { 0.0 }
        } else if is_odd_integer(exponent) {
            f64::NEG_INFINITY
        } else {
            f64::INFINITY
        };
    }

    if base < 0.0 && !is_integral_number(exponent) {
        return f64::NAN;
    }

    base.powf(exponent)
}

// ============================================================================
// EXPLICIT CONTEXT METHODS (RECOMMENDED FOR NEW CODE)
// ============================================================================
//
// These methods take &Context explicitly instead of calling Context::current().
// This makes them safe for multi-threaded and async contexts.
//
// Migration path:
// 1. New code should use these explicit context versions
// 2. Gradually migrate old code to pass context down the call stack
// 3. Eventually, the trait implementations can be removed or delegated
//
// Example:
//   let result = Value::add_with_context(&ctx, &a, &b);  // GOOD (explicit)
//   let result = a.add(&b);                               // OK (calls Context::current())

impl Value {
    // === Arithmetic (only add actually needs context) ===
    #[inline(always)]
    pub fn add_with_context(ctx: &Context, lhs: &Value, rhs: &Value) -> Value {
        let a = lhs.0;
        let b = rhs.0;

        // FAST FLOAT PATH (OR-Tag Dispatch)
        if ((a | b) & QNAN) != QNAN {
            return Value::f64(f64::from_bits(a) + f64::from_bits(b));
        }

        // FAST INT PATH
        if Self::is_tagged_int_bits(a) && Self::is_tagged_int_bits(b) {
            let a_int = Self::int_payload(a);
            let b_int = Self::int_payload(b);
            if let Some(result) = a_int.checked_add(b_int) {
                return Value::i32(result);
            }
            return Value::f64((a_int as f64) + (b_int as f64));
        }

        // STRING CONCATENATION (JS semantics)
        let lhs_is_string = lhs.as_atom().is_some() || lhs.heap_kind() == Some(HeapKind::String);
        let rhs_is_string = rhs.as_atom().is_some() || rhs.heap_kind() == Some(HeapKind::String);
        if lhs_is_string || rhs_is_string {
            let to_string = |value: &Value| {
                let s = value.to_string_with_context(ctx);
                if let Some(atom) = s.as_atom() {
                    ctx.resolve(atom).to_string()
                } else if let Some(i) = s.as_i32() {
                    i.to_string()
                } else if let Some(f) = s.as_f64() {
                    f.to_string()
                } else if s.is_null() {
                    "null".to_string()
                } else if s.is_undefined() {
                    "undefined".to_string()
                } else {
                    "[object Object]".to_string()
                }
            };
            let sa = to_string(lhs);
            let sb = to_string(rhs);
            return Value::atom(ctx.intern(&format!("{}{}", sa, sb)));
        }

        // SLOW PATH
        Value::f64(lhs.to_number_ecma() + rhs.to_number_ecma())
    }

    // All other arithmetic ops just delegate (no context needed)
    #[inline(always)]
    pub fn sub_with_context(_: &Context, lhs: &Value, rhs: &Value) -> Value {
        let a = lhs.0;
        let b = rhs.0;

        // FAST FLOAT PATH (OR-Tag Dispatch)
        if ((a | b) & QNAN) != QNAN {
            return Value::f64(f64::from_bits(a) - f64::from_bits(b));
        }

        // FAST INT PATH
        if Self::is_tagged_int_bits(a) && Self::is_tagged_int_bits(b) {
            let a_int = Self::int_payload(a);
            let b_int = Self::int_payload(b);
            if let Some(result) = a_int.checked_sub(b_int) {
                return Value::i32(result);
            }
            return Value::f64((a_int as f64) - (b_int as f64));
        }

        // SLOW PATH
        Value::f64(lhs.to_number_ecma() - rhs.to_number_ecma())
    }
    #[inline(always)]
    pub fn mul_with_context(_: &Context, lhs: &Value, rhs: &Value) -> Value {
        let a = lhs.0;
        let b = rhs.0;

        // FAST FLOAT PATH (OR-Tag Dispatch)
        if ((a | b) & QNAN) != QNAN {
            return Value::f64(f64::from_bits(a) * f64::from_bits(b));
        }

        // FAST INT PATH
        if Self::is_tagged_int_bits(a) && Self::is_tagged_int_bits(b) {
            let a_int = Self::int_payload(a);
            let b_int = Self::int_payload(b);
            if let Some(result) = a_int.checked_mul(b_int) {
                return Value::i32(result);
            }
            return Value::f64((a_int as f64) * (b_int as f64));
        }

        // SLOW PATH
        Value::f64(lhs.to_number_ecma() * rhs.to_number_ecma())
    }
    #[inline(always)]
    pub fn div_with_context(_: &Context, lhs: &Value, rhs: &Value) -> Value {
        let a = lhs.0;
        let b = rhs.0;

        // FAST FLOAT PATH (OR-Tag Dispatch)
        if ((a | b) & QNAN) != QNAN {
            return Value::f64(f64::from_bits(a) / f64::from_bits(b));
        }

        // FAST INT PATH
        if Self::is_tagged_int_bits(a) && Self::is_tagged_int_bits(b) {
            let a_int = Self::int_payload(a);
            let b_int = Self::int_payload(b);
            // Let IEEE 754 handle it: 0/0 = NaN, x/0 = ±Infinity
            return Value::f64((a_int as f64) / (b_int as f64));
        }

        // SLOW PATH
        Value::f64(lhs.to_number_ecma() / rhs.to_number_ecma())
    }
    #[inline(always)]
    pub fn rem_with_context(_: &Context, lhs: &Value, rhs: &Value) -> Value {
        let a = lhs.0;
        let b = rhs.0;

        // FAST FLOAT PATH (OR-Tag Dispatch)
        if ((a | b) & QNAN) != QNAN {
            return Value::f64(f64::from_bits(a) % f64::from_bits(b));
        }

        // FAST INT PATH
        if Self::is_tagged_int_bits(a) && Self::is_tagged_int_bits(b) {
            let a_int = Self::int_payload(a);
            let b_int = Self::int_payload(b);
            if b_int == 0 {
                return Value::f64(f64::NAN);
            }
            if let Some(result) = a_int.checked_rem(b_int) {
                return Value::i32(result);
            }
            return Value::f64((a_int as f64) % (b_int as f64));
        }

        // SLOW PATH
        Value::f64(lhs.to_number_ecma() % rhs.to_number_ecma())
    }
    #[inline(always)]
    pub fn pow_with_context(_: &Context, lhs: &Value, rhs: &Value) -> Value {
        Value::f64(ecma_number_pow(lhs.to_number_ecma(), rhs.to_number_ecma()))
    }

    // === Comparison (eq needs context for string→number) ===
    #[inline(always)]
    pub fn eq_with_context(ctx: &Context, lhs: &Value, rhs: &Value) -> Value {
        // Fast path: identical bits
        if lhs.0 == rhs.0 {
            return Value::bool(true);
        }

        if (lhs.is_html_dda() && (rhs.is_null() || rhs.is_undefined()))
            || (rhs.is_html_dda() && (lhs.is_null() || lhs.is_undefined()))
        {
            return Value::bool(true);
        }

        // null == undefined (ECMAScript spec)
        if (lhs.is_null() && rhs.is_undefined()) || (lhs.is_undefined() && rhs.is_null()) {
            return Value::bool(true);
        }

        // null/undefined are ONLY equal to each other (already handled above)
        if lhs.is_null() || lhs.is_undefined() || rhs.is_null() || rhs.is_undefined() {
            return Value::bool(false);
        }

        // Both strings → direct atom comparison
        if let (Some(a), Some(b)) = (lhs.as_atom(), rhs.as_atom()) {
            return Value::bool(ctx.resolve(a) == ctx.resolve(b));
        }

        // One side is string → ToNumber(string) with proper whitespace trimming
        if let Some(a) = lhs.as_atom() {
            let s = ctx.resolve(a);
            let trimmed = s.trim();
            let num = if trimmed.is_empty() {
                0.0
            } else if trimmed == "Infinity" {
                f64::INFINITY
            } else if trimmed == "-Infinity" {
                f64::NEG_INFINITY
            } else {
                trimmed.parse::<f64>().unwrap_or(f64::NAN)
            };
            return Value::bool(num == rhs.to_number_ecma());
        }

        if let Some(b) = rhs.as_atom() {
            let s = ctx.resolve(b);
            let trimmed = s.trim();
            let num = if trimmed.is_empty() {
                0.0
            } else if trimmed == "Infinity" {
                f64::INFINITY
            } else if trimmed == "-Infinity" {
                f64::NEG_INFINITY
            } else {
                trimmed.parse::<f64>().unwrap_or(f64::NAN)
            };
            return Value::bool(lhs.to_number_ecma() == num);
        }

        // Everything else: numeric coercion
        Value::bool(lhs.to_number_ecma() == rhs.to_number_ecma())
    }

    #[inline(always)]
    pub fn strict_eq_with_context(_: &Context, lhs: &Value, rhs: &Value) -> Value {
        if lhs.0 == rhs.0 {
            return Value::bool(true);
        }

        let lhs_num = lhs.as_i32().map(|v| v as f64).or_else(|| lhs.as_f64());
        let rhs_num = rhs.as_i32().map(|v| v as f64).or_else(|| rhs.as_f64());
        if lhs_num.is_some() || rhs_num.is_some() {
            if let (Some(a), Some(b)) = (lhs_num, rhs_num) {
                if a.is_nan() || b.is_nan() {
                    return Value::bool(false);
                }
                return Value::bool(a == b);
            }
            return Value::bool(false);
        }

        if let (Some(a), Some(b)) = (lhs.as_bool(), rhs.as_bool()) {
            return Value::bool(a == b);
        }

        if lhs.is_null() || rhs.is_null() || lhs.is_undefined() || rhs.is_undefined() {
            return Value::bool(
                (lhs.is_null() && rhs.is_null()) || (lhs.is_undefined() && rhs.is_undefined()),
            );
        }

        let string_atom = |value: &Value| -> Option<Atom> {
            if let Some(atom) = value.as_atom() {
                return Some(atom);
            }
            if value.heap_kind() == Some(HeapKind::String)
                && let Ok(s) = Gc::<QString>::try_from(*value)
            {
                return Some(s.borrow().atom);
            }
            None
        };
        let lhs_atom = string_atom(lhs);
        let rhs_atom = string_atom(rhs);
        if lhs_atom.is_some() || rhs_atom.is_some() {
            return Value::bool(lhs_atom == rhs_atom);
        }

        Value::bool(false)
    }

    #[inline(always)]
    pub fn gt_with_context(_: &Context, lhs: &Value, rhs: &Value) -> Value {
        let a = lhs.to_number_ecma();
        let b = rhs.to_number_ecma();
        Value::bool(a > b)
    }

    #[inline(always)]
    pub fn lt_with_context(_: &Context, lhs: &Value, rhs: &Value) -> Value {
        let a = lhs.to_number_ecma();
        let b = rhs.to_number_ecma();
        Value::bool(a < b)
    }

    #[inline(always)]
    pub fn ge_with_context(_: &Context, lhs: &Value, rhs: &Value) -> Value {
        let a = lhs.to_number_ecma();
        let b = rhs.to_number_ecma();
        Value::bool(a >= b)
    }

    #[inline(always)]
    pub fn le_with_context(_: &Context, lhs: &Value, rhs: &Value) -> Value {
        let a = lhs.to_number_ecma();
        let b = rhs.to_number_ecma();
        Value::bool(a <= b)
    }

    /// Convert to string with explicit context parameter
    #[inline]
    pub fn to_string_with_context(&self, ctx: &Context) -> Value {
        if self.heap_kind() == Some(HeapKind::String)
            && let Ok(s) = Gc::<QString>::try_from(*self)
        {
            return Value::atom(s.borrow().atom);
        }
        let type_str = match () {
            _ if self.is_null() => "null",
            _ if self.is_undefined() => "undefined",
            _ if self.as_bool().is_some() => {
                if self.as_bool().unwrap() {
                    "true"
                } else {
                    "false"
                }
            }
            _ if self.as_i32().is_some() => {
                let val = self.as_i32().unwrap();
                if let Some(cached) = Self::get_small_int_string(val) {
                    return Value::atom(ctx.intern(cached));
                }
                return Value::atom(ctx.intern(&val.to_string()));
            }
            _ if self.as_f64().is_some() => {
                let val = self.as_f64().unwrap();
                let s = if val.is_nan() {
                    "NaN".to_string()
                } else if val.is_infinite() {
                    if val.is_sign_positive() {
                        "Infinity".to_string()
                    } else {
                        "-Infinity".to_string()
                    }
                } else if val == 0.0 {
                    "0".to_string() // -0.0 must stringify as "0" in JS
                } else {
                    val.to_string()
                };
                return Value::atom(ctx.intern(&s));
            }
            _ if self.as_atom().is_some() => {
                return *self;
            }
            _ => "[object Object]",
        };
        Value::atom(ctx.intern(type_str))
    }

    /// Get typeof string with explicit context parameter
    #[inline(always)]
    pub fn typeof_with_context(&self, ctx: &Context) -> Value {
        if self.is_html_dda() {
            return Value::atom(ctx.intern("undefined"));
        }
        let bits = self.0;

        let type_str = if (bits & QNAN) != QNAN {
            "number"
        } else {
            match bits & TAG_MASK {
                TAG_BOOL => "boolean",
                TAG_INT => "number",
                TAG_NULL => "object",
                TAG_UNDEF | TAG_EMPTY => "undefined",
                TAG_ATOM => "string",
                TAG_HEAP => {
                    if let Some(kind) = self.heap_kind() {
                        match kind {
                            HeapKind::Function
                            | HeapKind::Closure
                            | HeapKind::NativeFunction
                            | HeapKind::NativeClosure => "function",
                            HeapKind::Object => {
                                if let Ok(obj) = Gc::<QObject>::try_from(*self) {
                                    let atom_bound = ctx.intern("__vm_bound_target");
                                    if obj
                                        .borrow()
                                        .get(atom_bound)
                                        .unwrap_or(Value::undefined())
                                        .is_truthy()
                                    {
                                        return Value::atom(ctx.intern("function"));
                                    }
                                    let atom_native = ctx.intern("__vm_native_fn");
                                    if obj
                                        .borrow()
                                        .get(atom_native)
                                        .unwrap_or(Value::undefined())
                                        .heap_kind()
                                        == Some(HeapKind::NativeFunction)
                                    {
                                        return Value::atom(ctx.intern("function"));
                                    }
                                    let atom_id = ctx.intern("__vm_bytecode_id");
                                    if obj
                                        .borrow()
                                        .get(atom_id)
                                        .unwrap_or(Value::undefined())
                                        .as_i32()
                                        .is_some()
                                    {
                                        return Value::atom(ctx.intern("function"));
                                    }
                                }
                                "object"
                            }
                            _ => "object",
                        }
                    } else {
                        "object"
                    }
                }
                _ => "object",
            }
        };

        Value::atom(ctx.intern(type_str))
    }

    /// Create Value from String slice with explicit context parameter
    #[inline]
    pub fn from_str_with_context(ctx: &Context, s: &str) -> Value {
        let string = ctx.new_string(s);
        Value::from(&string)
    }

    /// Create Value from String with explicit context parameter
    #[inline]
    pub fn from_string_with_context(ctx: &Context, s: String) -> Value {
        let string = ctx.new_string(&s);
        Value::from(&string)
    }

    /// Create Value from owned String with explicit context parameter
    #[inline]
    pub fn from_owned_string_with_context(ctx: &Context, s: String) -> Value {
        let string = ctx.new_string(&s);
        Value::from(&string)
    }
}

pub type JSValue = Value;

#[inline(always)]
pub fn make_int32(value: i32) -> JSValue {
    Value::i32(value)
}

#[inline(always)]
pub fn make_number(value: f64) -> JSValue {
    Value::f64(value)
}

#[inline(always)]
pub fn make_bool(value: bool) -> JSValue {
    Value::bool(value)
}

#[inline(always)]
pub fn make_true() -> JSValue {
    Value::bool(true)
}

#[inline(always)]
pub fn make_false() -> JSValue {
    Value::bool(false)
}

#[inline(always)]
pub fn make_null() -> JSValue {
    Value::null()
}

#[inline(always)]
pub fn make_undefined() -> JSValue {
    Value::undefined()
}

#[inline(always)]
pub fn make_object(ptr: *mut crate::vm::JSObject) -> JSValue {
    Value::heap(ptr.cast::<GCHeader>())
}

#[inline(always)]
pub fn make_string(ptr: *mut crate::vm::JSString) -> JSValue {
    Value::heap(ptr.cast::<GCHeader>())
}

#[inline(always)]
pub fn to_f64(value: JSValue) -> Option<f64> {
    value.as_i32().map(f64::from).or_else(|| value.as_f64())
}

#[inline(always)]
pub fn to_i32(value: JSValue) -> Option<i32> {
    value
        .as_i32()
        .or_else(|| value.as_f64().map(|number| number as i32))
}

#[inline(always)]
pub fn bool_from_value(value: JSValue) -> Option<bool> {
    value.as_bool()
}

#[inline(always)]
pub fn is_number(value: JSValue) -> bool {
    value.as_i32().is_some() || value.as_f64().is_some()
}

#[inline(always)]
pub fn is_null(value: JSValue) -> bool {
    value.is_null()
}

#[inline(always)]
pub fn is_undefined(value: JSValue) -> bool {
    value.is_undefined()
}

#[inline(always)]
pub fn is_truthy(value: JSValue) -> bool {
    value.is_truthy()
}

#[inline(always)]
pub fn object_from_value(value: JSValue) -> Option<*mut crate::vm::JSObject> {
    let ptr = value.as_heap_ptr()?;
    (unsafe { (*ptr).obj_type } == crate::gc::ObjType::Object)
        .then_some(ptr as *mut crate::vm::JSObject)
}

#[inline(always)]
pub fn string_from_value(value: JSValue) -> Option<*mut crate::vm::JSString> {
    let ptr = value.as_heap_ptr()?;
    (unsafe { (*ptr).obj_type } == crate::gc::ObjType::String)
        .then_some(ptr as *mut crate::vm::JSString)
}

#[inline(always)]
pub fn is_object(value: JSValue) -> bool {
    object_from_value(value).is_some()
}

#[inline(always)]
pub fn is_string(value: JSValue) -> bool {
    value.as_atom().is_some() || string_from_value(value).is_some()
}
