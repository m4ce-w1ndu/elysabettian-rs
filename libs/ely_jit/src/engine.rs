use cranelift::prelude::*;
use cranelift_jit::{JITBuilder, JITModule};
use cranelift_module::{Module, ModuleResult};

use crate::runtime::{ely_jit_get_global, ely_jit_print_value, ely_jit_set_global};

pub struct JitEngine {
    pub builder_context: FunctionBuilderContext,
    pub ctx: codegen::Context,
    pub module: JITModule,
}

impl JitEngine {
    pub fn new() -> ModuleResult<Self> {
        let mut builder = JITBuilder::new(cranelift_module::default_libcall_names())?;

        // Register native runtime helper symbols
        builder.symbol("ely_jit_set_global", ely_jit_set_global as *const u8);
        builder.symbol("ely_jit_get_global", ely_jit_get_global as *const u8);
        builder.symbol("ely_jit_print_value", ely_jit_print_value as *const u8);

        let module = JITModule::new(builder);

        Ok(Self {
            builder_context: FunctionBuilderContext::new(),
            ctx: module.make_context(),
            module,
        })
    }
}
