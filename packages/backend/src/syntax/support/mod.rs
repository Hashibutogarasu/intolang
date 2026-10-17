//! Shared helpers used by the syntax constructs.
//!
//! These modules hold the lowering primitives (expression parsing, message
//! emission, declaration analysis) so that individual constructs stay small.

pub mod analysis;
pub mod call;
pub mod emit;
pub mod literal;
pub mod text;
pub mod value;

pub use analysis::{parse_classes, parse_enums, parse_user_functions};
pub use call::{
    inline_user_function, resolve_method, resolve_receiver, static_field, static_receiver,
};
pub use emit::{comparison_operand, emit_comparison, emit_link_call, emit_print, emit_print_const};
pub use literal::{Literal, parse_literal};
pub use text::{
    collect_type_params, diagnostic, imported_call, is_identifier, parse_call, parse_comparison,
    split_annotation,
};
pub use value::declaration_value;
