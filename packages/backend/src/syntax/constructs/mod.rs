//! One module per syntactic construct.
//!
//! Each submodule exports a single construct type that implements both
//! [`Parse`](crate::syntax::parse::Parse) and
//! [`Format`](crate::syntax::format::Format). `mod.rs` only re-exports so the
//! construct list can be assembled without reaching into each file's details.

pub mod assignment;
pub mod block_end;
pub mod break_statement;
pub mod class;
pub mod declaration;
pub mod else_block;
pub mod enum_decl;
pub mod function;
pub mod function_call;
pub mod if_statement;
pub mod import;
pub mod init_block;
pub mod method_call;
pub mod native_call;
pub mod print;
pub mod unload;
pub mod while_statement;

pub use assignment::Assignment;
pub use block_end::BlockEnd;
pub use break_statement::BreakStatement;
pub use class::Class;
pub use declaration::Declaration;
pub use else_block::ElseBlock;
pub use enum_decl::EnumDecl;
pub use function::FunctionDecl;
pub use function_call::FunctionCall;
pub use if_statement::IfStatement;
pub use import::Import;
pub use init_block::InitBlock;
pub use method_call::MethodCall;
pub use native_call::NativeCall;
pub use print::Print;
pub use unload::Unload;
pub use while_statement::WhileStatement;

use crate::syntax::parse::Parse;

/// The default, ordered list of syntax constructs the parser will try.
///
/// Order matters: earlier constructs take precedence, matching the reference
/// grammar (imports and declarations first, then blocks, then statements).
pub fn default_constructs() -> Vec<Box<dyn Parse>> {
    vec![
        Box::new(Import),
        Box::new(EnumDecl),
        Box::new(Class),
        Box::new(FunctionDecl),
        Box::new(InitBlock),
        Box::new(ElseBlock),
        Box::new(BlockEnd),
        Box::new(IfStatement),
        Box::new(WhileStatement),
        Box::new(BreakStatement),
        Box::new(Declaration),
        Box::new(Assignment),
        Box::new(Unload),
        Box::new(MethodCall),
        Box::new(FunctionCall),
        Box::new(NativeCall),
        Box::new(Print),
    ]
}
