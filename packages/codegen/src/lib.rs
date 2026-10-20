extern crate llvm_sys as llvm;

mod context;
mod driver;
mod native;
mod node;
mod program;
mod support;

pub(crate) use context::CodegenContext;
pub(crate) use node::{Codegen, CodegenResult, Text};

pub use driver::{generate, generate_library};
pub use support::{llvm_failure, read_message};
