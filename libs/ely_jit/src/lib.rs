pub mod compiler;
pub mod engine;

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

        // Cast executable memory pointer to native function and invoke directly on CPU
        let native_func: extern "C" fn() -> f64 = unsafe { std::mem::transmute(code_ptr) };
        let result = native_func();

        assert_eq!(result, 35.0);
    }

    #[test]
    fn test_jit_function_with_parameters_and_locals() {
        let mut engine = JitEngine::new().expect("Failed to create JIT engine");
        let mut chunk = Chunk::new();

        // Computes: (a * 2.0) + (b - 3.0)
        // Parameter a: slot 1
        // Parameter b: slot 2
        // Local c: slot 3
        let c_two = chunk.add_constant(Value::number(2.0));
        let c_three = chunk.add_constant(Value::number(3.0));

        // c = a * 2.0; (SetLocal 3)
        chunk.write_opcode(Opcode::GetLocal, 1);
        chunk.write_byte(1, 1); // a
        chunk.write_opcode(Opcode::Constant, 1);
        chunk.write_byte(c_two as u8, 1);
        chunk.write_opcode(Opcode::Multiply, 1);
        chunk.write_opcode(Opcode::SetLocal, 1);
        chunk.write_byte(3, 1); // c
        chunk.write_opcode(Opcode::Pop, 1);

        // return c + (b - 3.0)
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

        // Invoke native function: (10.0 * 2.0) + (8.0 - 3.0) = 20.0 + 5.0 = 25.0
        let native_func: extern "C" fn(f64, f64) -> f64 = unsafe { std::mem::transmute(code_ptr) };
        let result = native_func(10.0, 8.0);

        assert_eq!(result, 25.0);
    }
}
