fn main() {
    let insn: u32 = 0x0602000A;
    println!("Instruction: 0x{:08X}", insn);
    println!("Binary: {:032b}", insn);
    
    // Bits:
    // 0-7:   bits 0-7
    // 8-15:  bits 8-15  
    // 16-23: bits 16-23
    // 24-31: bits 24-31
    
    let opcode = (insn & 0xFF) as u8;
    let a = ((insn >> 8) & 0xFF) as u8;
    let b = ((insn >> 16) & 0xFF) as u8;
    let c = ((insn >> 24) & 0xFF) as u8;
    
    println!("opcode: 0x{:02X} ({})", opcode, opcode);
    println!("a: 0x{:02X} ({})", a, a);
    println!("b: 0x{:02X} ({})", b, b);
    println!("c: 0x{:02X} ({})", c, c);
    
    // decode_asbx: ((insn >> 16) & 0xFFFF) as u16 as i16
    let decode_asbx = ((insn >> 16) & 0xFFFF) as u16 as i16;
    println!("decode_asbx: 0x{:04X} ({})", decode_asbx as u16, decode_asbx);
    
    // Alternative: (b << 8) | c
    let bc = ((b as u16) << 8) | (c as u16);
    println!("(b << 8) | c: 0x{:04X} ({})", bc, bc as i16);
    
    // Test AddI32 encoding: 0xF3010203
    println!("\nAddI32 instruction: 0xF3010203");
    let insn2: u32 = 0xF3010203;
    let opcode2 = (insn2 & 0xFF) as u8;
    let a2 = ((insn2 >> 8) & 0xFF) as u8;
    let b2 = ((insn2 >> 16) & 0xFF) as u8;
    let c2 = ((insn2 >> 24) & 0xFF) as u8;
    
    println!("opcode: 0x{:02X} ({})", opcode2, opcode2);
    println!("a: 0x{:02X} ({})", a2, a2);
    println!("b: 0x{:02X} ({})", b2, b2);
    println!("c: 0x{:02X} ({})", c2, c2);
}