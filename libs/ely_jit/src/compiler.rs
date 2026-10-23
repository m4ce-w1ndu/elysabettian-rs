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
        self.compile_function(chunk, 0, name)
    }

    /// Compiles a function with `arity` parameters and local variables into native code.
    pub fn compile_function(
        &mut self,
        chunk: &Chunk,
        arity: usize,
        name: &str,
    ) -> Result<*const u8, String> {
        // Define signature with `arity` f64 parameters, returning a native f64
        let mut sig = self.engine.module.make_signature();
        for _ in 0..arity {
            sig.params.push(AbiParam::new(types::F64));
        }
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

            // Track maximum local slot accessed to pre-declare SSA variables
            let mut max_local = arity;
            let mut scan_ip = 0;
            while scan_ip < chunk.count() {
                if let Ok(op) = Opcode::try_from(chunk.code[scan_ip]) {
                    scan_ip += 1;
                    match op {
                        Opcode::GetLocal | Opcode::SetLocal => {
                            let slot = chunk.code[scan_ip] as usize;
                            scan_ip += 1;
                            if slot > max_local {
                                max_local = slot;
                            }
                        }
                        Opcode::Constant => scan_ip += 1,
                        Opcode::Jump | Opcode::JumpIfFalse | Opcode::Loop => scan_ip += 2,
                        _ => {}
                    }
                } else {
                    scan_ip += 1;
                }
            }

            // Declare Cranelift SSA variables for each local slot (registers)
            let mut vars = Vec::with_capacity(max_local + 1);
            for i in 0..=max_local {
                let var = Variable::new(i);
                builder.declare_var(var, types::F64);
                vars.push(var);
            }

            // Bind incoming function arguments to the corresponding local slots
            // Slot 0 in Elysabettian convention is reserved for callee/receiver
            // Function parameters start at slot 1 ..= arity
            let params = builder.block_params(entry_block).to_vec();
            for (idx, &param_val) in params.iter().enumerate() {
                let slot = idx + 1;
                if slot < vars.len() {
                    builder.def_var(vars[slot], param_val);
                }
            }

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
                    Opcode::GetLocal => {
                        let slot = chunk.code[ip] as usize;
                        ip += 1;
                        let val = builder.use_var(vars[slot]);
                        eval_stack.push(val);
                    }
                    Opcode::SetLocal => {
                        let slot = chunk.code[ip] as usize;
                        ip += 1;
                        let val = *eval_stack.last().ok_or("Stack underflow in SetLocal")?;
                        builder.def_var(vars[slot], val);
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
                    Opcode::Pop | Opcode::Nop => {}
                    _ => {
                        return Err(format!("Unsupported opcode for JIT: {:?}", op));
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
