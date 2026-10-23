use cranelift::prelude::*;
use cranelift_jit::{JITBuilder, JITModule};
use cranelift_module::{Module, ModuleResult};

pub struct JitEngine {
    pub builder_context: FunctionBuilderContext,
    pub ctx: codegen::Context,
    pub module: JITModule,
}

impl JitEngine {
    pub fn new() -> ModuleResult<Self> {
        let builder = JITBuilder::new(cranelift_module::default_libcall_names())?;
        let module = JITModule::new(builder);

        Ok(Self {
            builder_context: FunctionBuilderContext::new(),
            ctx: module.make_context(),
            module,
        })
    }
}
