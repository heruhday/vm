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
| 0  | Mov              | ABC    | `regs[A] = regs[B]` |
| 1  | LoadK            | ABx    | `regs[A] = constant[Bx]` |
| 2  | Add              | BC     | `acc = regs[B] + regs[C]` |
| 3  | GetPropIc        | ABC    | `regs[A] = regs[B][IC[C].key]` (IC slot C) |
| 4  | Call             | A B    | Call `regs[A]` with B args in `A+1..A+B`, result in acc |
| 5  | Jmp              | sBx    | Unconditional jump by signed 16‑bit offset |
| 6  | LoadI            | AsBx   | `regs[A] = (int16_t)sBx` |
| 7  | JmpTrue          | AsBx   | Jump if `regs[A]` truthy |
| 8  | JmpFalse         | AsBx   | Jump if `regs[A]` falsy |
| 9  | SetPropIc        | ABC    | `regs[B][IC[C].key] = regs[A]` (IC slot C) |
| 10 | AddAccImm8       | B      | `acc += (int8_t)B` |
| 11 | IncAcc           | –      | `acc++` |
| 12 | LoadThis         | –      | `acc = this` |
| 13 | Load0            | –      | `acc = 0` |
| 14 | Load1            | –      | `acc = 1` |
| 15 | Eq               | BC     | `acc = (regs[B] == regs[C])` (abstract equality) |
| 16 | Lt               | BC     | `acc = (regs[B] < regs[C])` |
| 17 | Lte              | BC     | `acc = (regs[B] <= regs[C])` |
| 18 | AddAcc           | B      | `acc += regs[B]` |
| 19 | SubAcc           | B      | `acc -= regs[B]` |
| 20 | MulAcc           | B      | `acc *= regs[B]` |
| 21 | DivAcc           | B      | `acc /= regs[B]` |
| 22 | LoadNull         | –      | `acc = null` |
| 23 | LoadTrue         | –      | `acc = true` |
| 24 | LoadFalse        | –      | `acc = false` |
| 25 | LoadGlobalIc     | ABx    | `regs[A] = global[IC[Bx]]` (global IC) |
| 26 | SetGlobalIc      | ABx    | `global[IC[Bx]] = regs[A]` |
| 27 | Typeof           | AB     | `regs[A] = interned_string(typeof regs[B])` |
| 28 | ToNum            | AB     | `regs[A] = ToNumber(regs[B])` |
| 29 | ToStr            | AB     | `regs[A] = ToString(regs[B])` |
| 30 | IsUndef          | AB     | `regs[A] = (regs[B] === undefined)` |
| 31 | IsNull           | AB     | `regs[A] = (regs[B] === null)` |

### 3.2 Arithmetic & Chaining (32–63)

| Op | Mnemonic         | Format | Description |
|----|------------------|--------|-------------|
| 32 | SubAccImm8       | B      | `acc -= (int8_t)B` |
| 33 | MulAccImm8       | B      | `acc *= (int8_t)B` |
| 34 | DivAccImm8       | B      | `acc /= (int8_t)B` |
| 35 | AddStrAcc        | B      | `acc += ToString(regs[B])` |
| 36 | AddI             | ABC    | `acc = regs[B] + (int8_t)C`; store to regs[A] if A≠255 |
| 37 | SubI             | ABC    | `acc = regs[B] - (int8_t)C` |
| 38 | MulI             | ABC    | `acc = regs[B] * (int8_t)C` |
| 39 | DivI             | ABC    | `acc = regs[B] / (int8_t)C` |
| 40 | ModI             | ABC    | `acc = regs[B] % (int8_t)C` |
| 41 | Neg              | B      | `acc = -regs[B]` |
| 42 | Inc              | B      | `acc = regs[B] + 1` |
| 43 | Dec              | B      | `acc = regs[B] - 1` |
| 44 | AddStr           | BC     | `acc = ToString(regs[B]) + ToString(regs[C])` |
| 45 | ToPrimitive      | B      | `acc = ToPrimitive(regs[B])` |
| 46 | GetPropAcc       | BC     | `acc = regs[B][regs[C]]` (property name in register) |
| 47 | SetPropAcc       | BC     | `regs[B][regs[C]] = acc` |
| 48 | GetIdxFast       | ABC    | `regs[A] = regs[B][regs[C]]` (fast array path) |
| 49 | SetIdxFast       | ABC    | `regs[B][regs[C]] = regs[A]` (fast array path) |
| 50 | LoadArg          | AB     | `regs[A] = (B < argc) ? args[B] : undefined` |
| 51 | LoadAcc          | A      | `acc = regs[A]` |
| 52 | StrictEq         | BC     | `acc = (regs[B] === regs[C])` |
| 53 | StrictNeq        | BC     | `acc = (regs[B] !== regs[C])` |
| 54 | BitAnd           | BC     | `acc = regs[B] & regs[C]` (bitwise AND) |
| 55 | BitOr            | BC     | `acc = regs[B] | regs[C]` (bitwise OR) |
| 56 | BitXor           | BC     | `acc = regs[B] ^ regs[C]` (bitwise XOR) |
| 57 | BitNot           | B      | `acc = ~regs[B]` (bitwise NOT) |
| 58 | Shl              | BC     | `acc = regs[B] << regs[C]` (left shift) |
| 59 | Shr              | BC     | `acc = regs[B] >> regs[C]` (signed right shift) |
| 60 | Ushr             | BC     | `acc = regs[B] >>> regs[C]` (unsigned right shift) |
| 61 | reserved         |        | |
| 62 | reserved         |        | |
| 63 | reserved         |        | |

### 3.3 Property & Object (64–95)

| Op | Mnemonic         | Format | Description |
|----|------------------|--------|-------------|
| 64 | GetLengthIc      | ABC    | `regs[A] = regs[B].length` (IC slot C) |
| 65 | ArrayPushAcc     | A      | Push `acc` onto array `regs[A]` |
| 66 | NewObj           | A      | `regs[A] = {}` |
| 67 | NewArr           | AB     | `regs[A] = []` (size hint B) |
| 68 | NewFunc          | ABx    | Create closure from descriptor at constant Bx, store in regs[A] |
| 69 | NewClass         | AB     | Create class with base `regs[B]`, store in regs[A] |
| 70 | GetProp          | ABC    | Slow fallback: `regs[A] = regs[B][C]` |
| 71 | SetProp          | ABC    | Slow fallback: `regs[B][C] = regs[A]` |
| 72 | GetIdxIc         | ABC    | Keyed get with IC slot C, result in regs[A] |
| 73 | SetIdxIc         | ABC    | Keyed set with IC slot C, store regs[A] |
| 74 | GetGlobal        | ABx    | Slow global load into regs[A] |
| 75 | SetGlobal        | ABx    | Slow global store from regs[A] |
| 76 | GetUpval         | AB     | Get upvalue at index B into regs[A] |
| 77 | SetUpval         | AB     | Set upvalue at index B to regs[A] |
| 78 | GetScope         | AB     | Get lexical scope at depth B into regs[A] |
| 79 | SetScope         | AB     | Set lexical scope at depth B to regs[A] |
| 80 | ResolveScope     | ABx    | Resolve identifier Bx → environment in regs[A] |
| 81 | GetSuper         | ABC    | Super property get into regs[A] |
| 82 | SetSuper         | ABC    | Super property set with regs[A] |
| 83 | DeleteProp       | ABC    | `regs[A] = delete regs[B][C]` |
| 84 | HasProp          | ABC    | `regs[A] = (C in regs[B])` |
| 85 | Keys             | AB     | `regs[A] = Object.keys(regs[B])` |
| 86 | ForIn            | AB     | Prepare for‑in: iterator in regs[A], first key in acc |
| 87 | IteratorNext     | A      | Get next value from iterator in regs[A], result in acc |
| 88 | Spread           | AB     | Spread elements from array regs[B] into array regs[A] |
| 89 | Destructure      | AB     | Destructure from source regs[B] into registers starting at A |
| 90 | CreateEnv        | A      | Create lexical environment, store in regs[A] |
| 91 | LoadName         | ABx    | Load variable by identifier constant Bx using scope, result in acc |
| 92 | StoreName        | ABx    | Store regs[A] into variable named by identifier constant Bx |
| 93 | LoadClosure      | AB     | Load captured value from closure at index B into regs[A] |
| 94 | NewThis          | A      | Allocate this object for constructor, store in regs[A] |
| 95 | TypeofName       | ABx    | `regs[A] = interned_string(typeof variable named Bx)` (no ReferenceError) |

### 3.4 Control Flow (96–127)

| Op | Mnemonic         | Format | Description |
|----|------------------|--------|-------------|
| 96 | JmpEq            | ABC    | Jump if `regs[A] == regs[B]` |
| 97 | JmpNeq           | ABC    | Jump if `regs[A] != regs[B]` |
| 98 | JmpLt            | ABC    | Jump if `regs[A] < regs[B]` |
| 99 | JmpLte           | ABC    | Jump if `regs[A] <= regs[B]` |
| 100| LoopIncJmp       | AsBx   | Increment `regs[A]`, jump if still < bound (used in for‑loops) |
| 101| Switch           | AB     | Jump table dispatch (B = table index) |
| 102| LoopHint         | –      | Hint for OSR |
| 103| Ret              | –      | Return accumulator to caller |
| 104| RetU             | –      | Return undefined |
| 105| TailCall         | A B    | Tail call (reuse current frame) |
| 106| Construct        | A B    | Constructor call |
| 107| CallVar          | A      | Call with spread (args in array at regs[A+1]) |
| 108| Enter            | uBx    | Allocate frame of size uBx |
| 109| Leave            | –      | Destroy frame |
| 110| Yield            | A      | Generator yield (value in regs[A]) |
| 111| Await            | A      | Async await |
| 112| Throw            | A      | Throw `regs[A]` |
| 113| Try              | sBx    | Start try (jump to catch) |
| 114| EndTry           | –      | End try |
| 115| Catch            | A      | Catch handler (store exception in regs[A]) |
| 116| Finally          | –      | Finally block start |
| 117| Pow              | BC     | `acc = regs[B] ** regs[C]` (exponentiation) |
| 118| LogicalAnd       | BC     | `acc = regs[B] && regs[C]` (logical AND) |
| 119| LogicalOr        | BC     | `acc = regs[B] || regs[C]` (logical OR) |
| 120| NullishCoalesce  | BC     | `acc = regs[B] ?? regs[C]` (nullish coalescing) |
| 121| In               | BC     | `acc = regs[C] in regs[B]` (in operator) |
| 122| Instanceof       | BC     | `acc = regs[B] instanceof regs[C]` (instanceof operator) |
| 123| reserved         |        | |
| 124| reserved         |        | |
| 125| reserved         |        | |
| 126| reserved         |        | |
| 127| reserved         |        | |

### 3.5 Call & Return (128–159)

| Op | Mnemonic         | Format | Description |
|----|------------------|--------|-------------|
| 128| CallIc           | A B    | Call with IC (method call) |
| 129| CallIcVar        | A      | Spread call with IC |
| 130| AddI32Fast       | BC     | Fast 32-bit integer addition: `acc = regs[B] + regs[C]` |
| 131| AddF64Fast       | BC     | Fast 64-bit float addition: `acc = regs[B] + regs[C]` |
| 132| SubI32Fast       | BC     | Fast 32-bit integer subtraction: `acc = regs[B] - regs[C]` |
| 133| MulI32Fast       | BC     | Fast 32-bit integer multiplication: `acc = regs[B] * regs[C]` |
| 134| EqI32Fast        | BC     | Fast 32-bit integer equality: `acc = (regs[B] == regs[C])` |
| 135| LtI32Fast        | BC     | Fast 32-bit integer less than: `acc = (regs[B] < regs[C])` |
| 136| JmpI32Fast       | AsBx   | Fast jump based on 32-bit integer condition |
| 137| GetPropMono      | ABC    | Monomorphic property get with IC slot C |
| 138| CallMono         | A B    | Monomorphic call with B arguments |
| 139| Call0            | A      | Call function in regs[A] with 0 arguments |
| 140| Call1            | AB     | Call function in regs[A] with 1 argument in regs[B] |
| 141| Call2            | ABC    | Call function in regs[A] with 2 arguments in regs[B], regs[C] |
| 142| Call3            | ABC    | Call function in regs[A] with 3 arguments in regs[B], regs[C], regs[D] |
| 143| CallMethod1      | AB     | Call method with 1 argument: `acc = regs[A].method(regs[B])` |
| 144| CallMethod2      | ABC    | Call method with 2 arguments: `acc = regs[A].method(regs[B], regs[C])` |
| 145| GetPropCall      | AB     | Get property and call: `acc = regs[A][B]()` |
| 146| CallRet          | A      | Call and return result: `acc = regs[A]()` |
| 147| AssertValue      | A      | Assert that regs[A] is truthy |
| 148| AssertOk         | –      | Assert success (no-op in production) |
| 149| AssertFail       | –      | Assert failure (throws in debug mode) |
| 150| AssertThrows     | A      | Assert that calling regs[A] throws |
| 151| AssertDoesNotThrow | A    | Assert that calling regs[A] does not throw |
| 152| AssertRejects    | A      | Assert that promise in regs[A] rejects |
| 153| AssertDoesNotReject | A  | Assert that promise in regs[A] does not reject |
| 154| AssertEqual      | BC     | Assert that `regs[B] == regs[C]` |
| 155| AssertNotEqual   | BC     | Assert that `regs[B] != regs[C]` |
| 156| AssertDeepEqual  | BC     | Assert deep equality of regs[B] and regs[C] |
| 157| AssertNotDeepEqual | BC  | Assert not deep equal of regs[B] and regs[C] |
| 158| AssertStrictEqual | BC   | Assert that `regs[B] === regs[C]` |
| 159| AssertNotStrictEqual | BC | Assert that `regs[B] !== regs[C]` |

### 3.6 Profiling / OSR / Feedback (160–199)

| Op | Mnemonic         | Format | Description |
|----|------------------|--------|-------------|
| 160| ProfileType      | BC     | Record type of register B in slot C |
| 161| ProfileCall      | BC     | Record call target B in slot C |
| 162| ProfileRet       | –      | Profile return |
| 163| CheckType        | BC     | Deopt if type of B not expected |
| 164| CheckStruct      | BC     | Deopt if structure ID mismatch |
| 165| CheckIc          | BC     | Verify IC slot C for object B |
| 166| IcInit           | AB     | Initialize IC slot A with struct of B |
| 167| IcUpdate         | AB     | Update IC slot A to new struct |
| 168| IcMiss           | A      | Handle IC miss |
| 169| OsrEntry         | –      | OSR entry point |
| 170| ProfileHotLoop   | –      | Record loop hotness |
| 171| OsrExit          | –      | Deoptimize to interpreter |
| 172| JitHint          | A      | Hint for JIT |
| 173| SafetyCheck      | A      | Runtime safety |
| 174| AssertDeepStrictEqual | BC | Assert deep strict equality of regs[B] and regs[C] |
| 175| AssertNotDeepStrictEqual | BC | Assert not deep strict equal of regs[B] and regs[C] |
| 176| LoadAdd          | ABC    | Load and add: `regs[A] = regs[B] + regs[C]` |
| 177| LoadSub          | ABC    | Load and subtract: `regs[A] = regs[B] - regs[C]` |
| 178| LoadMul          | ABC    | Load and multiply: `regs[A] = regs[B] * regs[C]` |
| 179| LoadInc          | ABC    | Load and increment: `regs[A] = regs[B] + 1` |
| 180| LoadDec          | ABC    | Load and decrement: `regs[A] = regs[B] - 1` |
| 181| LoadCmpEq        | ABC    | Load and compare equality: `regs[A] = (regs[B] == regs[C])` |
| 182| LoadCmpLt        | ABC    | Load and compare less than: `regs[A] = (regs[B] < regs[C])` |
| 183| LoadJfalse       | AsBx   | Load and jump if false: if not `regs[A]`, jump by sBx |
| 184| LoadCmpEqJfalse  | ABC    | Load, compare equality, and jump if false |
| 185| LoadCmpLtJfalse  | ABC    | Load, compare less than, and jump if false |
| 186| LoadGetProp      | ABC    | Load and get property: `regs[A] = regs[B][C]` |
| 187| LoadGetPropCmpEq | ABC    | Load, get property, and compare equality |
| 188| GetProp2Ic       | ABC    | Get property with 2-level IC chain |
| 189| GetProp3Ic       | ABC    | Get property with 3-level IC chain |
| 190| GetElem          | ABC    | Get element: `regs[A] = regs[B][regs[C]]` |
| 191| SetElem          | ABC    | Set element: `regs[B][regs[C]] = regs[A]` |
| 192| GetPropElem      | ABC    | Get property element: `regs[A] = regs[B][C][D]` |
| 193| CallMethodIc     | ABC    | Call method with IC: `acc = regs[A].method(regs[B], regs[C])` |
| 194| CallMethod2Ic    | ABC    | Call method with 2-level IC |
| 195| reserved         |        | |
| 196| reserved         |        | |
| 197| reserved         |        | |
| 198| reserved         |        | |
| 199| reserved         |        | |

### 3.7 Superinstructions (200–255)

| Op | Mnemonic                     | Format | Fused Instructions |
|----|------------------------------|--------|---------------------|
| 200| GetPropIcCall                | ABC    | `GetPropIc + Call` |
| 201| IncJmpFalseLoop              | AsBx   | `IncAcc + JmpFalse` |
| 202| LoadKAddAcc                  | Bx     | `LoadK + AddAcc` |
| 203| AddMov                       | ABC    | `Add + Mov` |
| 204| EqJmpTrue                    | BC sBx | `Eq + JmpTrue` |
| 205| GetPropAccCall               | BC     | `GetPropAcc + Call` |
| 206| LoadKMulAcc                  | Bx     | `LoadK + MulAcc` |
| 207| LtJmp                        | BC sBx | `Lt + Jmp` |
| 208| GetPropIcMov                 | ABC    | `GetPropIc + Mov` |
| 209| GetPropAddImmSetPropIc       | AB C   | `GetPropIc + AddAccImm8 + SetPropIc` |
| 210| AddAccImm8Mov                | B A    | `AddAccImm8 + Mov` |
| 211| CallIcSuper                  | A B    | `GetPropIc + Call` (monomorphic) |
| 212| LoadThisCall                 | B      | `LoadThis + Call` |
| 213| EqJmpFalse                   | BC sBx | `Eq + JmpFalse` |
| 214| LoadKSubAcc                  | Bx     | `LoadK + SubAcc` |
| 215| GetLengthIcCall              | B      | `GetLengthIc + Call` |
| 216| AddStrAccMov                 | B A    | `AddStrAcc + Mov` |
| 217| IncAccJmp                    | sBx    | `IncAcc + Jmp` |
| 218| GetPropChainAcc              | B C    | `GetPropAcc + GetPropAcc` (chained: `acc = regs[regs[B]][C]`) |
| 219| TestJmpTrue                  | A sBx  | `Test + JmpTrue` |
| 220| LoadArgCall                  | A B    | `LoadArg + Call` |
| 221| MulAccMov                    | B A    | `MulAcc + Mov` |
| 222| LteJmpLoop                   | BC sBx | `Lte + Jmp` |
| 223| NewObjInitProp               | ABC    | `NewObj + InitProp` |
| 224| ProfileHotCall               | B C    | `Call + ProfileCall` |
| 225| Call1SubI                    | ABC    | `Call1 + SubI` |
| 226| JmpLteFalse                  | ABC    | `Jmp + Lte + False` |
| 227| RetReg                       | A      | `Ret + Reg` |
| 228| AddI32                       | ABC    | `Add + I32` |
| 229| AddF64                       | ABC    | `Add + F64` |
| 230| SubI32                       | ABC    | `Sub + I32` |
| 231| SubF64                       | ABC    | `Sub + F64` |
| 232| MulI32                       | ABC    | `Mul + I32` |
| 233| MulF64                       | ABC    | `Mul + F64` |
| 234| RetIfLteI                    | ABC    | `Ret + If + Lte + I` |
| 235| AddAccReg                    | AB     | `AddAcc + Reg` |
| 236| Call1Add                     | ABC    | `Call1 + Add` |
| 237| Call2Add                     | ABC    | `Call2 + Add` |
| 238| LoadKAdd                     | ABx    | `LoadK + Add` |
| 239| LoadKCmp                     | ABx    | `LoadK + Cmp` |
| 240| CmpJmp                       | ABC    | `Cmp + Jmp` |
| 241| reserved                     |        | |
| 242| reserved                     |        | |
| 243| reserved                     |        | |
| 244| reserved                     |        | |
| 245| reserved                     |        | |
| 246| reserved                     |        | |
| 247| reserved                     |        | |
| 248| reserved                     |        | |
| 249| reserved                     |        | |
| 250| reserved                     |        | |
| 251| reserved                     |        | |
| 252| reserved                     |        | |
| 253| reserved                     |        | |
| 254| reserved                     |        | |
| 255| reserved                     |        | |

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