use cranelift::prelude::*;
use cranelift_module::{Linkage, Module};
use ely_core::chunk::Chunk;
use ely_core::opcode::Opcode;

use crate::engine::JitEngine;

/// Compiles Elysabettian bytecode chunks into native machine code functions using Cranelift JIT.
pub struct JitCompiler<'a> {
    engine: &'a mut JitEngine,
}

impl<'a> JitCompiler<'a> {
    /// Initializes a new JIT compiler attached to the provided JIT engine instance.
    pub fn new(engine: &'a mut JitEngine) -> Self {
        Self { engine }
    }

    /// Compiles an arithmetic bytecode chunk into a native `extern "C" fn() -> f64` executable function.
    pub fn compile_arithmetic(&mut self, chunk: &Chunk, name: &str) -> Result<*const u8, String> {
        // Define signature: no arguments, returns a native 64-bit float
        let mut sig = self.engine.module.make_signature();
        sig.returns.push(AbiParam::new(types::F64));

        // Declare the function symbol in the JIT module
        let func_id = self
            .engine
            .module
            .declare_function(name, Linkage::Export, &sig)
            .map_err(|e| e.to_string())?;

        // Set signature in codegen context
        self.engine.ctx.func.signature = sig;

        {
            let mut builder = FunctionBuilder::new(
                &mut self.engine.ctx.func,
                &mut self.engine.builder_context,
            );

            // Create and configure entry block
            let entry_block = builder.create_block();
            builder.append_block_params_for_function_params(entry_block);
            builder.switch_to_block(entry_block);
            builder.seal_block(entry_block);

            // Compile-time abstract evaluation stack of Cranelift SSA values
            let mut eval_stack: Vec<Value> = Vec::new();

            let mut ip = 0;
            while ip < chunk.count() {
                let op = Opcode::try_from(chunk.code[ip])
                    .map_err(|_| format!("Invalid opcode byte at offset {}", ip))?;
                ip += 1;

                match op {
                    Opcode::Constant => {
                        let const_idx = chunk.code[ip] as usize;
                        ip += 1;
                        let const_val = chunk
                            .get_constant(const_idx)
                            .ok_or("Constant not found in chunk pool")?;
                        let val = builder.ins().f64const(const_val.as_number());
                        eval_stack.push(val);
                    }
                    Opcode::Add => {
                        let b = eval_stack.pop().ok_or("Stack underflow in Opcode::Add")?;
                        let a = eval_stack.pop().ok_or("Stack underflow in Opcode::Add")?;
                        eval_stack.push(builder.ins().fadd(a, b));
                    }
                    Opcode::Subtract => {
                        let b = eval_stack
                            .pop()
                            .ok_or("Stack underflow in Opcode::Subtract")?;
                        let a = eval_stack
                            .pop()
                            .ok_or("Stack underflow in Opcode::Subtract")?;
                        eval_stack.push(builder.ins().fsub(a, b));
                    }
                    Opcode::Multiply => {
                        let b = eval_stack
                            .pop()
                            .ok_or("Stack underflow in Opcode::Multiply")?;
                        let a = eval_stack
                            .pop()
                            .ok_or("Stack underflow in Opcode::Multiply")?;
                        eval_stack.push(builder.ins().fmul(a, b));
                    }
                    Opcode::Divide => {
                        let b = eval_stack
                            .pop()
                            .ok_or("Stack underflow in Opcode::Divide")?;
                        let a = eval_stack
                            .pop()
                            .ok_or("Stack underflow in Opcode::Divide")?;
                        eval_stack.push(builder.ins().fdiv(a, b));
                    }
                    Opcode::Negate => {
                        let a = eval_stack
                            .pop()
                            .ok_or("Stack underflow in Opcode::Negate")?;
                        eval_stack.push(builder.ins().fneg(a));
                    }
                    Opcode::Return => {
                        let res = eval_stack
                            .pop()
                            .ok_or("Stack underflow in Opcode::Return")?;
                        builder.ins().return_(&[res]);
                        break;
                    }
                    Opcode::Pop | Opcode::Nop => {
                        // Allow trailing pop or nop instructions before return
                    }
                    _ => {
                        return Err(format!("Unsupported opcode for arithmetic JIT: {:?}", op));
                    }
                }
            }

            builder.finalize();
        }

        // Define function body in the JIT module and finalize code emission
        self.engine
            .module
            .define_function(func_id, &mut self.engine.ctx)
            .map_err(|e| e.to_string())?;

        self.engine.module.clear_context(&mut self.engine.ctx);
        self.engine
            .module
            .finalize_definitions()
            .map_err(|e| e.to_string())?;

        // Retrieve the executable machine code function pointer
        let code_ptr = self.engine.module.get_finalized_function(func_id);
        Ok(code_ptr)
    }
}
