use cranelift::prelude::*;
use cranelift_jit::{JITBuilder, JITModule};
use cranelift_module::{Module, ModuleError, ModuleResult};

use crate::runtime::{
    ely_jit_call_0, ely_jit_call_1, ely_jit_call_2, ely_jit_get_global, ely_jit_print_value,
    ely_jit_set_global,
};

pub struct JitEngine {
    pub builder_context: FunctionBuilderContext,
    pub ctx: codegen::Context,
    pub module: JITModule,
}

impl JitEngine {
    pub fn new() -> ModuleResult<Self> {
        let mut flag_builder = settings::builder();
        // Disable PIC and colocated libcalls to avoid PLT requirements on aarch64 (macOS/Linux)
        flag_builder.set("use_colocated_libcalls", "false").unwrap();
        flag_builder.set("is_pic", "false").unwrap();

        let isa_builder =
            cranelift_native::builder().map_err(|e| ModuleError::Backend(anyhow::anyhow!(e)))?;
        let isa = isa_builder
            .finish(settings::Flags::new(flag_builder))
            .map_err(|e| ModuleError::Backend(anyhow::anyhow!(e)))?;

        let mut builder = JITBuilder::with_isa(isa, cranelift_module::default_libcall_names());

        // Register native runtime helper symbols
        builder.symbol("ely_jit_set_global", ely_jit_set_global as *const u8);
        builder.symbol("ely_jit_get_global", ely_jit_get_global as *const u8);
        builder.symbol("ely_jit_print_value", ely_jit_print_value as *const u8);
        builder.symbol("ely_jit_call_0", ely_jit_call_0 as *const u8);
        builder.symbol("ely_jit_call_1", ely_jit_call_1 as *const u8);
        builder.symbol("ely_jit_call_2", ely_jit_call_2 as *const u8);

        let module = JITModule::new(builder);

        Ok(Self {
            builder_context: FunctionBuilderContext::new(),
            ctx: module.make_context(),
            module,
        })
    }
}
