# Superinstruction Reference

## 🧮 Arithmetic Superinstructions

### 1. LOAD_ADD A, B
**Meaning**: `ACC = R[A] + R[B]`
**Example JS**: `x + y`
**Opcode**: 61 (0x3D)

### 2. LOAD_SUB A, B
**Meaning**: `ACC = R[A] - R[B]`
**Example JS**: `x - y`
**Opcode**: 62 (0x3E)

### 3. LOAD_MUL A, B
**Meaning**: `ACC = R[A] * R[B]`
**Example JS**: `x * y`
**Opcode**: 63 (0x3F)

### 4. LOAD_INC A
**Meaning**: `ACC = R[A] + 1`
**Example JS**: `i++`
**Opcode**: 123 (0x7B)

### 5. LOAD_DEC A
**Meaning**: `ACC = R[A] - 1`
**Example JS**: `i--`
**Opcode**: 124 (0x7C)

## 🔍 Comparison + Branching Superinstructions

### 6. LOAD_CMP_EQ A, B
**Meaning**: `ACC = (R[A] == R[B])`
**Example JS**: `a === b`
**Opcode**: 125 (0x7D)

### 7. LOAD_CMP_LT A, B
**Meaning**: `ACC = (R[A] < R[B])`
**Example JS**: `i < len`
**Opcode**: 126 (0x7E)

### 8. LOAD_JFALSE A, off
**Meaning**: `if (!R[A]) PC += off`
**Example JS**: `if (!flag) { ... }`
**Opcode**: 127 (0x7F)

### 9. LOAD_CMP_EQ_JFALSE A, B, off
**Meaning**: `if (!(R[A] == R[B])) PC += off`
**Example JS**: `if (a !== b) { ... }`
**Opcode**: 176 (0xB0)

### 10. LOAD_CMP_LT_JFALSE A, B, off
**Meaning**: `if (!(R[A] < R[B])) PC += off`
**Example JS**: `if (!(i < n)) break`
**Opcode**: 177 (0xB1)

## 🏷️ Property Access Superinstructions

### 11. LOAD_GETPROP A, prop
**Meaning**: `ACC = R[A][prop]`
**Example JS**: `obj.x`
**Opcode**: 178 (0xB2)

### 12. LOAD_GETPROP_CMP_EQ A, prop, B
**Meaning**: `ACC = (R[A][prop] == R[B])`
**Example JS**: `obj