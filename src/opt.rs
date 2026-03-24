use std::collections::{BTreeSet, HashMap};

use crate::js_value::{JSValue, is_null, is_undefined, make_false, make_number, make_true, to_f64};
use crate::vm::Opcode;

const ACC: u8 = 255;
const REG_COUNT: usize = 256;

#[derive(Clone, Debug)]
struct Instruction {
    opcode: Opcode,
    a: u8,
    b: u8,
    c: u8,
    bx: u16,
    sbx: i16,
    target: Option<usize>,
    removed: bool,
}

impl Instruction {
    fn decode(pc: usize, raw: u32) -> Self {
        let opcode = Opcode::from((raw & 0xFF) as u8);
        let a = ((raw >> 8) & 0xFF) as u8;
        let b = ((raw >> 16) & 0xFF) as u8;
        let c = ((raw >> 24) & 0xFF) as u8;
        let bx = ((raw >> 16) & 0xFFFF) as u16;
        let sbx = bx as i16;
        let target = decode_branch_target(opcode, pc, a, b, c, sbx);

        Self {
            opcode,
            a,
            b,
            c,
            bx,
            sbx,
            target,
            removed: false,
        }
    }

    fn encode(&self, pc: usize, boundary_map: &[usize]) -> u32 {
        match self.opcode {
            Opcode::Jmp
            | Opcode::JmpTrue
            | Opcode::JmpFalse
            | Opcode::LoopIncJmp
            | Opcode::Try
            | Opcode::IncJmpFalseLoop
            | Opcode::IncAccJmp
            | Opcode::TestJmpTrue => {
                let target = self.target.unwrap_or(pc + 1).min(boundary_map.len() - 1);
                let offset = boundary_map[target] as isize - (boundary_map[pc] as isize + 1);
                let sbx = i16::try_from(offset).expect("optimized jump offset must fit in i16");
                (((sbx as u16) as u32) << 16) | ((self.a as u32) << 8) | self.opcode.as_u8() as u32
            }
            Opcode::JmpEq
            | Opcode::JmpNeq
            | Opcode::JmpLt
            | Opcode::JmpLte
            | Opcode::JmpLteFalse => {
                let target = self.target.unwrap_or(pc + 1).min(boundary_map.len() - 1);
                let offset = boundary_map[target] as isize - (boundary_map[pc] as isize + 1);
                let offset =
                    i8::try_from(offset).expect("optimized conditional jump offset must fit in i8");
                ((offset as u8 as u32) << 24)
                    | ((self.b as u32) << 16)
                    | ((self.a as u32) << 8)
                    | self.opcode.as_u8() as u32
            }
            Opcode::EqJmpTrue | Opcode::LtJmp | Opcode::EqJmpFalse | Opcode::LteJmpLoop => {
                let target = self.target.unwrap_or(pc + 1).min(boundary_map.len() - 1);
                let offset = boundary_map[target] as isize - (boundary_map[pc] as isize + 1);
                let offset =
                    i8::try_from(offset).expect("optimized conditional jump offset must fit in i8");
                ((self.c as u32) << 24)
                    | ((self.b as u32) << 16)
                    | ((offset as u8 as u32) << 8)
                    | self.opcode.as_u8() as u32
            }
            Opcode::LoadI => {
                let sbx = self.sbx;
                (((sbx as u16) as u32) << 16) | ((self.a as u32) << 8) | self.opcode.as_u8() as u32
            }
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
            | Opcode::LoadKSubAcc
            | Opcode::Enter => {
                ((self.bx as u32) << 16) | ((self.a as u32) << 8) | self.opcode.as_u8() as u32
            }
            _ => {
                ((self.c as u32) << 24)
                    | ((self.b as u32) << 16)
                    | ((self.a as u32) << 8)
                    | self.opcode.as_u8() as u32
            }
        }
    }

    fn new_load_i(dst: u8, value: i16) -> Self {
        Self {
            opcode: Opcode::LoadI,
            a: dst,
            b: 0,
            c: 0,
            bx: value as u16,
            sbx: value,
            target: None,
            removed: false,
        }
    }

    fn new_mov(dst: u8, src: u8) -> Self {
        Self {
            opcode: Opcode::Mov,
            a: dst,
            b: src,
            c: 0,
            bx: src as u16,
            sbx: src as i16,
            target: None,
            removed: false,
        }
    }

    fn new_load_k(dst: u8, index: u16) -> Self {
        Self {
            opcode: Opcode::LoadK,
            a: dst,
            b: 0,
            c: 0,
            bx: index,
            sbx: index as i16,
            target: None,
            removed: false,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
enum RegisterValueKey {
    Immediate(i16),
    Constant(u16),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum KnownValueKind {
    Unknown,
    Undefined,
    Null,
    NonNullish,
}

fn decode_branch_target(
    opcode: Opcode,
    pc: usize,
    a: u8,
    _b: u8,
    c: u8,
    sbx: i16,
) -> Option<usize> {
    let next = pc as isize + 1;
    let target = match opcode {
        Opcode::Jmp
        | Opcode::JmpTrue
        | Opcode::JmpFalse
        | Opcode::LoopIncJmp
        | Opcode::Try
        | Opcode::IncJmpFalseLoop
        | Opcode::IncAccJmp
        | Opcode::TestJmpTrue => next + sbx as isize,
        Opcode::JmpEq | Opcode::JmpNeq | Opcode::JmpLt | Opcode::JmpLte | Opcode::JmpLteFalse => {
            next + c as i8 as isize
        }
        Opcode::EqJmpTrue | Opcode::LtJmp | Opcode::EqJmpFalse | Opcode::LteJmpLoop => {
            next + a as i8 as isize
        }
        _ => return None,
    };

    Some(target.max(0) as usize)
}

fn switch_targets(pc: usize, table_index: usize, constants: &[JSValue]) -> Vec<usize> {
    let Some(case_count) = constants.get(table_index).and_then(|value| to_f64(*value)) else {
        return Vec::new();
    };
    let case_count = case_count as usize;
    let mut targets = Vec::with_capacity(case_count + 1);

    if let Some(default_offset) = constants
        .get(table_index + 1)
        .and_then(|value| to_f64(*value))
    {
        targets.push(((pc + 1) as isize + default_offset as i16 as isize).max(0) as usize);
    }

    for case_index in 0..case_count {
        let offset_index = table_index + 2 + case_index * 2 + 1;
        if let Some(offset) = constants.get(offset_index).and_then(|value| to_f64(*value)) {
            targets.push(((pc + 1) as isize + offset as i16 as isize).max(0) as usize);
        }
    }

    targets
}

fn is_terminator(opcode: Opcode) -> bool {
    matches!(
        opcode,
        Opcode::Jmp | Opcode::Ret | Opcode::RetU | Opcode::RetReg | Opcode::Throw
    )
}

fn collect_block_leaders(insts: &[Instruction], constants: &[JSValue]) -> Vec<usize> {
    let mut leaders = BTreeSet::new();
    leaders.insert(0);

    for (pc, inst) in insts.iter().enumerate() {
        if let Some(target) = inst.target {
            leaders.insert(target.min(insts.len()));
            if pc + 1 < insts.len() {
                leaders.insert(pc + 1);
            }
        }

        if inst.opcode == Opcode::Switch {
            leaders.extend(
                switch_targets(pc, inst.b as usize, constants)
                    .into_iter()
                    .map(|target| target.min(insts.len())),
            );
            if pc + 1 < insts.len() {
                leaders.insert(pc + 1);
            }
        }

        if is_terminator(inst.opcode) && pc + 1 < insts.len() {
            leaders.insert(pc + 1);
        }
    }

    leaders.into_iter().collect()
}

fn rewrite_load_move(insts: &mut [Instruction], first: usize, second: usize) -> bool {
    if insts[first].opcode != Opcode::LoadI || insts[second].opcode != Opcode::Mov {
        return false;
    }
    if insts[second].b != insts[first].a {
        return false;
    }

    let value = insts[first].sbx;
    insts[second] = Instruction::new_load_i(insts[second].a, value);
    true
}

fn fold_const_add(insts: &mut [Instruction], first: usize, second: usize, third: usize) -> bool {
    if insts[first].opcode != Opcode::LoadI
        || insts[second].opcode != Opcode::LoadI
        || insts[third].opcode != Opcode::Add
    {
        return false;
    }

    if insts[first].a == insts[second].a {
        return false;
    }

    if insts[third].b != insts[first].a || insts[third].c != insts[second].a {
        return false;
    }

    let folded = insts[first].sbx as i32 + insts[second].sbx as i32;
    if !(i16::MIN as i32..=i16::MAX as i32).contains(&folded) {
        return false;
    }

    insts[third] = Instruction::new_load_i(ACC, folded as i16);
    true
}

fn eliminate_dead_defs(
    insts: &mut [Instruction],
    start: usize,
    end: usize,
    terminal: bool,
) -> bool {
    let mut changed = false;
    let mut live = [false; REG_COUNT];

    if !terminal {
        live.fill(true);
    }

    for index in (start..end).rev() {
        if insts[index].removed {
            continue;
        }

        match insts[index].opcode {
            Opcode::Mov => {
                let dst = insts[index].a as usize;
                let src = insts[index].b as usize;
                if !live[dst] {
                    insts[index].removed = true;
                    changed = true;
                    continue;
                }
                live[dst] = false;
                live[src] = true;
            }
            Opcode::LoadI | Opcode::LoadK => {
                let dst = insts[index].a as usize;
                if !live[dst] {
                    insts[index].removed = true;
                    changed = true;
                    continue;
                }
                live[dst] = false;
            }
            Opcode::Add
            | Opcode::Eq
            | Opcode::Lt
            | Opcode::Lte
            | Opcode::StrictEq
            | Opcode::StrictNeq
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
            | Opcode::Instanceof
            | Opcode::AddStr => {
                live[ACC as usize] = false;
                live[insts[index].b as usize] = true;
                live[insts[index].c as usize] = true;
            }
            Opcode::AddAcc
            | Opcode::SubAcc
            | Opcode::MulAcc
            | Opcode::DivAcc
            | Opcode::AddStrAcc => {
                live[ACC as usize] = true;
                live[insts[index].b as usize] = true;
            }
            Opcode::AddAccImm8
            | Opcode::SubAccImm8
            | Opcode::MulAccImm8
            | Opcode::DivAccImm8
            | Opcode::IncAcc => {
                live[ACC as usize] = true;
            }
            Opcode::LoadThis
            | Opcode::Load0
            | Opcode::Load1
            | Opcode::LoadNull
            | Opcode::LoadTrue
            | Opcode::LoadFalse => {
                live[ACC as usize] = false;
                if insts[index].opcode == Opcode::LoadThis {
                    live[0] = true;
                }
            }
            Opcode::LoadAcc => {
                live[ACC as usize] = false;
                live[insts[index].a as usize] = true;
            }
            Opcode::JmpTrue | Opcode::JmpFalse => {
                live[insts[index].a as usize] = true;
            }
            Opcode::JmpEq
            | Opcode::JmpNeq
            | Opcode::JmpLt
            | Opcode::JmpLte
            | Opcode::JmpLteFalse => {
                live[insts[index].a as usize] = true;
                live[insts[index].b as usize] = true;
            }
            Opcode::LoopIncJmp => {
                live[insts[index].a as usize] = true;
                live[ACC as usize] = true;
            }
            Opcode::EqJmpTrue | Opcode::LtJmp | Opcode::EqJmpFalse | Opcode::LteJmpLoop => {
                live[insts[index].b as usize] = true;
                live[insts[index].c as usize] = true;
            }
            Opcode::Ret => {
                live[ACC as usize] = true;
            }
            Opcode::RetReg => {
                live[insts[index].a as usize] = true;
            }
            Opcode::RetU | Opcode::Jmp => {}
            _ => {
                live.fill(true);
            }
        }
    }

    changed
}

fn optimize_peephole_block(insts: &mut [Instruction], start: usize, end: usize) -> bool {
    let mut changed = false;

    loop {
        let mut local_change = false;
        let live_indices: Vec<_> = (start..end)
            .filter(|&index| !insts[index].removed)
            .collect();

        for (pos, &index) in live_indices.iter().enumerate() {
            if insts[index].opcode == Opcode::Mov && insts[index].a == insts[index].b {
                insts[index].removed = true;
                local_change = true;
                continue;
            }

            if let Some(&next_index) = live_indices.get(pos + 1) {
                if insts[index].opcode == Opcode::Mov
                    && insts[next_index].opcode == Opcode::Mov
                    && insts[next_index].a == insts[index].b
                    && insts[next_index].b == insts[index].a
                {
                    insts[next_index].removed = true;
                    local_change = true;
                }

                if rewrite_load_move(insts, index, next_index) {
                    local_change = true;
                }
            }

            if let (Some(&next_index), Some(&third_index)) =
                (live_indices.get(pos + 1), live_indices.get(pos + 2))
                && fold_const_add(insts, index, next_index, third_index)
            {
                local_change = true;
            }
        }

        if !local_change {
            break;
        }

        changed = true;
    }

    changed
}

fn decode_program(bytecode: &[u32]) -> Vec<Instruction> {
    bytecode
        .iter()
        .enumerate()
        .map(|(pc, &raw)| Instruction::decode(pc, raw))
        .collect()
}

fn encode_program(insts: &[Instruction], mut constants: Vec<JSValue>) -> (Vec<u32>, Vec<JSValue>) {
    let boundary = build_boundary_map(insts);
    let bytecode: Vec<_> = insts
        .iter()
        .enumerate()
        .filter(|(_, inst)| !inst.removed)
        .map(|(pc, inst)| inst.encode(pc, &boundary))
        .collect();
    rewrite_switch_tables(insts, &mut constants, &boundary);
    (bytecode, constants)
}

fn has_removed_instructions(insts: &[Instruction]) -> bool {
    insts.iter().any(|inst| inst.removed)
}

fn run_block_pass<F>(insts: &mut [Instruction], constants: &[JSValue], mut pass: F) -> bool
where
    F: FnMut(&mut [Instruction], usize, usize, bool) -> bool,
{
    let leaders = collect_block_leaders(insts, constants);
    let mut changed = false;

    for (block_index, &start) in leaders.iter().enumerate() {
        if start >= insts.len() {
            continue;
        }
        let end = leaders
            .get(block_index + 1)
            .copied()
            .unwrap_or(insts.len())
            .min(insts.len());
        let terminal = end == insts.len()
            || (start..end)
                .rev()
                .find(|&index| !insts[index].removed)
                .is_some_and(|index| is_terminator(insts[index].opcode));
        if pass(insts, start, end, terminal) {
            changed = true;
        }
    }

    changed
}

fn resolve_alias(aliases: &[Option<u8>; REG_COUNT], reg: u8) -> u8 {
    let mut current = reg;
    let mut steps = 0usize;
    while steps < REG_COUNT {
        let Some(next) = aliases[current as usize] else {
            break;
        };
        if next == current {
            break;
        }
        current = next;
        steps += 1;
    }
    current
}

fn invalidate_alias(aliases: &mut [Option<u8>; REG_COUNT], reg: u8) {
    aliases[reg as usize] = None;
    for alias in aliases.iter_mut() {
        if *alias == Some(reg) {
            *alias = None;
        }
    }
}

fn rewrite_reg(aliases: &[Option<u8>; REG_COUNT], reg: &mut u8) -> bool {
    let resolved = resolve_alias(aliases, *reg);
    if resolved != *reg {
        *reg = resolved;
        true
    } else {
        false
    }
}

fn invalidate_value_key(
    available: &mut HashMap<RegisterValueKey, u8>,
    values: &mut [Option<RegisterValueKey>; REG_COUNT],
    reg: u8,
) {
    if let Some(key) = values[reg as usize].take()
        && available.get(&key).copied() == Some(reg)
    {
        available.remove(&key);
    }
}

fn record_value_key(
    available: &mut HashMap<RegisterValueKey, u8>,
    values: &mut [Option<RegisterValueKey>; REG_COUNT],
    reg: u8,
    key: RegisterValueKey,
) {
    invalidate_value_key(available, values, reg);
    values[reg as usize] = Some(key);
    available.insert(key, reg);
}

fn coalesce_registers_block(insts: &mut [Instruction], start: usize, end: usize) -> bool {
    let mut changed = false;
    let mut available = HashMap::new();
    let mut values = [None; REG_COUNT];

    for inst in &mut insts[start..end] {
        if inst.removed {
            continue;
        }

        match inst.opcode {
            Opcode::LoadI => {
                let key = RegisterValueKey::Immediate(inst.sbx);
                if let Some(src) = available.get(&key).copied()
                    && src != inst.a
                {
                    *inst = Instruction::new_mov(inst.a, src);
                    changed = true;
                }
                record_value_key(&mut available, &mut values, inst.a, key);
            }
            Opcode::LoadK => {
                let key = RegisterValueKey::Constant(inst.bx);
                if let Some(src) = available.get(&key).copied()
                    && src != inst.a
                {
                    *inst = Instruction::new_mov(inst.a, src);
                    changed = true;
                }
                record_value_key(&mut available, &mut values, inst.a, key);
            }
            Opcode::Mov => {
                invalidate_value_key(&mut available, &mut values, inst.a);
                if let Some(key) = values[inst.b as usize] {
                    record_value_key(&mut available, &mut values, inst.a, key);
                }
            }
            Opcode::LoadAcc => {
                invalidate_value_key(&mut available, &mut values, ACC);
                if let Some(key) = values[inst.a as usize] {
                    record_value_key(&mut available, &mut values, ACC, key);
                }
            }
            Opcode::Add
            | Opcode::Eq
            | Opcode::Lt
            | Opcode::Lte
            | Opcode::StrictEq
            | Opcode::StrictNeq
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
            | Opcode::Instanceof
            | Opcode::AddStr
            | Opcode::AddAcc
            | Opcode::SubAcc
            | Opcode::MulAcc
            | Opcode::DivAcc
            | Opcode::AddStrAcc
            | Opcode::AddAccImm8
            | Opcode::SubAccImm8
            | Opcode::MulAccImm8
            | Opcode::DivAccImm8
            | Opcode::IncAcc
            | Opcode::LoadThis
            | Opcode::Load0
            | Opcode::Load1
            | Opcode::LoadNull
            | Opcode::LoadTrue
            | Opcode::LoadFalse
            | Opcode::Neg
            | Opcode::Inc
            | Opcode::Dec
            | Opcode::ToPrimitive
            | Opcode::BitNot => {
                invalidate_value_key(&mut available, &mut values, ACC);
            }
            Opcode::Typeof
            | Opcode::ToNum
            | Opcode::ToStr
            | Opcode::IsUndef
            | Opcode::IsNull
            | Opcode::LoadArg
            | Opcode::GetScope
            | Opcode::SetScope
            | Opcode::LoadGlobalIc
            | Opcode::GetGlobal
            | Opcode::GetUpval
            | Opcode::NewObj
            | Opcode::NewArr
            | Opcode::NewFunc
            | Opcode::NewClass
            | Opcode::NewThis
            | Opcode::LoadClosure
            | Opcode::ResolveScope
            | Opcode::LoadName
            | Opcode::TypeofName
            | Opcode::CreateEnv
            | Opcode::Keys
            | Opcode::ForIn
            | Opcode::IteratorNext => {
                invalidate_value_key(&mut available, &mut values, inst.a);
            }
            Opcode::GetProp
            | Opcode::SetProp
            | Opcode::GetSuper
            | Opcode::SetSuper
            | Opcode::DeleteProp
            | Opcode::HasProp
            | Opcode::GetPropIc
            | Opcode::SetPropIc
            | Opcode::GetIdxFast
            | Opcode::SetIdxFast
            | Opcode::GetIdxIc
            | Opcode::SetIdxIc
            | Opcode::GetLengthIc => {
                invalidate_value_key(&mut available, &mut values, inst.a);
                invalidate_value_key(&mut available, &mut values, ACC);
            }
            Opcode::Ret
            | Opcode::RetU
            | Opcode::RetReg
            | Opcode::Jmp
            | Opcode::JmpTrue
            | Opcode::JmpFalse
            | Opcode::JmpEq
            | Opcode::JmpNeq
            | Opcode::JmpLt
            | Opcode::JmpLte
            | Opcode::JmpLteFalse
            | Opcode::EqJmpTrue
            | Opcode::LtJmp
            | Opcode::EqJmpFalse
            | Opcode::LoopIncJmp
            | Opcode::Try
            | Opcode::IncJmpFalseLoop
            | Opcode::IncAccJmp
            | Opcode::TestJmpTrue
            | Opcode::LteJmpLoop
            | Opcode::Yield
            | Opcode::Await
            | Opcode::Throw
            | Opcode::Switch
            | Opcode::LoopHint => {}
            _ => {
                available.clear();
                values.fill(None);
            }
        }
    }

    changed
}

fn classify_known_value(value: JSValue) -> KnownValueKind {
    if is_undefined(value) {
        KnownValueKind::Undefined
    } else if is_null(value) {
        KnownValueKind::Null
    } else {
        KnownValueKind::NonNullish
    }
}

fn bool_constant_index(
    constants: &mut Vec<JSValue>,
    value: bool,
    true_index: &mut Option<u16>,
    false_index: &mut Option<u16>,
) -> u16 {
    let slot = if value { true_index } else { false_index };
    if let Some(index) = *slot {
        return index;
    }

    let needle = if value { make_true() } else { make_false() };
    if let Some(index) = constants.iter().position(|constant| *constant == needle) {
        let index = index as u16;
        *slot = Some(index);
        return index;
    }

    let index = u16::try_from(constants.len()).expect("constant pool index must fit in u16");
    constants.push(needle);
    *slot = Some(index);
    index
}

fn fold_temporary_checks_block(
    insts: &mut [Instruction],
    constants: &mut Vec<JSValue>,
    start: usize,
    end: usize,
) -> bool {
    let mut changed = false;
    let mut known = [KnownValueKind::Unknown; REG_COUNT];
    let mut true_index = None;
    let mut false_index = None;

    for inst in &mut insts[start..end] {
        if inst.removed {
            continue;
        }

        match inst.opcode {
            Opcode::LoadI => {
                known[inst.a as usize] = KnownValueKind::NonNullish;
            }
            Opcode::LoadK => {
                known[inst.a as usize] = constants
                    .get(inst.bx as usize)
                    .copied()
                    .map(classify_known_value)
                    .unwrap_or(KnownValueKind::Unknown);
            }
            Opcode::Mov => {
                known[inst.a as usize] = known[inst.b as usize];
            }
            Opcode::LoadAcc => {
                known[ACC as usize] = known[inst.a as usize];
            }
            Opcode::LoadThis
            | Opcode::Load0
            | Opcode::Load1
            | Opcode::LoadTrue
            | Opcode::LoadFalse => {
                known[ACC as usize] = KnownValueKind::NonNullish;
            }
            Opcode::LoadNull => {
                known[ACC as usize] = KnownValueKind::Null;
            }
            Opcode::IsUndef => {
                let replacement = match known[inst.b as usize] {
                    KnownValueKind::Undefined => Some(true),
                    KnownValueKind::Null | KnownValueKind::NonNullish => Some(false),
                    KnownValueKind::Unknown => None,
                };
                if let Some(value) = replacement {
                    let index =
                        bool_constant_index(constants, value, &mut true_index, &mut false_index);
                    *inst = Instruction::new_load_k(inst.a, index);
                    changed = true;
                }
                known[inst.a as usize] = KnownValueKind::NonNullish;
            }
            Opcode::IsNull => {
                let replacement = match known[inst.b as usize] {
                    KnownValueKind::Null => Some(true),
                    KnownValueKind::Undefined | KnownValueKind::NonNullish => Some(false),
                    KnownValueKind::Unknown => None,
                };
                if let Some(value) = replacement {
                    let index =
                        bool_constant_index(constants, value, &mut true_index, &mut false_index);
                    *inst = Instruction::new_load_k(inst.a, index);
                    changed = true;
                }
                known[inst.a as usize] = KnownValueKind::NonNullish;
            }
            Opcode::Typeof
            | Opcode::ToNum
            | Opcode::ToStr
            | Opcode::LoadArg
            | Opcode::GetScope
            | Opcode::SetScope
            | Opcode::LoadGlobalIc
            | Opcode::GetGlobal
            | Opcode::GetUpval
            | Opcode::NewObj
            | Opcode::NewArr
            | Opcode::NewFunc
            | Opcode::NewClass
            | Opcode::NewThis
            | Opcode::LoadClosure
            | Opcode::ResolveScope
            | Opcode::LoadName
            | Opcode::TypeofName
            | Opcode::CreateEnv
            | Opcode::Keys
            | Opcode::ForIn
            | Opcode::IteratorNext => {
                known[inst.a as usize] = KnownValueKind::Unknown;
            }
            Opcode::GetProp
            | Opcode::SetProp
            | Opcode::GetSuper
            | Opcode::SetSuper
            | Opcode::DeleteProp
            | Opcode::HasProp
            | Opcode::GetPropIc
            | Opcode::SetPropIc
            | Opcode::GetIdxFast
            | Opcode::SetIdxFast
            | Opcode::GetIdxIc
            | Opcode::SetIdxIc
            | Opcode::GetLengthIc => {
                known[inst.a as usize] = KnownValueKind::Unknown;
                known[ACC as usize] = KnownValueKind::Unknown;
            }
            Opcode::Add
            | Opcode::Eq
            | Opcode::Lt
            | Opcode::Lte
            | Opcode::StrictEq
            | Opcode::StrictNeq
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
            | Opcode::Instanceof
            | Opcode::AddStr
            | Opcode::AddAcc
            | Opcode::SubAcc
            | Opcode::MulAcc
            | Opcode::DivAcc
            | Opcode::AddStrAcc
            | Opcode::AddAccImm8
            | Opcode::SubAccImm8
            | Opcode::MulAccImm8
            | Opcode::DivAccImm8
            | Opcode::IncAcc
            | Opcode::Neg
            | Opcode::Inc
            | Opcode::Dec
            | Opcode::ToPrimitive
            | Opcode::BitNot => {
                known[ACC as usize] = KnownValueKind::Unknown;
            }
            Opcode::Ret
            | Opcode::RetU
            | Opcode::RetReg
            | Opcode::Jmp
            | Opcode::JmpTrue
            | Opcode::JmpFalse
            | Opcode::JmpEq
            | Opcode::JmpNeq
            | Opcode::JmpLt
            | Opcode::JmpLte
            | Opcode::JmpLteFalse
            | Opcode::EqJmpTrue
            | Opcode::LtJmp
            | Opcode::EqJmpFalse
            | Opcode::LoopIncJmp
            | Opcode::Try
            | Opcode::IncJmpFalseLoop
            | Opcode::IncAccJmp
            | Opcode::TestJmpTrue
            | Opcode::LteJmpLoop
            | Opcode::Yield
            | Opcode::Await
            | Opcode::Throw
            | Opcode::Switch
            | Opcode::LoopHint => {}
            _ => {
                known.fill(KnownValueKind::Unknown);
            }
        }
    }

    changed
}

fn copy_propagation_block(insts: &mut [Instruction], start: usize, end: usize) -> bool {
    let mut changed = false;
    let mut aliases = [None; REG_COUNT];

    for inst in &mut insts[start..end] {
        if inst.removed {
            continue;
        }

        match inst.opcode {
            Opcode::Mov => {
                changed |= rewrite_reg(&aliases, &mut inst.b);
                invalidate_alias(&mut aliases, inst.a);
                if inst.a != inst.b {
                    aliases[inst.a as usize] = Some(inst.b);
                }
            }
            Opcode::Add
            | Opcode::Eq
            | Opcode::Lt
            | Opcode::Lte
            | Opcode::StrictEq
            | Opcode::StrictNeq
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
            | Opcode::Instanceof
            | Opcode::AddStr => {
                changed |= rewrite_reg(&aliases, &mut inst.b);
                changed |= rewrite_reg(&aliases, &mut inst.c);
                invalidate_alias(&mut aliases, ACC);
            }
            Opcode::AddAcc
            | Opcode::SubAcc
            | Opcode::MulAcc
            | Opcode::DivAcc
            | Opcode::AddStrAcc
            | Opcode::Neg
            | Opcode::Inc
            | Opcode::Dec
            | Opcode::ToPrimitive
            | Opcode::BitNot => {
                changed |= rewrite_reg(&aliases, &mut inst.b);
                invalidate_alias(&mut aliases, ACC);
            }
            Opcode::Typeof
            | Opcode::ToNum
            | Opcode::ToStr
            | Opcode::IsUndef
            | Opcode::IsNull
            | Opcode::LoadArg
            | Opcode::GetScope
            | Opcode::SetScope => {
                changed |= rewrite_reg(&aliases, &mut inst.b);
                invalidate_alias(&mut aliases, inst.a);
            }
            Opcode::GetProp
            | Opcode::SetProp
            | Opcode::GetSuper
            | Opcode::SetSuper
            | Opcode::DeleteProp
            | Opcode::HasProp
            | Opcode::GetPropIc
            | Opcode::SetPropIc
            | Opcode::GetIdxFast
            | Opcode::SetIdxFast
            | Opcode::GetIdxIc
            | Opcode::SetIdxIc
            | Opcode::GetLengthIc => {
                changed |= rewrite_reg(&aliases, &mut inst.a);
                changed |= rewrite_reg(&aliases, &mut inst.b);
                changed |= rewrite_reg(&aliases, &mut inst.c);
                invalidate_alias(&mut aliases, inst.a);
                invalidate_alias(&mut aliases, ACC);
            }
            Opcode::LoadAcc => {
                changed |= rewrite_reg(&aliases, &mut inst.a);
                invalidate_alias(&mut aliases, ACC);
                aliases[ACC as usize] = Some(inst.a);
            }
            Opcode::JmpTrue
            | Opcode::JmpFalse
            | Opcode::Yield
            | Opcode::Await
            | Opcode::Throw
            | Opcode::RetReg => {
                changed |= rewrite_reg(&aliases, &mut inst.a);
            }
            Opcode::JmpEq
            | Opcode::JmpNeq
            | Opcode::JmpLt
            | Opcode::JmpLte
            | Opcode::JmpLteFalse => {
                changed |= rewrite_reg(&aliases, &mut inst.a);
                changed |= rewrite_reg(&aliases, &mut inst.b);
            }
            Opcode::EqJmpTrue | Opcode::LtJmp | Opcode::EqJmpFalse | Opcode::LteJmpLoop => {
                changed |= rewrite_reg(&aliases, &mut inst.b);
                changed |= rewrite_reg(&aliases, &mut inst.c);
            }
            Opcode::LoadI
            | Opcode::LoadK
            | Opcode::LoadGlobalIc
            | Opcode::GetGlobal
            | Opcode::GetUpval
            | Opcode::NewObj
            | Opcode::NewArr
            | Opcode::NewFunc
            | Opcode::NewClass
            | Opcode::NewThis
            | Opcode::LoadClosure
            | Opcode::ResolveScope
            | Opcode::LoadName
            | Opcode::TypeofName
            | Opcode::CreateEnv
            | Opcode::Keys
            | Opcode::ForIn
            | Opcode::IteratorNext => {
                invalidate_alias(&mut aliases, inst.a);
            }
            Opcode::Ret | Opcode::RetU | Opcode::Jmp | Opcode::Switch | Opcode::LoopHint => {}
            _ => {
                aliases.fill(None);
            }
        }
    }

    changed
}

fn resolve_threaded_target(insts: &[Instruction], mut target: usize) -> usize {
    let mut seen = BTreeSet::new();

    while target < insts.len() {
        if !seen.insert(target) {
            break;
        }

        let Some(inst) = insts.get(target) else {
            break;
        };

        if inst.removed {
            target += 1;
            continue;
        }

        if inst.opcode == Opcode::Jmp
            && let Some(next) = inst.target
        {
            target = next;
            continue;
        }

        break;
    }

    target
}

fn thread_jumps(insts: &mut [Instruction]) -> bool {
    let mut changed = false;

    for index in 0..insts.len() {
        if insts[index].removed || insts[index].opcode != Opcode::Jmp {
            continue;
        }

        let Some(target) = insts[index].target else {
            continue;
        };
        let threaded = resolve_threaded_target(insts, target);
        if threaded != target {
            insts[index].target = Some(threaded);
            changed = true;
        }

        if insts[index].target == Some(index + 1) {
            insts[index].removed = true;
            changed = true;
        }
    }

    changed
}

fn build_boundary_map(insts: &[Instruction]) -> Vec<usize> {
    let mut boundary = vec![0; insts.len() + 1];
    let mut next = insts.iter().filter(|inst| !inst.removed).count();
    boundary[insts.len()] = next;

    for index in (0..insts.len()).rev() {
        if !insts[index].removed {
            next -= 1;
            boundary[index] = next;
        } else {
            boundary[index] = next;
        }
    }

    boundary
}

fn rewrite_switch_tables(insts: &[Instruction], constants: &mut [JSValue], boundary: &[usize]) {
    for (pc, inst) in insts.iter().enumerate() {
        if inst.removed || inst.opcode != Opcode::Switch {
            continue;
        }

        let table_index = inst.b as usize;
        let Some(case_count) = constants.get(table_index).and_then(|value| to_f64(*value)) else {
            continue;
        };
        let case_count = case_count as usize;
        let base = boundary[pc] as isize + 1;

        if let Some(default_slot) = constants.get_mut(table_index + 1)
            && let Some(old_offset) = to_f64(*default_slot)
        {
            let old_target = ((pc + 1) as isize + old_offset as i16 as isize).max(0) as usize;
            let new_target = boundary[old_target.min(insts.len())] as isize;
            *default_slot = make_number((new_target - base) as f64);
        }

        for case_index in 0..case_count {
            let offset_index = table_index + 2 + case_index * 2 + 1;
            if let Some(slot) = constants.get_mut(offset_index)
                && let Some(old_offset) = to_f64(*slot)
            {
                let old_target =
                    ((pc + 1) as isize + old_offset as i16 as isize).max(0) as usize;
                let new_target = boundary[old_target.min(insts.len())] as isize;
                *slot = make_number((new_target - base) as f64);
            }
        }
    }
}

pub fn simplify_branches(bytecode: Vec<u32>, constants: Vec<JSValue>) -> (Vec<u32>, Vec<JSValue>) {
    let mut insts = decode_program(&bytecode);
    if !thread_jumps(&mut insts) {
        return (bytecode, constants);
    }
    encode_program(&insts, constants)
}

pub fn eliminate_dead_code(
    bytecode: Vec<u32>,
    constants: Vec<JSValue>,
) -> (Vec<u32>, Vec<JSValue>) {
    let mut insts = decode_program(&bytecode);
    if !run_block_pass(&mut insts, &constants, eliminate_dead_defs) {
        return (bytecode, constants);
    }
    encode_program(&insts, constants)
}

pub fn copy_propagation(bytecode: Vec<u32>, constants: Vec<JSValue>) -> (Vec<u32>, Vec<JSValue>) {
    let mut insts = decode_program(&bytecode);
    let changed = run_block_pass(&mut insts, &constants, |insts, start, end, _terminal| {
        copy_propagation_block(insts, start, end)
    });
    if !changed {
        return (bytecode, constants);
    }
    encode_program(&insts, constants)
}

pub fn coalesce_registers(bytecode: Vec<u32>, constants: Vec<JSValue>) -> (Vec<u32>, Vec<JSValue>) {
    let mut insts = decode_program(&bytecode);
    let changed = run_block_pass(&mut insts, &constants, |insts, start, end, _terminal| {
        coalesce_registers_block(insts, start, end)
    });
    if !changed {
        return (bytecode, constants);
    }
    encode_program(&insts, constants)
}

pub fn fold_temporary_checks(
    bytecode: Vec<u32>,
    mut constants: Vec<JSValue>,
) -> (Vec<u32>, Vec<JSValue>) {
    let mut insts = decode_program(&bytecode);
    let leaders = collect_block_leaders(&insts, &constants);
    let mut changed = false;

    for (block_index, &start) in leaders.iter().enumerate() {
        if start >= insts.len() {
            continue;
        }
        let end = leaders
            .get(block_index + 1)
            .copied()
            .unwrap_or(insts.len())
            .min(insts.len());
        if fold_temporary_checks_block(&mut insts, &mut constants, start, end) {
            changed = true;
        }
    }

    if !changed {
        return (bytecode, constants);
    }
    encode_program(&insts, constants)
}

pub fn optimize_peephole(bytecode: Vec<u32>, constants: Vec<JSValue>) -> (Vec<u32>, Vec<JSValue>) {
    let mut insts = decode_program(&bytecode);
    let changed = run_block_pass(&mut insts, &constants, |insts, start, end, _terminal| {
        optimize_peephole_block(insts, start, end)
    });
    if !changed {
        return (bytecode, constants);
    }
    encode_program(&insts, constants)
}

pub fn relocate_jumps(bytecode: Vec<u32>, constants: Vec<JSValue>) -> (Vec<u32>, Vec<JSValue>) {
    let insts = decode_program(&bytecode);
    if !has_removed_instructions(&insts) {
        return encode_program(&insts, constants);
    }
    encode_program(&insts, constants)
}

pub fn optimize_bytecode(bytecode: Vec<u32>, constants: Vec<JSValue>) -> (Vec<u32>, Vec<JSValue>) {
    let (bytecode, constants) = simplify_branches(bytecode, constants);
    let (bytecode, constants) = copy_propagation(bytecode, constants);
    let (bytecode, constants) = optimize_peephole(bytecode, constants);
    let (bytecode, constants) = coalesce_registers(bytecode, constants);
    let (bytecode, constants) = copy_propagation(bytecode, constants);
    let (bytecode, constants) = fold_temporary_checks(bytecode, constants);
    let (bytecode, constants) = eliminate_dead_code(bytecode, constants);
    let (bytecode, constants) = simplify_branches(bytecode, constants);
    relocate_jumps(bytecode, constants)
}
