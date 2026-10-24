use crate::{chunk::Chunk, opcode::Opcode};

/// Prints a zero-operand instruction and returns the offset of the next instruction.
fn simple_instruction(name: &str, offset: usize) -> usize {
    println!("{}", name);
    offset + 1
}

/// Prints an instruction that references a constant from the constant pool.
fn constant_instruction(name: &str, chunk: &Chunk, offset: usize) -> usize {
    let constant_idx = chunk.code[offset + 1];
    let value_str = chunk
        .get_constant(constant_idx as usize)
        .map(|v| v.to_string())
        .unwrap_or_else(|| "<unknown>".to_string());

    println!("{:<16} {:4} '{}'", name, constant_idx, value_str);
    offset + 2
}

/// Prints an instruction with a single 1-byte operand (such as stack slots or argument counts).
fn byte_instruction(name: &str, chunk: &Chunk, offset: usize) -> usize {
    let slot = chunk.code[offset + 1];
    println!("{:<16} {:4}", name, slot);
    offset + 2
}

/// Prints a 16-bit jump instruction with relative offset and computed target address.
fn jump_instruction(name: &str, sign: i32, chunk: &Chunk, offset: usize) -> usize {
    let jump = ((chunk.code[offset + 1] as u16) << 8) | (chunk.code[offset + 2] as u16);
    let target = (offset as isize + 3 + (sign as isize * jump as isize)) as usize;
    println!("{:<16} {:4} -> {}", name, offset, target);
    offset + 3
}

/// Disassembles a single instruction at the specified bytecode offset.
///
/// Prints the byte offset, source line information, instruction name, and operand details,
/// returning the bytecode offset of the subsequent instruction.
pub fn disassemble_instruction(chunk: &Chunk, offset: usize) -> usize {
    print!("{:04} ", offset);

    let line = chunk.get_line(offset);
    if offset > 0 && line == chunk.get_line(offset - 1) {
        print!("   | ");
    } else {
        print!("{:4} ", line);
    }

    let instruction = match chunk.code.get(offset) {
        Some(&byte) => byte,
        None => {
            println!("Unexpected end of chunk at offset {}", offset);
            return offset + 1;
        }
    };

    let opcode = match Opcode::try_from(instruction) {
        Ok(op) => op,
        Err(unknown) => {
            println!("Unknown opcode {:#04x}", unknown);
            return offset + 1;
        }
    };

    match opcode {
        Opcode::Constant => constant_instruction("const", chunk, offset),
        Opcode::Nop => simple_instruction("nop", offset),
        Opcode::True => simple_instruction("true", offset),
        Opcode::False => simple_instruction("false", offset),
        Opcode::Pop => simple_instruction("pop", offset),
        Opcode::GetLocal => byte_instruction("getlcl", chunk, offset),
        Opcode::GetGlobal => constant_instruction("getglb", chunk, offset),
        Opcode::DefineGlobal => constant_instruction("defglb", chunk, offset),
        Opcode::SetLocal => byte_instruction("setlcl", chunk, offset),
        Opcode::SetGlobal => constant_instruction("setglb", chunk, offset),
        Opcode::GetUpvalue => byte_instruction("getupv", chunk, offset),
        Opcode::SetUpvalue => byte_instruction("setupv", chunk, offset),
        Opcode::GetProperty => constant_instruction("getprp", chunk, offset),
        Opcode::SetProperty => constant_instruction("setprp", chunk, offset),
        Opcode::GetSuper => constant_instruction("getsup", chunk, offset),
        Opcode::Equal => simple_instruction("eq", offset),
        Opcode::Greater => simple_instruction("gt", offset),
        Opcode::Less => simple_instruction("lt", offset),
        Opcode::Add => simple_instruction("add", offset),
        Opcode::Subtract => simple_instruction("sub", offset),
        Opcode::Multiply => simple_instruction("mul", offset),
        Opcode::Divide => simple_instruction("div", offset),
        Opcode::Not => simple_instruction("not", offset),
        Opcode::Negate => simple_instruction("neg", offset),
        Opcode::Print => simple_instruction("print", offset),
        Opcode::Jump => jump_instruction("jmp", 1, chunk, offset),
        Opcode::JumpIfFalse => jump_instruction("jz", 1, chunk, offset),
        Opcode::Loop => jump_instruction("iter", -1, chunk, offset),
        Opcode::Call => byte_instruction("call", chunk, offset),
        Opcode::Invoke => {
            // Fused method lookup + call: takes [const_name: u8, argc: u8]
            let name_idx = chunk.code.get(offset + 1).copied().unwrap_or(0);
            let argc = chunk.code.get(offset + 2).copied().unwrap_or(0);
            let value_str = chunk
                .get_constant(name_idx as usize)
                .map(|v| v.to_string())
                .unwrap_or_else(|| "<unknown>".to_string());
            println!(
                "{:<16} ({argc} args) {:4} '{}'",
                "invk", name_idx, value_str
            );
            offset + 3
        }
        Opcode::SuperInvoke => {
            let name_idx = chunk.code.get(offset + 1).copied().unwrap_or(0);
            let argc = chunk.code.get(offset + 2).copied().unwrap_or(0);
            let value_str = chunk
                .get_constant(name_idx as usize)
                .map(|v| v.to_string())
                .unwrap_or_else(|| "<unknown>".to_string());
            println!(
                "{:<16} ({argc} args) {:4} '{}'",
                "sinvk", name_idx, value_str
            );
            offset + 3
        }
        Opcode::Closure => {
            // Closure encoding: [const_idx: u8, upvalues...]
            let mut curr = offset + 1;
            let const_idx = chunk.code.get(curr).copied().unwrap_or(0);
            let value_str = chunk
                .get_constant(const_idx as usize)
                .map(|v| v.to_string())
                .unwrap_or_else(|| "<unknown>".to_string());
            println!("{:<16} {:4} '{}'", "closure", const_idx, value_str);
            curr += 1;

            // Notice: Full upvalue operand decoding will be wired once Function object is defined in core
            // For now advance past the closure constant
            curr
        }
        Opcode::CloseUpvalue => simple_instruction("close_upv", offset),
        Opcode::Return => simple_instruction("ret", offset),
        Opcode::Class => constant_instruction("class", chunk, offset),
        Opcode::Inherit => simple_instruction("inherit", offset),
        Opcode::Method => constant_instruction("method", chunk, offset),
        Opcode::BwAnd => simple_instruction("and", offset),
        Opcode::BwOr => simple_instruction("or", offset),
        Opcode::BwXor => simple_instruction("xor", offset),
        Opcode::BwNot => simple_instruction("bwnot", offset),
        Opcode::ShiftLeft => simple_instruction("shl", offset),
        Opcode::ShiftRight => simple_instruction("shr", offset),
        Opcode::ArrayBuild => byte_instruction("arrbld", chunk, offset),
        Opcode::ArrayIndex => simple_instruction("arridx", offset),
        Opcode::ArrayStore => simple_instruction("arrstr", offset),
        Opcode::Import => byte_instruction("import", chunk, offset),
    }
}

/// Disassembles all instructions contained in a `Chunk` with a header label.
pub fn disassemble_chunk(chunk: &Chunk, name: &str) {
    println!("== {} ==", name);
    let mut offset = 0;
    while offset < chunk.count() {
        offset = disassemble_instruction(chunk, offset);
    }
}
