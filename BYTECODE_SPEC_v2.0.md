# QJL Bytecode & Runtime Specification v2.0 (Final)

## 1. Introduction

This document is the definitive specification for the QJL virtual machine, combining the complete bytecode definition with a clean Rust runtime core. It is designed for building a high‑performance JavaScript interpreter with inline caches, shape‑based objects, and a generational garbage collector.

---

## 2. Bytecode Encoding

Instructions are 32 bits wide: `[C (8)] [B (8)] [A (8)] [OPCODE (8)]`.

Formats:
- **ABC**: three 8‑bit register operands.
- **ABx**: A + 16‑bit unsigned immediate (B and C combined).
- **AsBx**: A + 16‑bit signed offset (B and C combined).
- **A**: single register operand.
- **BC**: two registers, result in accumulator.

---

## 3. Full Opcode List

The accumulator is register 255. Opcodes are grouped by hotness.

### 3.1 Ultra‑Hot Core (0–31)

| Op | Mnemonic         | Format | Description |
|----|------------------|--------|-------------|
| 0  | mov              | ABC    | `regs[A] = regs[B]` |
| 1  | load_k           | ABx    | `regs[A] = constant[Bx]` |
| 2  | add              | BC     | `acc = regs[B] + regs[C]` |
| 3  | get_prop_ic      | ABC    | `regs[A] = regs[B][IC[C].key]` (IC slot C) |
| 4  | call             | A B    | Call `regs[A]` with B args in `A+1..A+B`, result in acc |
| 5  | jmp              | sBx    | Unconditional jump by signed 16‑bit offset |
| 6  | load_i           | AsBx   | `regs[A] = (int16_t)sBx` |
| 7  | jmp_true         | AsBx   | Jump if `regs[A]` truthy |
| 8  | jmp_false        | AsBx   | Jump if `regs[A]` falsy |
| 9  | set_prop_ic      | ABC    | `regs[B][IC[C].key] = regs[A]` (IC slot C) |
| 10 | add_acc_imm8     | B      | `acc += (int8_t)B` |
| 11 | inc_acc          | –      | `acc++` |
| 12 | load_this        | –      | `acc = this` |
| 13 | load_0           | –      | `acc = 0` |
| 14 | load_1           | –      | `acc = 1` |
| 15 | eq               | BC     | `acc = (regs[B] == regs[C])` (abstract equality) |
| 16 | lt               | BC     | `acc = (regs[B] < regs[C])` |
| 17 | lte              | BC     | `acc = (regs[B] <= regs[C])` |
| 18 | add_acc          | B      | `acc += regs[B]` |
| 19 | sub_acc          | B      | `acc -= regs[B]` |
| 20 | mul_acc          | B      | `acc *= regs[B]` |
| 21 | div_acc          | B      | `acc /= regs[B]` |
| 22 | load_null        | –      | `acc = null` |
| 23 | load_true        | –      | `acc = true` |
| 24 | load_false       | –      | `acc = false` |
| 25 | load_global_ic   | ABx    | `regs[A] = global[IC[Bx]]` (global IC) |
| 26 | set_global_ic    | ABx    | `global[IC[Bx]] = regs[A]` |
| 27 | typeof           | AB     | `regs[A] = interned_string(typeof regs[B])` |
| 28 | to_num           | AB     | `regs[A] = ToNumber(regs[B])` |
| 29 | to_str           | AB     | `regs[A] = ToString(regs[B])` |
| 30 | is_undef         | AB     | `regs[A] = (regs[B] === undefined)` |
| 31 | is_null          | AB     | `regs[A] = (regs[B] === null)` |

### 3.2 Arithmetic & Chaining (32–63)

| Op | Mnemonic         | Format | Description |
|----|------------------|--------|-------------|
| 32 | sub_acc_imm8     | B      | `acc -= (int8_t)B` |
| 33 | mul_acc_imm8     | B      | `acc *= (int8_t)B` |
| 34 | div_acc_imm8     | B      | `acc /= (int8_t)B` |
| 35 | add_str_acc      | B      | `acc += ToString(regs[B])` |
| 36 | add_i            | ABC    | `acc = regs[B] + (int8_t)C`; store to regs[A] if A≠255 |
| 37 | sub_i            | ABC    | `acc = regs[B] - (int8_t)C` |
| 38 | mul_i            | ABC    | `acc = regs[B] * (int8_t)C` |
| 39 | div_i            | ABC    | `acc = regs[B] / (int8_t)C` |
| 40 | mod_i            | ABC    | `acc = regs[B] % (int8_t)C` |
| 41 | neg              | B      | `acc = -regs[B]` |
| 42 | inc              | B      | `acc = regs[B] + 1` |
| 43 | dec              | B      | `acc = regs[B] - 1` |
| 44 | add_str          | BC     | `acc = ToString(regs[B]) + ToString(regs[C])` |
| 45 | to_primitive     | B      | `acc = ToPrimitive(regs[B])` |
| 46 | get_prop_acc     | BC     | `acc = regs[B][regs[C]]` (property name in register) |
| 47 | set_prop_acc     | BC     | `regs[B][regs[C]] = acc` |
| 48 | get_idx_fast     | ABC    | `regs[A] = regs[B][regs[C]]` (fast array path) |
| 49 | set_idx_fast     | ABC    | `regs[B][regs[C]] = regs[A]` (fast array path) |
| 50 | load_arg         | AB     | `regs[A] = (B < argc) ? args[B] : undefined` |
| 51 | load_acc         | A      | `acc = regs[A]` |
| 52 | strict_eq        | BC     | `acc = (regs[B] === regs[C])` |
| 53 | strict_neq       | BC     | `acc = (regs[B] !== regs[C])` |
| 54–63 | reserved       |        | |

### 3.3 Property & Object (64–95)

| Op | Mnemonic         | Format | Description |
|----|------------------|--------|-------------|
| 64 | get_length_ic    | ABC    | `regs[A] = regs[B].length` (IC slot C) |
| 65 | array_push_acc   | A      | Push `acc` onto array `regs[A]` |
| 66 | new_obj          | A      | `regs[A] = {}` |
| 67 | new_arr          | AB     | `regs[A] = []` (size hint B) |
| 68 | new_func         | ABx    | Create closure from descriptor at constant Bx, store in regs[A] |
| 69 | new_class        | AB     | Create class with base `regs[B]`, store in regs[A] |
| 70 | get_prop         | ABC    | Slow fallback: `regs[A] = regs[B][C]` |
| 71 | set_prop         | ABC    | Slow fallback: `regs[B][C] = regs[A]` |
| 72 | get_idx_ic       | ABC    | Keyed get with IC slot C, result in regs[A] |
| 73 | set_idx_ic       | ABC    | Keyed set with IC slot C, store regs[A] |
| 74 | get_global       | ABx    | Slow global load into regs[A] |
| 75 | set_global       | ABx    | Slow global store from regs[A] |
| 76 | get_upval        | AB     | Get upvalue at index B into regs[A] |
| 77 | set_upval        | AB     | Set upvalue at index B to regs[A] |
| 78 | get_scope        | AB     | Get lexical scope at depth B into regs[A] |
| 79 | set_scope        | AB     | Set lexical scope at depth B to regs[A] |
| 80 | resolve_scope    | ABx    | Resolve identifier Bx → environment in regs[A] |
| 81 | get_super        | ABC    | Super property get into regs[A] |
| 82 | set_super        | ABC    | Super property set with regs[A] |
| 83 | delete_prop      | ABC    | `regs[A] = delete regs[B][C]` |
| 84 | has_prop         | ABC    | `regs[A] = (C in regs[B])` |
| 85 | keys             | AB     | `regs[A] = Object.keys(regs[B])` |
| 86 | for_in           | AB     | Prepare for‑in: iterator in regs[A], first key in acc |
| 87 | iterator_next    | A      | Get next value from iterator in regs[A], result in acc |
| 88 | spread           | AB     | Spread elements from array regs[B] into array regs[A] |
| 89 | destructure      | AB     | Destructure from source regs[B] into registers starting at A |
| 90 | create_env       | A      | Create lexical environment, store in regs[A] |
| 91 | load_name        | ABx    | Load variable by identifier constant Bx using scope, result in acc |
| 92 | store_name       | ABx    | Store regs[A] into variable named by identifier constant Bx |
| 93 | load_closure     | AB     | Load captured value from closure at index B into regs[A] |
| 94 | new_this         | A      | Allocate this object for constructor, store in regs[A] |
| 95 | typeof_name      | ABx    | `regs[A] = interned_string(typeof variable named Bx)` (no ReferenceError) |

### 3.4 Control Flow (96–127)

| Op | Mnemonic         | Format | Description |
|----|------------------|--------|-------------|
| 96 | jmp_eq           | ABC    | Jump if `regs[A] == regs[B]` |
| 97 | jmp_neq          | ABC    | Jump if `regs[A] != regs[B]` |
| 98 | jmp_lt           | ABC    | Jump if `regs[A] < regs[B]` |
| 99 | jmp_lte          | ABC    | Jump if `regs[A] <= regs[B]` |
| 100| loop_inc_jmp     | AsBx   | Increment `regs[A]`, jump if still < bound (used in for‑loops) |
| 101| switch           | AB     | Jump table dispatch (B = table index) |
| 102| loop_hint        | –      | Hint for OSR |
| 103| ret              | –      | Return accumulator to caller |
| 104| ret_u            | –      | Return undefined |
| 105| tail_call        | A B    | Tail call (reuse current frame) |
| 106| construct        | A B    | Constructor call |
| 107| call_var         | A      | Call with spread (args in array at regs[A+1]) |
| 108| enter            | uBx    | Allocate frame of size uBx |
| 109| leave            | –      | Destroy frame |
| 110| yield            | A      | Generator yield (value in regs[A]) |
| 111| await            | A      | Async await |
| 112| throw            | A      | Throw `regs[A]` |
| 113| try              | sBx    | Start try (jump to catch) |
| 114| end_try          | –      | End try |
| 115| catch            | A      | Catch handler (store exception in regs[A]) |
| 116| finally          | –      | Finally block start |
| 117–127| reserved     |        | |

### 3.5 Call & Return (128–159)

| Op | Mnemonic         | Format | Description |
|----|------------------|--------|-------------|
| 128| call_ic          | A B    | Call with IC (method call) |
| 129| call_ic_var      | A      | Spread call with IC |
| 130–159| reserved     |        | |

### 3.6 Profiling / OSR / Feedback (160–199)

| Op | Mnemonic         | Format | Description |
|----|------------------|--------|-------------|
| 160| profile_type     | BC     | Record type of register B in slot C |
| 161| profile_call     | BC     | Record call target B in slot C |
| 162| profile_ret      | –      | Profile return |
| 163| check_type       | BC     | Deopt if type of B not expected |
| 164| check_struct     | BC     | Deopt if structure ID mismatch |
| 165| check_ic         | BC     | Verify IC slot C for object B |
| 166| ic_init          | AB     | Initialize IC slot A with struct of B |
| 167| ic_update        | AB     | Update IC slot A to new struct |
| 168| ic_miss          | A      | Handle IC miss |
| 169| osr_entry        | –      | OSR entry point |
| 170| profile_hot_loop | –      | Record loop hotness |
| 171| osr_exit         | –      | Deoptimize to interpreter |
| 172| jit_hint         | A      | Hint for JIT |
| 173| safety_check     | A      | Runtime safety |
| 174–199| reserved     |        | |

### 3.7 Superinstructions (200–255)

| Op | Mnemonic                     | Format | Fused Instructions |
|----|------------------------------|--------|---------------------|
| 200| get_prop_ic_call             | ABC    | `get_prop_ic + call` |
| 201| inc_jmp_false_loop           | AsBx   | `inc_acc + jmp_false` |
| 202| load_k_add_acc               | Bx     | `load_k + add_acc` |
| 203| add_mov                      | ABC    | `add + mov` |
| 204| eq_jmp_true                  | BC sBx | `eq + jmp_true` |
| 205| get_prop_acc_call            | BC     | `get_prop_acc + call` |
| 206| load_k_mul_acc               | Bx     | `load_k + mul_acc` |
| 207| lt_jmp                       | BC sBx | `lt + jmp` |
| 208| get_prop_ic_mov              | ABC    | `get_prop_ic + mov` |
| 209| get_prop_add_imm_set_prop_ic | AB C   | `get_prop_ic + add_acc_imm8 + set_prop_ic` |
| 210| add_acc_imm8_mov             | B A    | `add_acc_imm8 + mov` |
| 211| call_ic                      | A B    | `get_prop_ic + call` (monomorphic) |
| 212| load_this_call               | B      | `load_this + call` |
| 213| eq_jmp_false                 | BC sBx | `eq + jmp_false` |
| 214| load_k_sub_acc               | Bx     | `load_k + sub_acc` |
| 215| get_length_ic_call           | B      | `get_length_ic + call` |
| 216| add_str_acc_mov              | B A    | `add_str_acc + mov` |
| 217| inc_acc_jmp                  | sBx    | `inc_acc + jmp` |
| 218| get_prop_chain_acc           | B C    | `get_prop_acc + get_prop_acc` (chained: `acc = regs[regs[B]][C]`) |
| 219| test_jmp_true                | A sBx  | `test + jmp_true` |
| 220| load_arg_call                | A B    | `load_arg + call` |
| 221| mul_acc_mov                  | B A    | `mul_acc + mov` |
| 222| lte_jmp_loop                 | BC sBx | `lte + jmp` |
| 223| new_obj_init_prop            | ABC    | `new_obj + init_prop` |
| 224| profile_hot_call             | B C    | `call + profile_call` |
| 225–255| reserved                 |        | |

---

## 4. Rust Runtime Core

### 4.1 JSValue (NaN‑Boxing)

```rust
#[repr(transparent)]
#[derive(Clone, Copy)]
pub struct JSValue(pub u64);

impl JSValue {
    const TAG_INT:   u64 = 0xFFFF_0000_0000_0000;
    const TAG_BOOL:  u64 = 0xFFFE_0000_0000_0000;
    const TAG_NULL:  u64 = 0xFFFD_0000_0000_0000;
    const TAG_UNDEF: u64 = 0xFFFC_0000_0000_0000;
    const TAG_OBJ:   u64 = 0xFFFB_0000_0000_0000;
    const TAG_STR:   u64 = 0xFFFA_0000_0000_0000;

    #[inline(always)]
    pub fn int(i: i32) -> Self {
        JSValue(Self::TAG_INT | (i as u32 as u64))
    }

    #[inline(always)]
    pub fn bool(b: bool) -> Self {
        JSValue(Self::TAG_BOOL | (b as u64))
    }

    #[inline(always)]
    pub fn undefined() -> Self {
        JSValue(Self::TAG_UNDEF)
    }

    #[inline(always)]
    pub fn null() -> Self {
        JSValue(Self::TAG_NULL)
    }

    #[inline(always)]
    pub fn object(ptr: *mut JSObject) -> Self {
        JSValue(Self::TAG_OBJ | (ptr as u64))
    }

    #[inline(always)]
    pub fn string(ptr: *mut JSString) -> Self {
        JSValue(Self::TAG_STR | (ptr as u64))
    }

    #[inline(always)]
    pub fn is_object(self) -> bool {
        (self.0 & 0xFFFF_0000_0000_0000) == Self::TAG_OBJ
    }

    #[inline(always)]
    pub fn as_object(self) -> *mut JSObject {
        (self.0 & 0x0000_FFFF_FFFF_FFFF) as *mut JSObject
    }
}
```

### 4.2 Object, Shape, String

#### JSObject

```rust
pub struct JSObject {
    pub shape: *mut Shape,
    pub props: *mut JSValue,
}
```

#### Shape (Hidden Class)

```rust
pub struct Shape {
    pub id: u32,
    pub parent: *mut Shape,
    pub transitions: *mut TransitionTable,
    pub prop_offsets: *mut u32,
    pub prop_count: u32,
}
```

#### Transition Table (for adding properties)

```rust
pub struct Transition {
    pub key: *mut JSString,
    pub next_shape: *mut Shape,
}

pub struct TransitionTable {
    pub entries: *mut Transition,
    pub count: u32,
}
```

#### JSString (interned)

```rust
pub struct JSString {
    pub hash: u32,
    pub len: u32,
    pub data: *const u8,
}
```

### 4.3 Inline Cache

```rust
#[repr(C)]
pub struct InlineCache {
    pub shape_id: u32,
    pub offset: u32,
    pub key: JSValue,   // interned string
    pub state: ICState,
}

#[repr(u8)]
pub enum ICState {
    Uninit = 0,
    Mono   = 1,
    Poly   = 2,
    Mega   = 3,
}
```

### 4.4 Function & Bytecode

```rust
pub struct Function {
    pub code: *const u8,        // bytecode
    pub constants: *const JSValue,
    pub ic_count: u32,
    pub reg_count: u32,
    pub param_count: u32,
}
```

### 4.5 Frame (Matches Your Spec)

```rust
#[repr(C)]
pub struct Frame {
    pub prev: *mut Frame,
    pub return_pc: *const u8,
    pub func: *mut Function,
    pub env: *mut Env,
    pub ic_vector: *mut InlineCache,
    pub args: *const JSValue,
    pub argc: u32,
    pub reg_count: u32,
    pub regs: [JSValue; 256],   // regs[255] = accumulator
}
```

Accumulator access:

```rust
#[inline(always)]
pub fn acc(frame: *mut Frame) -> *mut JSValue {
    unsafe { &mut (*frame).regs[255] }
}
```

### 4.6 Environment (Scope Chain)

```rust
pub struct Env {
    pub parent: *mut Env,
    pub slots: *mut JSValue,
    pub count: u32,
}
```

### 4.7 VM Structure

```rust
pub struct VM {
    pub current_frame: *mut Frame,
    pub global_obj: *mut JSObject,

    // GC
    pub heap: *mut u8,
    pub heap_size: usize,

    pub nursery_start: *mut u8,
    pub nursery_end: *mut u8,
    pub nursery_ptr: *mut u8,

    // Interned strings
    pub string_table: *mut StringTable,
}
```

### 4.8 GC Header (Placed Before Every Heap Object)

```rust
#[repr(C)]
pub struct GCHeader {
    pub marked: bool,
    pub obj_type: ObjType,
}

#[repr(u8)]
pub enum ObjType {
    Object,
    String,
    Array,
    Function,
    Env,
}
```

Example of an object with header:

```rust
#[repr(C)]
pub struct GCObject {
    pub header: GCHeader,
    pub object: JSObject,
}
```

### 4.9 Write Barrier (Required for Generational GC)

```rust
#[inline(always)]
pub fn write_barrier(vm: &mut VM, obj: *mut JSObject, value: JSValue) {
    if is_old(obj) && is_young(value) {
        remember_set_insert(vm, obj);
    }
}
```

Call this in:
- `set_prop_ic`
- `set_idx_fast`
- `store_name`
- `set_upval`

### 4.10 Fast Inline Cache Example (Rust)

```rust
#[inline(always)]
pub unsafe fn get_prop_ic(
    regs: *mut JSValue,
    ic: *mut InlineCache,
    dst: u8,
    obj_reg: u8,
    ic_slot: u8,
) {
    let obj_val = *regs.add(obj_reg as usize);
    let obj = obj_val.as_object();

    let ic_entry = ic.add(ic_slot as usize);

    if (*obj).shape.id == (*ic_entry).shape_id {
        let offset = (*ic_entry).offset;
        *regs.add(dst as usize) = *(*obj).props.add(offset as usize);
    } else {
        ic_miss(obj, ic_entry, regs.add(dst as usize));
    }
}
```

### 4.11 GC Root Scanning

```rust
pub unsafe fn scan_roots(vm: &mut VM) {
    let mut frame = vm.current_frame;

    while !frame.is_null() {
        mark_slice((*frame).regs.as_mut_ptr(), 256);
        mark_slice((*frame).args as *mut JSValue, (*frame).argc as usize);
        mark_env((*frame).env);
        frame = (*frame).prev;
    }

    mark_object(vm.global_obj);
}
```

---

## 5. Interpreter Loop (Rust)

```rust
type OpHandler = unsafe fn(&mut VM, *mut u8, *mut Frame) -> *mut u8;

static DISPATCH_TABLE: [OpHandler; 256] = [
    op_mov, op_load_k, op_add, op_get_prop_ic, // ...
];

unsafe fn run(mut vm: &mut VM) {
    let mut frame = vm.current_frame;
    let mut pc = frame.return_pc as *const Instruction;

    loop {
        let instr = &*pc;
        pc = pc.add(1);
        let handler = DISPATCH_TABLE[instr.opcode as usize];
        pc = handler(vm, pc as *mut u8, frame);
        if pc.is_null() { break; }
    }
}
```

Example handler:

```rust
unsafe fn op_mov(vm: &mut VM, pc: *mut u8, frame: *mut Frame) -> *mut u8 {
    let instr = &*(pc as *const Instruction);
    let regs = &mut (*frame).regs;
    regs[instr.a as usize] = regs[instr.b as usize];
    pc.add(4)
}
```

---

## 6. Implementation Order

1. `JSValue` with NaN‑boxing.
2. GC header + bump allocator.
3. `JSObject` and `Shape`.
4. `InlineCache`.
5. `Frame` and `Function`.
6. Interpreter loop with dispatch table.
7. `get_prop_ic` fast path.
8. `call` and tail call.
9. GC mark‑sweep.
10. Write barrier and generational GC.
11. Arrays (fast path).
12. Superinstructions.

---

## 7. Version History

| Version | Date     | Changes |
|---------|----------|---------|
| 2.0     | 2026-03  | Final version with complete bytecode list and clean Rust runtime core. |

---

**End of QJL Bytecode & Runtime Specification v2.0**