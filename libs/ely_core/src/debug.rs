use crate::chunk::Chunk;

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
