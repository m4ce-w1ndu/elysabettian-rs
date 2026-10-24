use std::collections::{BTreeSet, HashMap};

use cranelift::prelude::*;
use cranelift_module::{Linkage, Module};
use ely_core::chunk::Chunk;
use ely_core::opcode::Opcode;
use ely_core::value::Value as ElyValue;

use crate::engine::JitEngine;

/// Compiles Elysabettian bytecode chunks into native machine code functions using Cranelift JIT.
pub struct JitCompiler<'a> {
    engine: &'a mut JitEngine,
    pub dump_ir: bool,
    pub dump_asm: bool,
}

impl<'a> JitCompiler<'a> {
    /// Initializes a new JIT compiler attached to the provided JIT engine instance.
    pub fn new(engine: &'a mut JitEngine) -> Self {
        Self {
            engine,
            dump_ir: false,
            dump_asm: false,
        }
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

        // Declare runtime helper function signatures
        let mut set_global_sig = self.engine.module.make_signature();
        set_global_sig.params.push(AbiParam::new(types::I32)); // id
        set_global_sig.params.push(AbiParam::new(types::I64)); // val_bits
        let set_global_id = self
            .engine
            .module
            .declare_function("ely_jit_set_global", Linkage::Import, &set_global_sig)
            .map_err(|e| e.to_string())?;

        let mut get_global_sig = self.engine.module.make_signature();
        get_global_sig.params.push(AbiParam::new(types::I32)); // id
        get_global_sig.returns.push(AbiParam::new(types::I64)); // val_bits
        let get_global_id = self
            .engine
            .module
            .declare_function("ely_jit_get_global", Linkage::Import, &get_global_sig)
            .map_err(|e| e.to_string())?;

        let mut print_val_sig = self.engine.module.make_signature();
        print_val_sig.params.push(AbiParam::new(types::I64)); // val_bits
        let print_val_id = self
            .engine
            .module
            .declare_function("ely_jit_print_value", Linkage::Import, &print_val_sig)
            .map_err(|e| e.to_string())?;

        let mut call_0_sig = self.engine.module.make_signature();
        call_0_sig.params.push(AbiParam::new(types::I64)); // callee_bits
        call_0_sig.returns.push(AbiParam::new(types::F64));
        let call_0_id = self
            .engine
            .module
            .declare_function("ely_jit_call_0", Linkage::Import, &call_0_sig)
            .map_err(|e| e.to_string())?;

        let mut call_1_sig = self.engine.module.make_signature();
        call_1_sig.params.push(AbiParam::new(types::I64)); // callee_bits
        call_1_sig.params.push(AbiParam::new(types::F64)); // arg0
        call_1_sig.returns.push(AbiParam::new(types::F64));
        let call_1_id = self
            .engine
            .module
            .declare_function("ely_jit_call_1", Linkage::Import, &call_1_sig)
            .map_err(|e| e.to_string())?;

        let mut call_2_sig = self.engine.module.make_signature();
        call_2_sig.params.push(AbiParam::new(types::I64)); // callee_bits
        call_2_sig.params.push(AbiParam::new(types::F64)); // arg0
        call_2_sig.params.push(AbiParam::new(types::F64)); // arg1
        call_2_sig.returns.push(AbiParam::new(types::F64));
        let call_2_id = self
            .engine
            .module
            .declare_function("ely_jit_call_2", Linkage::Import, &call_2_sig)
            .map_err(|e| e.to_string())?;

        // Set signature in codegen context
        self.engine.ctx.func.signature = sig;

        {
            let mut builder = FunctionBuilder::new(
                &mut self.engine.ctx.func,
                &mut self.engine.builder_context,
            );

            // Import helper function references into current function
            let set_global_func = self
                .engine
                .module
                .declare_func_in_func(set_global_id, &mut builder.func);
            let get_global_func = self
                .engine
                .module
                .declare_func_in_func(get_global_id, &mut builder.func);
            let print_val_func = self
                .engine
                .module
                .declare_func_in_func(print_val_id, &mut builder.func);
            let call_0_func = self
                .engine
                .module
                .declare_func_in_func(call_0_id, &mut builder.func);
            let call_1_func = self
                .engine
                .module
                .declare_func_in_func(call_1_id, &mut builder.func);
            let call_2_func = self
                .engine
                .module
                .declare_func_in_func(call_2_id, &mut builder.func);

            // Pre-pass: scan chunk to find branch targets and maximum local slot
            let mut branch_targets = BTreeSet::new();
            branch_targets.insert(0); // Entry block offset

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
                        Opcode::Constant
                        | Opcode::DefineGlobal
                        | Opcode::GetGlobal
                        | Opcode::SetGlobal
                        | Opcode::Closure => scan_ip += 1,
                        Opcode::Jump | Opcode::JumpIfFalse => {
                            let high = chunk.code[scan_ip] as usize;
                            let low = chunk.code[scan_ip + 1] as usize;
                            let offset = (high << 8) | low;
                            scan_ip += 2;
                            let target = scan_ip + offset;
                            branch_targets.insert(target);
                            branch_targets.insert(scan_ip); // Fallthrough block
                        }
                        Opcode::Loop => {
                            let high = chunk.code[scan_ip] as usize;
                            let low = chunk.code[scan_ip + 1] as usize;
                            let offset = (high << 8) | low;
                            scan_ip += 2;
                            let target = scan_ip - offset;
                            branch_targets.insert(target);
                            branch_targets.insert(scan_ip);
                        }
                        _ => {}
                    }
                } else {
                    scan_ip += 1;
                }
            }

            // Create Cranelift basic blocks for each branch target
            let mut blocks: HashMap<usize, Block> = HashMap::new();
            for &target in &branch_targets {
                blocks.insert(target, builder.create_block());
            }

            let entry_block = *blocks.get(&0).unwrap();
            builder.append_block_params_for_function_params(entry_block);
            builder.switch_to_block(entry_block);

            // Declare Cranelift SSA variables for each local slot (registers)
            let mut vars = Vec::with_capacity(max_local + 1);
            for i in 0..=max_local {
                let var = Variable::new(i);
                builder.declare_var(var, types::F64);
                vars.push(var);
            }

            // Bind incoming function arguments to the corresponding local slots
            let params = builder.block_params(entry_block).to_vec();
            for (idx, &param_val) in params.iter().enumerate() {
                let slot = idx + 1;
                if slot < vars.len() {
                    builder.def_var(vars[slot], param_val);
                }
            }

            // Compile-time abstract evaluation stack of Cranelift SSA values
            let mut eval_stack: Vec<Value> = Vec::new();
            let mut current_block_terminated = false;

            let mut ip = 0;
            while ip < chunk.count() {
                // If the current IP is a known branch target and not entry at start, transition blocks
                if ip > 0 {
                    if let Some(&block) = blocks.get(&ip) {
                        if !current_block_terminated {
                            builder.ins().jump(block, &[]);
                        }
                        builder.switch_to_block(block);
                        current_block_terminated = false;
                    }
                }

                let op = Opcode::try_from(chunk.code[ip])
                    .map_err(|_| format!("Invalid opcode byte at offset {}", ip))?;
                ip += 1;

                if current_block_terminated {
                    // The current basic block has already ended with a terminator (Return or unconditional Jump).
                    // Skip dead/unreachable code until we reach a branch target block.
                    match op {
                        Opcode::Constant
                        | Opcode::GetLocal
                        | Opcode::SetLocal
                        | Opcode::GetGlobal
                        | Opcode::DefineGlobal
                        | Opcode::SetGlobal
                        | Opcode::Call
                        | Opcode::Closure => {
                            ip += 1;
                        }
                        Opcode::Jump | Opcode::JumpIfFalse | Opcode::Loop => {
                            ip += 2;
                        }
                        _ => {}
                    }
                    continue;
                }

                match op {
                    Opcode::Constant => {
                        let const_idx = chunk.code[ip] as usize;
                        ip += 1;
                        let const_val = chunk
                            .get_constant(const_idx)
                            .ok_or("Constant not found in chunk pool")?;
                        let val = if const_val.is_number() {
                            builder.ins().f64const(const_val.as_number())
                        } else {
                            // Non-numeric literal (e.g. object / string handle)
                            let bits = const_val.0 as i64;
                            let i64_val = builder.ins().iconst(types::I64, bits);
                            builder.ins().bitcast(types::F64, MemFlags::new(), i64_val)
                        };
                        eval_stack.push(val);
                    }
                    Opcode::True => {
                        let bits = ElyValue::bool(true).0 as i64;
                        let i64_val = builder.ins().iconst(types::I64, bits);
                        eval_stack.push(builder.ins().bitcast(types::F64, MemFlags::new(), i64_val));
                    }
                    Opcode::False => {
                        let bits = ElyValue::bool(false).0 as i64;
                        let i64_val = builder.ins().iconst(types::I64, bits);
                        eval_stack.push(builder.ins().bitcast(types::F64, MemFlags::new(), i64_val));
                    }
                    Opcode::Closure => {
                        let const_idx = chunk.code[ip] as usize;
                        ip += 1;
                        let const_val = chunk
                            .get_constant(const_idx)
                            .ok_or("Constant not found in chunk pool for Opcode::Closure")?;
                        let bits = const_val.0 as i64;
                        let i64_val = builder.ins().iconst(types::I64, bits);
                        eval_stack.push(builder.ins().bitcast(types::F64, MemFlags::new(), i64_val));
                    }
                    Opcode::DefineGlobal => {
                        let const_idx = chunk.code[ip] as usize;
                        ip += 1;
                        let const_val = chunk.get_constant(const_idx).ok_or("Global name constant not found")?;
                        let name_id = const_val.as_obj() as u32;

                        let val = eval_stack.pop().ok_or("Stack underflow in DefineGlobal")?;
                        let bits = builder.ins().bitcast(types::I64, MemFlags::new(), val);
                        let id_val = builder.ins().iconst(types::I32, name_id as i64);
                        builder.ins().call(set_global_func, &[id_val, bits]);
                    }
                    Opcode::GetGlobal => {
                        let const_idx = chunk.code[ip] as usize;
                        ip += 1;
                        let const_val = chunk.get_constant(const_idx).ok_or("Global name constant not found")?;
                        let name_id = const_val.as_obj() as u32;

                        let id_val = builder.ins().iconst(types::I32, name_id as i64);
                        let call = builder.ins().call(get_global_func, &[id_val]);
                        let bits = builder.inst_results(call)[0];
                        let val = builder.ins().bitcast(types::F64, MemFlags::new(), bits);
                        eval_stack.push(val);
                    }
                    Opcode::SetGlobal => {
                        let const_idx = chunk.code[ip] as usize;
                        ip += 1;
                        let const_val = chunk.get_constant(const_idx).ok_or("Global name constant not found")?;
                        let name_id = const_val.as_obj() as u32;

                        let val = *eval_stack.last().ok_or("Stack underflow in SetGlobal")?;
                        let bits = builder.ins().bitcast(types::I64, MemFlags::new(), val);
                        let id_val = builder.ins().iconst(types::I32, name_id as i64);
                        builder.ins().call(set_global_func, &[id_val, bits]);
                    }
                    Opcode::Call => {
                        let arg_count = chunk.code[ip] as usize;
                        ip += 1;

                        let mut args = Vec::with_capacity(arg_count);
                        for _ in 0..arg_count {
                            args.push(eval_stack.pop().ok_or("Stack underflow in Call arguments")?);
                        }
                        args.reverse();

                        let callee = eval_stack.pop().ok_or("Stack underflow in Call callee")?;
                        let callee_i64 = builder.ins().bitcast(types::I64, MemFlags::new(), callee);

                        let res = match arg_count {
                            0 => {
                                let inst = builder.ins().call(call_0_func, &[callee_i64]);
                                builder.inst_results(inst)[0]
                            }
                            1 => {
                                let inst = builder.ins().call(call_1_func, &[callee_i64, args[0]]);
                                builder.inst_results(inst)[0]
                            }
                            2 => {
                                let inst = builder.ins().call(call_2_func, &[callee_i64, args[0], args[1]]);
                                builder.inst_results(inst)[0]
                            }
                            _ => return Err(format!("JIT does not yet support calls with {} arguments", arg_count)),
                        };
                        eval_stack.push(res);
                    }
                    Opcode::Print => {
                        let val = eval_stack.pop().ok_or("Stack underflow in Opcode::Print")?;
                        let bits = builder.ins().bitcast(types::I64, MemFlags::new(), val);
                        builder.ins().call(print_val_func, &[bits]);
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
                    Opcode::Not => {
                        let a = eval_stack.pop().ok_or("Stack underflow in Opcode::Not")?;
                        let zero = builder.ins().f64const(0.0);
                        let is_zero = builder.ins().fcmp(FloatCC::Equal, a, zero);
                        let true_val = builder.ins().f64const(1.0);
                        let false_val = builder.ins().f64const(0.0);
                        let res = builder.ins().select(is_zero, true_val, false_val);
                        eval_stack.push(res);
                    }
                    Opcode::Less => {
                        let b = eval_stack.pop().ok_or("Stack underflow in Opcode::Less")?;
                        let a = eval_stack.pop().ok_or("Stack underflow in Opcode::Less")?;
                        let cmp = builder.ins().fcmp(FloatCC::LessThan, a, b);
                        let true_val = builder.ins().f64const(1.0);
                        let false_val = builder.ins().f64const(0.0);
                        let res = builder.ins().select(cmp, true_val, false_val);
                        eval_stack.push(res);
                    }
                    Opcode::Greater => {
                        let b = eval_stack
                            .pop()
                            .ok_or("Stack underflow in Opcode::Greater")?;
                        let a = eval_stack
                            .pop()
                            .ok_or("Stack underflow in Opcode::Greater")?;
                        let cmp = builder.ins().fcmp(FloatCC::GreaterThan, a, b);
                        let true_val = builder.ins().f64const(1.0);
                        let false_val = builder.ins().f64const(0.0);
                        let res = builder.ins().select(cmp, true_val, false_val);
                        eval_stack.push(res);
                    }
                    Opcode::Equal => {
                        let b = eval_stack.pop().ok_or("Stack underflow in Opcode::Equal")?;
                        let a = eval_stack.pop().ok_or("Stack underflow in Opcode::Equal")?;
                        let cmp = builder.ins().fcmp(FloatCC::Equal, a, b);
                        let true_val = builder.ins().f64const(1.0);
                        let false_val = builder.ins().f64const(0.0);
                        let res = builder.ins().select(cmp, true_val, false_val);
                        eval_stack.push(res);
                    }
                    Opcode::BwAnd => {
                        let b = eval_stack.pop().ok_or("Stack underflow in Opcode::BwAnd")?;
                        let a = eval_stack.pop().ok_or("Stack underflow in Opcode::BwAnd")?;
                        let a_int = builder.ins().fcvt_to_sint(types::I64, a);
                        let b_int = builder.ins().fcvt_to_sint(types::I64, b);
                        let res_int = builder.ins().band(a_int, b_int);
                        let res = builder.ins().fcvt_from_sint(types::F64, res_int);
                        eval_stack.push(res);
                    }
                    Opcode::BwOr => {
                        let b = eval_stack.pop().ok_or("Stack underflow in Opcode::BwOr")?;
                        let a = eval_stack.pop().ok_or("Stack underflow in Opcode::BwOr")?;
                        let a_int = builder.ins().fcvt_to_sint(types::I64, a);
                        let b_int = builder.ins().fcvt_to_sint(types::I64, b);
                        let res_int = builder.ins().bor(a_int, b_int);
                        let res = builder.ins().fcvt_from_sint(types::F64, res_int);
                        eval_stack.push(res);
                    }
                    Opcode::BwXor => {
                        let b = eval_stack.pop().ok_or("Stack underflow in Opcode::BwXor")?;
                        let a = eval_stack.pop().ok_or("Stack underflow in Opcode::BwXor")?;
                        let a_int = builder.ins().fcvt_to_sint(types::I64, a);
                        let b_int = builder.ins().fcvt_to_sint(types::I64, b);
                        let res_int = builder.ins().bxor(a_int, b_int);
                        let res = builder.ins().fcvt_from_sint(types::F64, res_int);
                        eval_stack.push(res);
                    }
                    Opcode::BwNot => {
                        let a = eval_stack.pop().ok_or("Stack underflow in Opcode::BwNot")?;
                        let a_int = builder.ins().fcvt_to_sint(types::I64, a);
                        let res_int = builder.ins().bnot(a_int);
                        let res = builder.ins().fcvt_from_sint(types::F64, res_int);
                        eval_stack.push(res);
                    }
                    Opcode::ShiftLeft => {
                        let b = eval_stack.pop().ok_or("Stack underflow in Opcode::ShiftLeft")?;
                        let a = eval_stack.pop().ok_or("Stack underflow in Opcode::ShiftLeft")?;
                        let a_int = builder.ins().fcvt_to_sint(types::I64, a);
                        let b_int = builder.ins().fcvt_to_sint(types::I64, b);
                        let res_int = builder.ins().ishl(a_int, b_int);
                        let res = builder.ins().fcvt_from_sint(types::F64, res_int);
                        eval_stack.push(res);
                    }
                    Opcode::ShiftRight => {
                        let b = eval_stack.pop().ok_or("Stack underflow in Opcode::ShiftRight")?;
                        let a = eval_stack.pop().ok_or("Stack underflow in Opcode::ShiftRight")?;
                        let a_int = builder.ins().fcvt_to_sint(types::I64, a);
                        let b_int = builder.ins().fcvt_to_sint(types::I64, b);
                        let res_int = builder.ins().sshr(a_int, b_int);
                        let res = builder.ins().fcvt_from_sint(types::F64, res_int);
                        eval_stack.push(res);
                    }
                    Opcode::Jump => {
                        let high = chunk.code[ip] as usize;
                        let low = chunk.code[ip + 1] as usize;
                        let offset = (high << 8) | low;
                        ip += 2;
                        let target = ip + offset;
                        let target_block = *blocks.get(&target).unwrap();
                        builder.ins().jump(target_block, &[]);
                        current_block_terminated = true;
                    }
                    Opcode::JumpIfFalse => {
                        let high = chunk.code[ip] as usize;
                        let low = chunk.code[ip + 1] as usize;
                        let offset = (high << 8) | low;
                        ip += 2;
                        let target = ip + offset;
                        let target_block = *blocks.get(&target).unwrap();
                        let fallthrough_block = *blocks.get(&ip).unwrap();

                        let cond_val = *eval_stack
                            .last()
                            .ok_or("Stack underflow in JumpIfFalse")?;
                        let zero = builder.ins().f64const(0.0);
                        let is_falsy = builder.ins().fcmp(FloatCC::Equal, cond_val, zero);

                        builder
                            .ins()
                            .brif(is_falsy, target_block, &[], fallthrough_block, &[]);
                        current_block_terminated = true;
                    }
                    Opcode::Loop => {
                        let high = chunk.code[ip] as usize;
                        let low = chunk.code[ip + 1] as usize;
                        let offset = (high << 8) | low;
                        ip += 2;
                        let target = ip - offset;
                        let target_block = *blocks.get(&target).unwrap();
                        builder.ins().jump(target_block, &[]);
                        current_block_terminated = true;
                    }
                    Opcode::Return => {
                        let res = eval_stack
                            .pop()
                            .unwrap_or_else(|| builder.ins().f64const(0.0));
                        builder.ins().return_(&[res]);
                        current_block_terminated = true;
                    }
                    Opcode::Pop => {
                        eval_stack.pop();
                    }
                    Opcode::Nop => {}
                    _ => {
                        return Err(format!("Unsupported opcode for JIT: {:?}", op));
                    }
                }
            }

            // Seal all blocks after CFG generation is complete
            for &block in blocks.values() {
                builder.seal_block(block);
            }

            builder.finalize();
        }

        if self.dump_ir {
            println!("=== Cranelift IR [{}] ===", name);
            println!("{}", self.engine.ctx.func);
        }

        if self.dump_asm {
            self.engine.ctx.set_disasm(true);
        }

        // Define function body in the JIT module and finalize code emission
        self.engine
            .module
            .define_function(func_id, &mut self.engine.ctx)
            .map_err(|e| e.to_string())?;

        if self.dump_asm {
            if let Some(compiled_code) = self.engine.ctx.compiled_code() {
                println!("=== Machine Code Disassembly [{}] ===", name);
                if let Some(ref disasm) = compiled_code.vcode {
                    println!("{}", disasm);
                } else {
                    println!("Binary size: {} bytes", compiled_code.buffer.data().len());
                }
            }
        }

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
