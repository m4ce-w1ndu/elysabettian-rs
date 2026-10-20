/// Prints a zero-operand instruction and returns the offset of the next instruction.
fn simple_instruction(name: &str, offset: usize) -> usize {
    println!("{}", name);
    offset + 1
}
