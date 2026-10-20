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
