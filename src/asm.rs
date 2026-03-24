//! Assembly code generator for QJL bytecode
//!
//! This module provides functionality to disassemble bytecode into human-readable
//! assembly code based on the BYTECODE_SPEC_v2.0.md specification.

use crate::vm::{Opcode, VM};

/// Instruction format for disassembly
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Format {
    /// ABC: three 8-bit register operands
    ABC,
    /// ABx: A + 16-bit unsigned immediate (B and C combined)
    ABx,
    /// AsBx: A + 16-bit signed offset (B and C combined)
    AsBx,
    /// A: single register operand
    A,
    /// BC: two registers, result in accumulator
    BC,
}

/// Assembly instruction with decoded operands
#[derive(Debug, Clone)]
pub struct AsmInstruction {
    /// Program counter (byte offset)
    pub pc: usize,
    /// Raw instruction word
    pub raw: u32,
    /// Opcode
    pub opcode: Opcode,
    /// Format
    pub format: Format,
    /// Decoded A operand
    pub a: u8,
    /// Decoded B operand
    pub b: u8,
    /// Decoded C operand
    pub c: u8,
    /// Decoded Bx/ABx operand (unsigned 16-bit)
    pub bx: u16,
    /// Decoded sBx/AsBx operand (signed 16-bit)
    pub sbx: i16,
}

impl AsmInstruction {
    /// Decode a raw instruction word
    pub fn decode(pc: usize, raw: u32) -> Self {
        let opcode = Opcode::from((raw & 0xFF) as u8);
        let a = ((raw >> 8) & 0xFF) as u8;
        let b = ((raw >> 16) & 0xFF) as u8;
        let c = ((raw >> 24) & 0xFF) as u8;
        let bx = ((raw >> 16) & 0xFFFF) as u16;
        let sbx = bx as i16;

        // Determine format based on opcode
        let format = match opcode {
            Opcode::Mov
            | Opcode::GetPropIc
            | Opcode::SetPropIc
            | Opcode::AddI
            | Opcode::SubI
            | Opcode::MulI
            | Opcode::DivI
            | Opcode::ModI
            | Opcode::GetIdxFast
            | Opcode::SetIdxFast
            | Opcode::GetProp
            | Opcode::SetProp
            | Opcode::GetIdxIc
            | Opcode::SetIdxIc
            | Opcode::GetLengthIc
            | Opcode::GetSuper
            | Opcode::SetSuper
            | Opcode::DeleteProp
            | Opcode::HasProp
            | Opcode::JmpEq
            | Opcode::JmpNeq
            | Opcode::JmpLt
            | Opcode::JmpLte
            | Opcode::JmpLteFalse
            | Opcode::GetPropIcCall
            | Opcode::Call1SubI
            | Opcode::AddMov
            | Opcode::GetPropIcMov
            | Opcode::GetPropAddImmSetPropIc
            | Opcode::GetPropChainAcc
            | Opcode::NewObjInitProp
            | Opcode::ProfileHotCall
            | Opcode::AddI32
            | Opcode::AddF64
            | Opcode::SubI32
            | Opcode::SubF64
            | Opcode::MulI32
            | Opcode::MulF64 => Format::ABC,

            Opcode::LoadK
            | Opcode::LoadGlobalIc
            | Opcode::SetGlobalIc
            | Opcode::NewFunc
            | Opcode::GetGlobal
            | Opcode::SetGlobal
            | Opcode::ResolveScope
            | Opcode::LoadName
            | Opcode::StoreName
            | Opcode::TypeofName
            | Opcode::LoadKAddAcc
            | Opcode::LoadKMulAcc
            | Opcode::LoadKSubAcc => Format::ABx,

            Opcode::Jmp
            | Opcode::LoadI
            | Opcode::JmpTrue
            | Opcode::JmpFalse
            | Opcode::LoopIncJmp
            | Opcode::Try
            | Opcode::IncJmpFalseLoop
            | Opcode::EqJmpTrue
            | Opcode::LtJmp
            | Opcode::EqJmpFalse
            | Opcode::IncAccJmp
            | Opcode::TestJmpTrue
            | Opcode::LteJmpLoop => Format::AsBx,

            Opcode::AddAccImm8
            | Opcode::IncAcc
            | Opcode::LoadThis
            | Opcode::Load0
            | Opcode::Load1
            | Opcode::AddAcc
            | Opcode::SubAcc
            | Opcode::MulAcc
            | Opcode::DivAcc
            | Opcode::LoadNull
            | Opcode::LoadTrue
            | Opcode::LoadFalse
            | Opcode::SubAccImm8
            | Opcode::MulAccImm8
            | Opcode::DivAccImm8
            | Opcode::AddStrAcc
            | Opcode::BitNot
            | Opcode::Neg
            | Opcode::Inc
            | Opcode::Dec
            | Opcode::ToPrimitive
            | Opcode::ArrayPushAcc
            | Opcode::NewObj
            | Opcode::NewArr
            | Opcode::NewClass
            | Opcode::GetUpval
            | Opcode::SetUpval
            | Opcode::GetScope
            | Opcode::SetScope
            | Opcode::ForIn
            | Opcode::IteratorNext
            | Opcode::Spread
            | Opcode::Destructure
            | Opcode::CreateEnv
            | Opcode::LoadClosure
            | Opcode::NewThis
            | Opcode::LoopHint
            | Opcode::Ret
            | Opcode::RetU
            | Opcode::RetReg
            | Opcode::Leave
            | Opcode::Yield
            | Opcode::Await
            | Opcode::EndTry
            | Opcode::Finally
            | Opcode::ProfileRet
            | Opcode::IcMiss
            | Opcode::OsrEntry
            | Opcode::ProfileHotLoop
            | Opcode::OsrExit
            | Opcode::JitHint
            | Opcode::SafetyCheck
            | Opcode::AssertValue
            | Opcode::AssertOk
            | Opcode::AssertFail
            | Opcode::AssertThrows
            | Opcode::AssertDoesNotThrow
            | Opcode::AssertRejects
            | Opcode::AssertDoesNotReject => Format::A,

            Opcode::Add
            | Opcode::Eq
            | Opcode::Lt
            | Opcode::Lte
            | Opcode::AddStr
            | Opcode::GetPropAcc
            | Opcode::SetPropAcc
            | Opcode::StrictEq
            | Opcode::StrictNeq
            | Opcode::GetPropAccCall
            | Opcode::GetLengthIcCall
            | Opcode::AssertEqual
            | Opcode::AssertNotEqual
            | Opcode::AssertDeepEqual
            | Opcode::AssertNotDeepEqual
            | Opcode::AssertStrictEqual
            | Opcode::AssertNotStrictEqual
            | Opcode::AssertDeepStrictEqual
            | Opcode::AssertNotDeepStrictEqual
            | Opcode::BitAnd
            | Opcode::BitOr
            | Opcode::BitXor
            | Opcode::Shl
            | Opcode::Shr
            | Opcode::Ushr
            | Opcode::Pow
            | Opcode::LogicalAnd
            | Opcode::LogicalOr
            | Opcode::NullishCoalesce
            | Opcode::In
            | Opcode::Instanceof => Format::BC,

            Opcode::Call
            | Opcode::TailCall
            | Opcode::Construct
            | Opcode::CallIc
            | Opcode::CallIcSuper
            | Opcode::LoadThisCall
            | Opcode::LoadArgCall => Format::ABC, // Special: A B format

            Opcode::Typeof
            | Opcode::ToNum
            | Opcode::ToStr
            | Opcode::IsUndef
            | Opcode::IsNull
            | Opcode::LoadArg
            | Opcode::LoadAcc
            | Opcode::Keys
            | Opcode::Switch
            | Opcode::Enter
            | Opcode::Throw
            | Opcode::Catch
            | Opcode::CallVar
            | Opcode::CallIcVar
            | Opcode::ProfileType
            | Opcode::ProfileCall
            | Opcode::CheckType
            | Opcode::CheckStruct
            | Opcode::CheckIc
            | Opcode::IcInit
            | Opcode::IcUpdate
            | Opcode::AddAccImm8Mov
            | Opcode::AddStrAccMov
            | Opcode::MulAccMov => Format::ABC,

            Opcode::Reserved(_) => Format::ABC,
        };

        Self {
            pc,
            raw,
            opcode,
            format,
            a,
            b,
            c,
            bx,
            sbx,
        }
    }

    /// Format instruction as human-readable assembly
    pub fn to_asm(&self, constants: &[crate::js_value::JSValue]) -> String {
        let pc_byte_offset = self.pc * 4;
        let opcode_str = self.opcode_to_mnemonic();

        match self.format {
            Format::ABC => {
                match self.opcode {
                    Opcode::Call
                    | Opcode::TailCall
                    | Opcode::Construct
                    | Opcode::CallIc
                    | Opcode::CallIcSuper
                    | Opcode::LoadThisCall
                    | Opcode::LoadArgCall => {
                        // A B format
                        format!(
                            "{:04X}: {} r{}, {}",
                            pc_byte_offset, opcode_str, self.a, self.b
                        )
                    }
                    Opcode::Call1SubI => {
                        format!(
                            "{:04X}: {} r{}, r{}, {}",
                            pc_byte_offset, opcode_str, self.a, self.b, self.c as i8
                        )
                    }
                    Opcode::JmpEq
                    | Opcode::JmpNeq
                    | Opcode::JmpLt
                    | Opcode::JmpLte
                    | Opcode::JmpLteFalse => {
                        let target_pc = (self.pc as isize + self.c as i8 as isize + 1) as usize;
                        let target_byte_offset = target_pc.saturating_mul(4);
                        format!(
                            "{:04X}: {} r{}, r{}, -> {:04X}",
                            pc_byte_offset, opcode_str, self.a, self.b, target_byte_offset
                        )
                    }
                    _ => {
                        // Standard ABC format
                        format!(
                            "{:04X}: {} r{}, r{}, r{}",
                            pc_byte_offset, opcode_str, self.a, self.b, self.c
                        )
                    }
                }
            }
            Format::ABx => {
                match self.opcode {
                    Opcode::LoadK | Opcode::NewFunc => {
                        // Load constant
                        let const_idx = self.bx as usize;
                        let const_val = if const_idx < constants.len() {
                            format!("const[{}]", const_idx)
                        } else {
                            "const[out of bounds]".to_string()
                        };
                        format!(
                            "{:04X}: {} r{}, {}",
                            pc_byte_offset, opcode_str, self.a, const_val
                        )
                    }
                    Opcode::LoadGlobalIc
                    | Opcode::SetGlobalIc
                    | Opcode::GetGlobal
                    | Opcode::SetGlobal => {
                        format!(
                            "{:04X}: {} r{}, global[{}]",
                            pc_byte_offset, opcode_str, self.a, self.bx
                        )
                    }
                    Opcode::ResolveScope
                    | Opcode::LoadName
                    | Opcode::StoreName
                    | Opcode::TypeofName => {
                        format!(
                            "{:04X}: {} r{}, identifier[{}]",
                            pc_byte_offset, opcode_str, self.a, self.bx
                        )
                    }
                    Opcode::LoadKAddAcc | Opcode::LoadKMulAcc | Opcode::LoadKSubAcc => {
                        let const_idx = self.bx as usize;
                        let const_val = if const_idx < constants.len() {
                            format!("const[{}]", const_idx)
                        } else {
                            "const[out of bounds]".to_string()
                        };
                        format!("{:04X}: {} {}", pc_byte_offset, opcode_str, const_val)
                    }
                    _ => {
                        format!(
                            "{:04X}: {} r{}, {}",
                            pc_byte_offset, opcode_str, self.a, self.bx
                        )
                    }
                }
            }
            Format::AsBx => {
                // Handle overflow in multiplication
                let target_pc = (self.pc as isize + self.sbx as isize + 1) as usize;
                let target_byte_offset = target_pc.saturating_mul(4);
                match self.opcode {
                    Opcode::Jmp => {
                        format!(
                            "{:04X}: {} -> {:04X}",
                            pc_byte_offset, opcode_str, target_byte_offset
                        )
                    }
                    Opcode::LoadI => {
                        format!(
                            "{:04X}: {} r{}, {}",
                            pc_byte_offset, opcode_str, self.a, self.sbx
                        )
                    }
                    Opcode::JmpTrue
                    | Opcode::JmpFalse
                    | Opcode::LoopIncJmp
                    | Opcode::Try
                    | Opcode::IncJmpFalseLoop
                    | Opcode::EqJmpTrue
                    | Opcode::LtJmp
                    | Opcode::EqJmpFalse
                    | Opcode::IncAccJmp
                    | Opcode::TestJmpTrue
                    | Opcode::LteJmpLoop => {
                        format!(
                            "{:04X}: {} r{}, -> {:04X}",
                            pc_byte_offset, opcode_str, self.a, target_byte_offset
                        )
                    }
                    _ => {
                        format!(
                            "{:04X}: {} r{}, {}",
                            pc_byte_offset, opcode_str, self.a, self.sbx
                        )
                    }
                }
            }
            Format::A => match self.opcode {
                Opcode::AddAccImm8
                | Opcode::SubAccImm8
                | Opcode::MulAccImm8
                | Opcode::DivAccImm8 => {
                    format!("{:04X}: {} {}", pc_byte_offset, opcode_str, self.b as i8)
                }
                Opcode::AddAcc
                | Opcode::SubAcc
                | Opcode::MulAcc
                | Opcode::DivAcc
                | Opcode::Neg
                | Opcode::Inc
                | Opcode::Dec
                | Opcode::ToPrimitive
                | Opcode::AddStrAcc => {
                    format!("{:04X}: {} r{}", pc_byte_offset, opcode_str, self.b)
                }
                Opcode::ArrayPushAcc
                | Opcode::GetUpval
                | Opcode::SetUpval
                | Opcode::GetScope
                | Opcode::SetScope
                | Opcode::ForIn
                | Opcode::IteratorNext
                | Opcode::Spread
                | Opcode::Destructure
                | Opcode::CreateEnv
                | Opcode::LoadClosure
                | Opcode::NewThis => {
                    format!("{:04X}: {} r{}", pc_byte_offset, opcode_str, self.a)
                }
                Opcode::IncAcc
                | Opcode::LoadThis
                | Opcode::Load0
                | Opcode::Load1
                | Opcode::LoadNull
                | Opcode::LoadTrue
                | Opcode::LoadFalse
                | Opcode::NewObj
                | Opcode::NewArr
                | Opcode::NewClass
                | Opcode::LoopHint
                | Opcode::Ret
                | Opcode::RetU
                | Opcode::Leave
                | Opcode::Yield
                | Opcode::Await
                | Opcode::EndTry
                | Opcode::Finally
                | Opcode::ProfileRet
                | Opcode::IcMiss
                | Opcode::OsrEntry
                | Opcode::ProfileHotLoop
                | Opcode::OsrExit
                | Opcode::JitHint
                | Opcode::SafetyCheck
                | Opcode::AssertValue
                | Opcode::AssertOk
                | Opcode::AssertFail
                | Opcode::AssertThrows
                | Opcode::AssertDoesNotThrow
                | Opcode::AssertRejects
                | Opcode::AssertDoesNotReject => {
                    format!("{:04X}: {}", pc_byte_offset, opcode_str)
                }
                _ => {
                    format!("{:04X}: {} r{}", pc_byte_offset, opcode_str, self.a)
                }
            },
            Format::BC => match self.opcode {
                Opcode::GetPropAccCall | Opcode::GetLengthIcCall => {
                    format!(
                        "{:04X}: {} r{}, r{}",
                        pc_byte_offset, opcode_str, self.b, self.c
                    )
                }
                _ => {
                    format!(
                        "{:04X}: {} r{}, r{}",
                        pc_byte_offset, opcode_str, self.b, self.c
                    )
                }
            },
        }
    }

    /// Convert opcode to mnemonic string (clean format without underscores)
    fn opcode_to_clean_mnemonic(&self) -> &'static str {
        match self.opcode {
            Opcode::Mov => "mov",
            Opcode::LoadK => "loadk",
            Opcode::Add => "add",
            Opcode::GetPropIc => "getpropic",
            Opcode::Call => "call",
            Opcode::Jmp => "jmp",
            Opcode::LoadI => "loadi",
            Opcode::JmpTrue => "jmptrue",
            Opcode::JmpFalse => "jmpfalse",
            Opcode::SetPropIc => "setpropic",
            Opcode::AddAccImm8 => "addaccimm8",
            Opcode::IncAcc => "incacc",
            Opcode::LoadThis => "loadthis",
            Opcode::Load0 => "load0",
            Opcode::Load1 => "load1",
            Opcode::Eq => "eq",
            Opcode::Lt => "lt",
            Opcode::Lte => "lte",
            Opcode::AddAcc => "addacc",
            Opcode::SubAcc => "subacc",
            Opcode::MulAcc => "mulacc",
            Opcode::DivAcc => "divacc",
            Opcode::LoadNull => "loadnull",
            Opcode::LoadTrue => "loadtrue",
            Opcode::LoadFalse => "loadfalse",
            Opcode::LoadGlobalIc => "loadglobalic",
            Opcode::SetGlobalIc => "setglobalic",
            Opcode::Typeof => "typeof",
            Opcode::ToNum => "tonum",
            Opcode::ToStr => "tostr",
            Opcode::IsUndef => "isundef",
            Opcode::IsNull => "isnull",
            Opcode::SubAccImm8 => "subaccimm8",
            Opcode::MulAccImm8 => "mulaccimm8",
            Opcode::DivAccImm8 => "divaccimm8",
            Opcode::AddStrAcc => "addstracc",
            Opcode::AddI => "addi",
            Opcode::SubI => "subi",
            Opcode::MulI => "muli",
            Opcode::DivI => "divi",
            Opcode::ModI => "modi",
            Opcode::Neg => "neg",
            Opcode::Inc => "inc",
            Opcode::Dec => "dec",
            Opcode::AddStr => "addstr",
            Opcode::ToPrimitive => "toprimitive",
            Opcode::GetPropAcc => "getpropacc",
            Opcode::SetPropAcc => "setpropacc",
            Opcode::GetIdxFast => "getidxfast",
            Opcode::SetIdxFast => "setidxfast",
            Opcode::LoadArg => "loadarg",
            Opcode::LoadAcc => "loadacc",
            Opcode::StrictEq => "stricteq",
            Opcode::StrictNeq => "strictneq",
            Opcode::BitAnd => "bitand",
            Opcode::BitOr => "bitor",
            Opcode::BitXor => "bitxor",
            Opcode::BitNot => "bitnot",
            Opcode::Shl => "shl",
            Opcode::Shr => "shr",
            Opcode::Ushr => "ushr",
            Opcode::Pow => "pow",
            Opcode::LogicalAnd => "logicaland",
            Opcode::LogicalOr => "logicalor",
            Opcode::NullishCoalesce => "nullishcoalesce",
            Opcode::In => "in",
            Opcode::Instanceof => "instanceof",
            Opcode::GetLengthIc => "getlengthic",
            Opcode::ArrayPushAcc => "arraypushacc",
            Opcode::NewObj => "newobj",
            Opcode::NewArr => "newarr",
            Opcode::NewFunc => "newfunc",
            Opcode::NewClass => "newclass",
            Opcode::GetProp => "getprop",
            Opcode::SetProp => "setprop",
            Opcode::GetIdxIc => "getidxic",
            Opcode::SetIdxIc => "setidxic",
            Opcode::GetGlobal => "getglobal",
            Opcode::SetGlobal => "setglobal",
            Opcode::GetUpval => "getupval",
            Opcode::SetUpval => "setupval",
            Opcode::GetScope => "getscope",
            Opcode::SetScope => "setscope",
            Opcode::ResolveScope => "resolvescope",
            Opcode::GetSuper => "getsuper",
            Opcode::SetSuper => "setsuper",
            Opcode::DeleteProp => "deleteprop",
            Opcode::HasProp => "hasprop",
            Opcode::Keys => "keys",
            Opcode::ForIn => "forin",
            Opcode::IteratorNext => "iteratornext",
            Opcode::Spread => "spread",
            Opcode::Destructure => "destructure",
            Opcode::CreateEnv => "createenv",
            Opcode::LoadName => "loadname",
            Opcode::StoreName => "storename",
            Opcode::LoadClosure => "loadclosure",
            Opcode::NewThis => "newthis",
            Opcode::TypeofName => "typeofname",
            Opcode::JmpEq => "jmpeq",
            Opcode::JmpNeq => "jmpneq",
            Opcode::JmpLt => "jmplt",
            Opcode::JmpLte => "jmplte",
            Opcode::LoopIncJmp => "loopincjmp",
            Opcode::Switch => "switch",
            Opcode::LoopHint => "loophint",
            Opcode::Ret => "ret",
            Opcode::RetU => "retu",
            Opcode::RetReg => "retreg",
            Opcode::TailCall => "tailcall",
            Opcode::Construct => "construct",
            Opcode::CallVar => "callvar",
            Opcode::Enter => "enter",
            Opcode::Leave => "leave",
            Opcode::Yield => "yield",
            Opcode::Await => "await",
            Opcode::Throw => "throw",
            Opcode::Try => "try",
            Opcode::EndTry => "endtry",
            Opcode::Catch => "catch",
            Opcode::Finally => "finally",
            Opcode::CallIc => "callic",
            Opcode::CallIcVar => "callicvar",
            Opcode::ProfileType => "profiletype",
            Opcode::ProfileCall => "profilecall",
            Opcode::ProfileRet => "profileret",
            Opcode::CheckType => "checktype",
            Opcode::CheckStruct => "checkstruct",
            Opcode::CheckIc => "checkic",
            Opcode::IcInit => "icinit",
            Opcode::IcUpdate => "icupdate",
            Opcode::IcMiss => "icmiss",
            Opcode::OsrEntry => "osrentry",
            Opcode::ProfileHotLoop => "profilehotloop",
            Opcode::OsrExit => "osrexit",
            Opcode::JitHint => "jithint",
            Opcode::SafetyCheck => "safetycheck",
            Opcode::GetPropIcCall => "getpropiccall",
            Opcode::IncJmpFalseLoop => "incjmpfalseloop",
            Opcode::LoadKAddAcc => "loadkaddacc",
            Opcode::AddMov => "addmov",
            Opcode::EqJmpTrue => "eqjmptrue",
            Opcode::GetPropAccCall => "getpropacccall",
            Opcode::LoadKMulAcc => "loadkmulacc",
            Opcode::LtJmp => "ltjmp",
            Opcode::GetPropIcMov => "getpropicmov",
            Opcode::GetPropAddImmSetPropIc => "getpropaddimmsetpropic",
            Opcode::AddAccImm8Mov => "addaccimm8mov",
            Opcode::CallIcSuper => "callicsuper",
            Opcode::LoadThisCall => "loadthiscall",
            Opcode::EqJmpFalse => "eqjmpfalse",
            Opcode::LoadKSubAcc => "loadksubacc",
            Opcode::GetLengthIcCall => "getlengthiccall",
            Opcode::AddStrAccMov => "addstraccmov",
            Opcode::IncAccJmp => "incaccjmp",
            Opcode::GetPropChainAcc => "getpropchainacc",
            Opcode::TestJmpTrue => "testjmptrue",
            Opcode::LoadArgCall => "loadargcall",
            Opcode::MulAccMov => "mulaccmov",
            Opcode::LteJmpLoop => "ltejmploop",
            Opcode::NewObjInitProp => "newobjinitprop",
            Opcode::ProfileHotCall => "profilehotcall",
            Opcode::Call1SubI => "call1subi",
            Opcode::JmpLteFalse => "jmpltefalse",
            Opcode::AssertValue => "assertvalue",
            Opcode::AssertOk => "assertok",
            Opcode::AssertEqual => "assertequal",
            Opcode::AssertNotEqual => "assertnotequal",
            Opcode::AssertDeepEqual => "assertdeepequal",
            Opcode::AssertNotDeepEqual => "assertnotdeepequal",
            Opcode::AssertStrictEqual => "assertstrictequal",
            Opcode::AssertNotStrictEqual => "assertnotstrictequal",
            Opcode::AssertDeepStrictEqual => "assertdeepstrictequal",
            Opcode::AssertNotDeepStrictEqual => "assertnotdeepstrictequal",
            Opcode::AssertThrows => "assertthrows",
            Opcode::AssertDoesNotThrow => "assertdoesnotthrow",
            Opcode::AssertRejects => "assertrejects",
            Opcode::AssertDoesNotReject => "assertdoesnotreject",
            Opcode::AssertFail => "assertfail",
            Opcode::AddI32 => "addi32",
            Opcode::AddF64 => "addf64",
            Opcode::SubI32 => "subi32",
            Opcode::SubF64 => "subf64",
            Opcode::MulI32 => "muli32",
            Opcode::MulF64 => "mulf64",
            Opcode::Reserved(n) => {
                // Use a static string for reserved opcodes
                match n {
                    61..=63 => "reserved_61_63",
                    123..=127 => "reserved_123_127",
                    130..=159 => "reserved_130_159",
                    174..=199 => "reserved_174_199",
                    225..=239 => "reserved_225_239",
                    243..=255 => "reserved_243_255",
                    _ => "reserved",
                }
            }
        }
    }

    /// Convert opcode to mnemonic string (with underscores)
    pub fn opcode_to_mnemonic(&self) -> &'static str {
        match self.opcode {
            Opcode::Mov => "mov",
            Opcode::LoadK => "load_k",
            Opcode::Add => "add",
            Opcode::GetPropIc => "get_prop_ic",
            Opcode::Call => "call",
            Opcode::Jmp => "jmp",
            Opcode::LoadI => "load_i",
            Opcode::JmpTrue => "jmp_true",
            Opcode::JmpFalse => "jmp_false",
            Opcode::SetPropIc => "set_prop_ic",
            Opcode::AddAccImm8 => "add_acc_imm8",
            Opcode::IncAcc => "inc_acc",
            Opcode::LoadThis => "load_this",
            Opcode::Load0 => "load_0",
            Opcode::Load1 => "load_1",
            Opcode::Eq => "eq",
            Opcode::Lt => "lt",
            Opcode::Lte => "lte",
            Opcode::AddAcc => "add_acc",
            Opcode::SubAcc => "sub_acc",
            Opcode::MulAcc => "mul_acc",
            Opcode::DivAcc => "div_acc",
            Opcode::LoadNull => "load_null",
            Opcode::LoadTrue => "load_true",
            Opcode::LoadFalse => "load_false",
            Opcode::LoadGlobalIc => "load_global_ic",
            Opcode::SetGlobalIc => "set_global_ic",
            Opcode::Typeof => "typeof",
            Opcode::ToNum => "to_num",
            Opcode::ToStr => "to_str",
            Opcode::IsUndef => "is_undef",
            Opcode::IsNull => "is_null",
            Opcode::SubAccImm8 => "sub_acc_imm8",
            Opcode::MulAccImm8 => "mul_acc_imm8",
            Opcode::DivAccImm8 => "div_acc_imm8",
            Opcode::AddStrAcc => "add_str_acc",
            Opcode::AddI => "add_i",
            Opcode::SubI => "sub_i",
            Opcode::MulI => "mul_i",
            Opcode::DivI => "div_i",
            Opcode::ModI => "mod_i",
            Opcode::Neg => "neg",
            Opcode::Inc => "inc",
            Opcode::Dec => "dec",
            Opcode::AddStr => "add_str",
            Opcode::ToPrimitive => "to_primitive",
            Opcode::GetPropAcc => "get_prop_acc",
            Opcode::SetPropAcc => "set_prop_acc",
            Opcode::GetIdxFast => "get_idx_fast",
            Opcode::SetIdxFast => "set_idx_fast",
            Opcode::LoadArg => "load_arg",
            Opcode::LoadAcc => "load_acc",
            Opcode::StrictEq => "strict_eq",
            Opcode::StrictNeq => "strict_neq",
            Opcode::BitAnd => "bit_and",
            Opcode::BitOr => "bit_or",
            Opcode::BitXor => "bit_xor",
            Opcode::BitNot => "bit_not",
            Opcode::Shl => "shl",
            Opcode::Shr => "shr",
            Opcode::Ushr => "ushr",
            Opcode::Pow => "pow",
            Opcode::LogicalAnd => "logical_and",
            Opcode::LogicalOr => "logical_or",
            Opcode::NullishCoalesce => "nullish_coalesce",
            Opcode::In => "in",
            Opcode::Instanceof => "instanceof",
            Opcode::GetLengthIc => "get_length_ic",
            Opcode::ArrayPushAcc => "array_push_acc",
            Opcode::NewObj => "new_obj",
            Opcode::NewArr => "new_arr",
            Opcode::NewFunc => "new_func",
            Opcode::NewClass => "new_class",
            Opcode::GetProp => "get_prop",
            Opcode::SetProp => "set_prop",
            Opcode::GetIdxIc => "get_idx_ic",
            Opcode::SetIdxIc => "set_idx_ic",
            Opcode::GetGlobal => "get_global",
            Opcode::SetGlobal => "set_global",
            Opcode::GetUpval => "get_upval",
            Opcode::SetUpval => "setupval",
            Opcode::GetScope => "get_scope",
            Opcode::SetScope => "set_scope",
            Opcode::ResolveScope => "resolve_scope",
            Opcode::GetSuper => "get_super",
            Opcode::SetSuper => "set_super",
            Opcode::DeleteProp => "delete_prop",
            Opcode::HasProp => "has_prop",
            Opcode::Keys => "keys",
            Opcode::ForIn => "for_in",
            Opcode::IteratorNext => "iterator_next",
            Opcode::Spread => "spread",
            Opcode::Destructure => "destructure",
            Opcode::CreateEnv => "create_env",
            Opcode::LoadName => "load_name",
            Opcode::StoreName => "store_name",
            Opcode::LoadClosure => "load_closure",
            Opcode::NewThis => "new_this",
            Opcode::TypeofName => "typeof_name",
            Opcode::JmpEq => "jmp_eq",
            Opcode::JmpNeq => "jmp_neq",
            Opcode::JmpLt => "jmp_lt",
            Opcode::JmpLte => "jmp_lte",
            Opcode::LoopIncJmp => "loop_inc_jmp",
            Opcode::Switch => "switch",
            Opcode::LoopHint => "loop_hint",
            Opcode::Ret => "ret",
            Opcode::RetU => "ret_u",
            Opcode::RetReg => "ret_reg",
            Opcode::TailCall => "tail_call",
            Opcode::Construct => "construct",
            Opcode::CallVar => "call_var",
            Opcode::Enter => "enter",
            Opcode::Leave => "leave",
            Opcode::Yield => "yield",
            Opcode::Await => "await",
            Opcode::Throw => "throw",
            Opcode::Try => "try",
            Opcode::EndTry => "end_try",
            Opcode::Catch => "catch",
            Opcode::Finally => "finally",
            Opcode::CallIc => "call_ic",
            Opcode::CallIcVar => "call_ic_var",
            Opcode::ProfileType => "profile_type",
            Opcode::ProfileCall => "profile_call",
            Opcode::ProfileRet => "profile_ret",
            Opcode::CheckType => "check_type",
            Opcode::CheckStruct => "check_struct",
            Opcode::CheckIc => "check_ic",
            Opcode::IcInit => "ic_init",
            Opcode::IcUpdate => "ic_update",
            Opcode::IcMiss => "ic_miss",
            Opcode::OsrEntry => "osr_entry",
            Opcode::ProfileHotLoop => "profile_hot_loop",
            Opcode::OsrExit => "osr_exit",
            Opcode::JitHint => "jit_hint",
            Opcode::SafetyCheck => "safety_check",
            Opcode::GetPropIcCall => "get_prop_ic_call",
            Opcode::IncJmpFalseLoop => "inc_jmp_false_loop",
            Opcode::LoadKAddAcc => "load_k_add_acc",
            Opcode::AddMov => "add_mov",
            Opcode::EqJmpTrue => "eq_jmp_true",
            Opcode::GetPropAccCall => "get_prop_acc_call",
            Opcode::LoadKMulAcc => "load_k_mul_acc",
            Opcode::LtJmp => "lt_jmp",
            Opcode::GetPropIcMov => "get_prop_ic_mov",
            Opcode::GetPropAddImmSetPropIc => "get_prop_add_imm_set_prop_ic",
            Opcode::AddAccImm8Mov => "add_acc_imm8_mov",
            Opcode::CallIcSuper => "call_ic_super",
            Opcode::LoadThisCall => "load_this_call",
            Opcode::EqJmpFalse => "eq_jmp_false",
            Opcode::LoadKSubAcc => "load_k_sub_acc",
            Opcode::GetLengthIcCall => "get_length_ic_call",
            Opcode::AddStrAccMov => "add_str_acc_mov",
            Opcode::IncAccJmp => "inc_acc_jmp",
            Opcode::GetPropChainAcc => "get_prop_chain_acc",
            Opcode::TestJmpTrue => "test_jmp_true",
            Opcode::LoadArgCall => "load_arg_call",
            Opcode::MulAccMov => "mul_acc_mov",
            Opcode::LteJmpLoop => "lte_jmp_loop",
            Opcode::NewObjInitProp => "new_obj_init_prop",
            Opcode::ProfileHotCall => "profile_hot_call",
            Opcode::Call1SubI => "call1_subi",
            Opcode::JmpLteFalse => "jmp_lte_false",
            Opcode::AssertValue => "assert_value",
            Opcode::AssertOk => "assert_ok",
            Opcode::AssertEqual => "assert_equal",
            Opcode::AssertNotEqual => "assert_not_equal",
            Opcode::AssertDeepEqual => "assert_deep_equal",
            Opcode::AssertNotDeepEqual => "assert_not_deep_equal",
            Opcode::AssertStrictEqual => "assert_strict_equal",
            Opcode::AssertNotStrictEqual => "assert_not_strict_equal",
            Opcode::AssertDeepStrictEqual => "assert_deep_strict_equal",
            Opcode::AssertNotDeepStrictEqual => "assert_not_deep_strict_equal",
            Opcode::AssertThrows => "assert_throws",
            Opcode::AssertDoesNotThrow => "assert_does_not_throw",
            Opcode::AssertRejects => "assert_rejects",
            Opcode::AssertDoesNotReject => "assert_does_not_reject",
            Opcode::AssertFail => "assert_fail",
            Opcode::AddI32 => "add_i32",
            Opcode::AddF64 => "add_f64",
            Opcode::SubI32 => "sub_i32",
            Opcode::SubF64 => "sub_f64",
            Opcode::MulI32 => "mul_i32",
            Opcode::MulF64 => "mul_f64",
            Opcode::Reserved(n) => match n {
                61..=63 => "reserved_61_63",
                123..=127 => "reserved_123_127",
                130..=159 => "reserved_130_159",
                174..=199 => "reserved_174_199",
                225..=239 => "reserved_225_239",
                243..=255 => "reserved_243_255",
                _ => "reserved",
            },
        }
    }

    /// Format instruction as clean human-readable assembly (no byte offsets, clean mnemonics)
    pub fn to_clean_asm(&self, constants: &[crate::js_value::JSValue]) -> String {
        let opcode_str = self.opcode_to_clean_mnemonic();

        match self.format {
            Format::ABC => {
                match self.opcode {
                    Opcode::Call
                    | Opcode::TailCall
                    | Opcode::Construct
                    | Opcode::CallIc
                    | Opcode::CallIcSuper
                    | Opcode::LoadThisCall
                    | Opcode::LoadArgCall => {
                        // A B format
                        format!("{} r{}, {}", opcode_str, self.a, self.b)
                    }
                    Opcode::Call1SubI => {
                        format!("{} r{}, r{}, {}", opcode_str, self.a, self.b, self.c as i8)
                    }
                    Opcode::JmpEq
                    | Opcode::JmpNeq
                    | Opcode::JmpLt
                    | Opcode::JmpLte
                    | Opcode::JmpLteFalse => {
                        format!(
                            "{} r{}, r{}, -> L{}",
                            opcode_str, self.a, self.b, self.c as i8
                        )
                    }
                    _ => {
                        // Standard ABC format
                        format!("{} r{}, r{}, r{}", opcode_str, self.a, self.b, self.c)
                    }
                }
            }
            Format::ABx => {
                match self.opcode {
                    Opcode::LoadK | Opcode::NewFunc => {
                        // Load constant
                        let const_idx = self.bx as usize;
                        let const_val = if const_idx < constants.len() {
                            format!("const[{}]", const_idx)
                        } else {
                            "const[out of bounds]".to_string()
                        };
                        format!("{} r{}, {}", opcode_str, self.a, const_val)
                    }
                    Opcode::LoadGlobalIc
                    | Opcode::SetGlobalIc
                    | Opcode::GetGlobal
                    | Opcode::SetGlobal => {
                        format!("{} r{}, global[{}]", opcode_str, self.a, self.bx)
                    }
                    Opcode::ResolveScope
                    | Opcode::LoadName
                    | Opcode::StoreName
                    | Opcode::TypeofName => {
                        format!("{} r{}, identifier[{}]", opcode_str, self.a, self.bx)
                    }
                    Opcode::LoadKAddAcc | Opcode::LoadKMulAcc | Opcode::LoadKSubAcc => {
                        let const_idx = self.bx as usize;
                        let const_val = if const_idx < constants.len() {
                            format!("const[{}]", const_idx)
                        } else {
                            "const[out of bounds]".to_string()
                        };
                        format!("{} {}", opcode_str, const_val)
                    }
                    _ => {
                        format!("{} r{}, {}", opcode_str, self.a, self.bx)
                    }
                }
            }
            Format::AsBx => match self.opcode {
                Opcode::Jmp => {
                    format!("{} -> L{}", opcode_str, self.sbx)
                }
                Opcode::LoadI => {
                    format!("{} r{}, {}", opcode_str, self.a, self.sbx)
                }
                Opcode::JmpTrue
                | Opcode::JmpFalse
                | Opcode::LoopIncJmp
                | Opcode::Try
                | Opcode::IncJmpFalseLoop
                | Opcode::EqJmpTrue
                | Opcode::LtJmp
                | Opcode::EqJmpFalse
                | Opcode::IncAccJmp
                | Opcode::TestJmpTrue
                | Opcode::LteJmpLoop => {
                    format!("{} r{}, -> L{}", opcode_str, self.a, self.sbx)
                }
                _ => {
                    format!("{} r{}, {}", opcode_str, self.a, self.sbx)
                }
            },
            Format::A => match self.opcode {
                Opcode::AddAccImm8
                | Opcode::SubAccImm8
                | Opcode::MulAccImm8
                | Opcode::DivAccImm8 => {
                    format!("{} {}", opcode_str, self.b as i8)
                }
                Opcode::AddAcc
                | Opcode::SubAcc
                | Opcode::MulAcc
                | Opcode::DivAcc
                | Opcode::Neg
                | Opcode::Inc
                | Opcode::Dec
                | Opcode::ToPrimitive
                | Opcode::AddStrAcc => {
                    format!("{} r{}", opcode_str, self.b)
                }
                Opcode::ArrayPushAcc
                | Opcode::GetUpval
                | Opcode::SetUpval
                | Opcode::GetScope
                | Opcode::SetScope
                | Opcode::ForIn
                | Opcode::IteratorNext
                | Opcode::Spread
                | Opcode::Destructure
                | Opcode::CreateEnv
                | Opcode::LoadClosure
                | Opcode::NewThis => {
                    format!("{} r{}", opcode_str, self.a)
                }
                Opcode::IncAcc
                | Opcode::LoadThis
                | Opcode::Load0
                | Opcode::Load1
                | Opcode::LoadNull
                | Opcode::LoadTrue
                | Opcode::LoadFalse
                | Opcode::NewObj
                | Opcode::NewArr
                | Opcode::NewClass
                | Opcode::LoopHint
                | Opcode::Ret
                | Opcode::RetU
                | Opcode::Leave
                | Opcode::Yield
                | Opcode::Await
                | Opcode::EndTry
                | Opcode::Finally
                | Opcode::ProfileRet
                | Opcode::IcMiss
                | Opcode::OsrEntry
                | Opcode::ProfileHotLoop
                | Opcode::OsrExit
                | Opcode::JitHint
                | Opcode::SafetyCheck => opcode_str.to_string(),
                Opcode::RetReg => {
                    format!("{} r{}", opcode_str, self.a)
                }
                _ => {
                    format!("{} r{}", opcode_str, self.a)
                }
            },
            Format::BC => match self.opcode {
                Opcode::GetPropAccCall | Opcode::GetLengthIcCall => {
                    format!("{} r{}, r{}", opcode_str, self.b, self.c)
                }
                _ => {
                    format!("{} r{}, r{}", opcode_str, self.b, self.c)
                }
            },
        }
    }
}

/// Disassemble bytecode into human-readable assembly
pub fn disassemble(bytecode: &[u32], constants: &[crate::js_value::JSValue]) -> Vec<String> {
    let mut result = Vec::new();

    for (pc, &raw) in bytecode.iter().enumerate() {
        let instr = AsmInstruction::decode(pc, raw);
        result.push(instr.to_asm(constants));
    }

    result
}

/// Disassemble bytecode into clean human-readable assembly (no byte offsets, clean mnemonics)
pub fn disassemble_clean(bytecode: &[u32], constants: &[crate::js_value::JSValue]) -> Vec<String> {
    let mut result = Vec::new();

    for (pc, &raw) in bytecode.iter().enumerate() {
        let instr = AsmInstruction::decode(pc, raw);
        result.push(instr.to_clean_asm(constants));
    }

    result
}

/// Disassemble and print bytecode to stdout
pub fn disassemble_print(bytecode: &[u32], constants: &[crate::js_value::JSValue]) {
    let asm = disassemble(bytecode, constants);
    for line in asm {
        println!("{}", line);
    }
}

/// Disassemble a VM's current bytecode
pub fn disassemble_vm(vm: &VM) -> Vec<String> {
    disassemble(&vm.bytecode, &vm.const_pool)
}

/// Disassemble and print a VM's current bytecode
pub fn disassemble_vm_print(vm: &VM) {
    disassemble_print(&vm.bytecode, &vm.const_pool);
}
