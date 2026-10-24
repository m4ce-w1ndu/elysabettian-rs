pub mod compiler;
pub mod engine;
pub mod runtime;

#[cfg(test)]
mod tests {
    use super::compiler::JitCompiler;
    use super::engine::JitEngine;
    use ely_core::chunk::Chunk;
    use ely_core::opcode::Opcode;
    use ely_core::value::Value;

    #[test]
    fn test_jit_arithmetic_execution() {
        let mut engine = JitEngine::new().expect("Failed to create JIT engine");
        let mut chunk = Chunk::new();

        // Bytecode for: (15.5 + 4.5) * 2.0 - 10.0 / 2.0 = 20.0 * 2.0 - 5.0 = 35.0
        let c1 = chunk.add_constant(Value::number(15.5));
        let c2 = chunk.add_constant(Value::number(4.5));
        let c3 = chunk.add_constant(Value::number(2.0));
        let c4 = chunk.add_constant(Value::number(10.0));
        let c5 = chunk.add_constant(Value::number(2.0));

        chunk.write_opcode(Opcode::Constant, 1);
        chunk.write_byte(c1 as u8, 1);
        chunk.write_opcode(Opcode::Constant, 1);
        chunk.write_byte(c2 as u8, 1);
        chunk.write_opcode(Opcode::Add, 1);

        chunk.write_opcode(Opcode::Constant, 1);
        chunk.write_byte(c3 as u8, 1);
        chunk.write_opcode(Opcode::Multiply, 1);

        chunk.write_opcode(Opcode::Constant, 1);
        chunk.write_byte(c4 as u8, 1);
        chunk.write_opcode(Opcode::Constant, 1);
        chunk.write_byte(c5 as u8, 1);
        chunk.write_opcode(Opcode::Divide, 1);

        chunk.write_opcode(Opcode::Subtract, 1);
        chunk.write_opcode(Opcode::Return, 1);

        let mut compiler = JitCompiler::new(&mut engine);
        let code_ptr = compiler
            .compile_arithmetic(&chunk, "test_math")
            .expect("JIT compilation failed");

        let native_func: extern "C" fn() -> f64 = unsafe { std::mem::transmute(code_ptr) };
        let result = native_func();

        assert_eq!(result, 35.0);
    }

    #[test]
    fn test_jit_function_with_parameters_and_locals() {
        let mut engine = JitEngine::new().expect("Failed to create JIT engine");
        let mut chunk = Chunk::new();

        // Computes: (a * 2.0) + (b - 3.0)
        let c_two = chunk.add_constant(Value::number(2.0));
        let c_three = chunk.add_constant(Value::number(3.0));

        chunk.write_opcode(Opcode::GetLocal, 1);
        chunk.write_byte(1, 1); // a
        chunk.write_opcode(Opcode::Constant, 1);
        chunk.write_byte(c_two as u8, 1);
        chunk.write_opcode(Opcode::Multiply, 1);
        chunk.write_opcode(Opcode::SetLocal, 1);
        chunk.write_byte(3, 1); // c
        chunk.write_opcode(Opcode::Pop, 1);

        chunk.write_opcode(Opcode::GetLocal, 1);
        chunk.write_byte(3, 1); // c
        chunk.write_opcode(Opcode::GetLocal, 1);
        chunk.write_byte(2, 1); // b
        chunk.write_opcode(Opcode::Constant, 1);
        chunk.write_byte(c_three as u8, 1);
        chunk.write_opcode(Opcode::Subtract, 1);
        chunk.write_opcode(Opcode::Add, 1);
        chunk.write_opcode(Opcode::Return, 1);

        let mut compiler = JitCompiler::new(&mut engine);
        let code_ptr = compiler
            .compile_function(&chunk, 2, "test_params_locals")
            .expect("JIT compilation failed");

        let native_func: extern "C" fn(f64, f64) -> f64 = unsafe { std::mem::transmute(code_ptr) };
        let result = native_func(10.0, 8.0);

        assert_eq!(result, 25.0);
    }

    #[test]
    fn test_jit_loop_and_conditionals() {
        let mut engine = JitEngine::new().expect("Failed to create JIT engine");
        let mut chunk = Chunk::new();

        let c_zero = chunk.add_constant(Value::number(0.0));
        let c_one = chunk.add_constant(Value::number(1.0));

        // acc = 0.0 (slot 2)
        chunk.write_opcode(Opcode::Constant, 1);
        chunk.write_byte(c_zero as u8, 1);
        chunk.write_opcode(Opcode::SetLocal, 1);
        chunk.write_byte(2, 1);
        chunk.write_opcode(Opcode::Pop, 1);

        // i = 1.0 (slot 3)
        chunk.write_opcode(Opcode::Constant, 1);
        chunk.write_byte(c_one as u8, 1);
        chunk.write_opcode(Opcode::SetLocal, 1);
        chunk.write_byte(3, 1);
        chunk.write_opcode(Opcode::Pop, 1);

        // Loop header
        let loop_start = chunk.count();

        // Condition: (i < n)
        chunk.write_opcode(Opcode::GetLocal, 1);
        chunk.write_byte(3, 1); // i
        chunk.write_opcode(Opcode::GetLocal, 1);
        chunk.write_byte(1, 1); // n
        chunk.write_opcode(Opcode::Less, 1); // (i < n)
        chunk.write_opcode(Opcode::JumpIfFalse, 1);
        let exit_jump_idx = chunk.count();
        chunk.write_byte(0xff, 1);
        chunk.write_byte(0xff, 1);
        chunk.write_opcode(Opcode::Pop, 1);

        // Body: acc = acc + i
        chunk.write_opcode(Opcode::GetLocal, 1);
        chunk.write_byte(2, 1); // acc
        chunk.write_opcode(Opcode::GetLocal, 1);
        chunk.write_byte(3, 1); // i
        chunk.write_opcode(Opcode::Add, 1);
        chunk.write_opcode(Opcode::SetLocal, 1);
        chunk.write_byte(2, 1);
        chunk.write_opcode(Opcode::Pop, 1);

        // Increment: i = i + 1.0
        chunk.write_opcode(Opcode::GetLocal, 1);
        chunk.write_byte(3, 1); // i
        chunk.write_opcode(Opcode::Constant, 1);
        chunk.write_byte(c_one as u8, 1);
        chunk.write_opcode(Opcode::Add, 1);
        chunk.write_opcode(Opcode::SetLocal, 1);
        chunk.write_byte(3, 1);
        chunk.write_opcode(Opcode::Pop, 1);

        // Loop back to loop_start
        chunk.write_opcode(Opcode::Loop, 1);
        let loop_offset = chunk.count() + 2 - loop_start;
        chunk.write_byte(((loop_offset >> 8) & 0xff) as u8, 1);
        chunk.write_byte((loop_offset & 0xff) as u8, 1);

        // Loop exit target: patch JumpIfFalse
        let exit_target = chunk.count();
        let forward_offset = exit_target - exit_jump_idx - 2;
        chunk.code[exit_jump_idx] = ((forward_offset >> 8) & 0xff) as u8;
        chunk.code[exit_jump_idx + 1] = (forward_offset & 0xff) as u8;

        // return acc
        chunk.write_opcode(Opcode::Pop, 1);
        chunk.write_opcode(Opcode::GetLocal, 1);
        chunk.write_byte(2, 1);
        chunk.write_opcode(Opcode::Return, 1);

        let mut compiler = JitCompiler::new(&mut engine);
        let code_ptr = compiler
            .compile_function(&chunk, 1, "test_loop_sum")
            .expect("JIT compilation failed");

        let native_func: extern "C" fn(f64) -> f64 = unsafe { std::mem::transmute(code_ptr) };
        let sum_10 = native_func(11.0);
        let sum_100 = native_func(101.0);

        assert_eq!(sum_10, 55.0);
        assert_eq!(sum_100, 5050.0);
    }

    #[test]
    fn test_jit_bitwise_execution() {
        let mut engine = JitEngine::new().expect("Failed to create JIT engine");
        let mut chunk = Chunk::new();

        let c_a = chunk.add_constant(Value::number(12.0)); // 0b1100
        let c_b = chunk.add_constant(Value::number(10.0)); // 0b1010
        let c_shift = chunk.add_constant(Value::number(2.0));

        // (12 & 10) -> 8
        chunk.write_opcode(Opcode::Constant, 1);
        chunk.write_byte(c_a as u8, 1);
        chunk.write_opcode(Opcode::Constant, 1);
        chunk.write_byte(c_b as u8, 1);
        chunk.write_opcode(Opcode::BwAnd, 1);

        // (8 << 2) -> 32
        chunk.write_opcode(Opcode::Constant, 1);
        chunk.write_byte(c_shift as u8, 1);
        chunk.write_opcode(Opcode::ShiftLeft, 1);

        chunk.write_opcode(Opcode::Return, 1);

        let mut compiler = JitCompiler::new(&mut engine);
        let code_ptr = compiler
            .compile_function(&chunk, 0, "test_bitwise")
            .expect("JIT compilation failed");

        let native_func: extern "C" fn() -> f64 = unsafe { std::mem::transmute(code_ptr) };
        assert_eq!(native_func(), 32.0);
    }

    #[test]
    fn test_jit_dead_code_after_return() {
        let mut engine = JitEngine::new().expect("Failed to create JIT engine");
        let mut chunk = Chunk::new();

        let c_val = chunk.add_constant(Value::number(42.0));
        let c_dead = chunk.add_constant(Value::number(999.0));

        chunk.write_opcode(Opcode::Constant, 1);
        chunk.write_byte(c_val as u8, 1);
        chunk.write_opcode(Opcode::Return, 1);

        // Dead code after return: should be safely ignored by JIT CFG lowering
        chunk.write_opcode(Opcode::Constant, 1);
        chunk.write_byte(c_dead as u8, 1);
        chunk.write_opcode(Opcode::Nop, 1);
        chunk.write_opcode(Opcode::Return, 1);

        let mut compiler = JitCompiler::new(&mut engine);
        let code_ptr = compiler
            .compile_function(&chunk, 0, "test_dead_code")
            .expect("JIT compilation failed on dead code");

        let native_func: extern "C" fn() -> f64 = unsafe { std::mem::transmute(code_ptr) };
        assert_eq!(native_func(), 42.0);
    }
}
