use crate::codegen::{
    Class, Codegen, NativeArith, NativeAssign, NativeBreak, NativeCall, NativeFieldPrint,
    NativeIfBegin, NativeIfCall, NativeIfConst, NativeIfElse, NativeIfEnd, NativeLinkAssign,
    NativeLoopBegin, NativeLoopEnd, NativePrint, NativePrintConst, NativeWhileBegin, Text,
};

/// A lowered message split into native class definitions and executable
/// statements.
pub struct Program {
    pub classes: Vec<Class>,
    pub statements: Vec<Box<dyn Codegen>>,
}

/// Parse the lowered message into an ordered AST, falling back to literal text
/// for any line that is not a compiler directive.
pub(crate) fn parse(message: &str) -> Program {
    let mut classes = Vec::new();
    let mut statements: Vec<Box<dyn Codegen>> = Vec::new();
    for line in message.split_inclusive('\n') {
        if let Some(node) = Class::parse(line) {
            classes.push(node);
        } else if let Some(node) = NativeIfCall::parse(line) {
            statements.push(Box::new(node));
        } else if let Some(node) = NativeIfConst::parse(line) {
            statements.push(Box::new(node));
        } else if let Some(node) = NativeIfBegin::parse(line) {
            statements.push(Box::new(node));
        } else if let Some(node) = NativeIfElse::parse(line) {
            statements.push(Box::new(node));
        } else if let Some(node) = NativeIfEnd::parse(line) {
            statements.push(Box::new(node));
        } else if let Some(node) = NativeBreak::parse(line) {
            statements.push(Box::new(node));
        } else if let Some(node) = NativeFieldPrint::parse(line) {
            statements.push(Box::new(node));
        } else if let Some(node) = NativePrint::parse(line) {
            statements.push(Box::new(node));
        } else if let Some(node) = NativePrintConst::parse(line) {
            statements.push(Box::new(node));
        } else if let Some(node) = NativeLoopBegin::parse(line) {
            statements.push(Box::new(node));
        } else if let Some(node) = NativeWhileBegin::parse(line) {
            statements.push(Box::new(node));
        } else if let Some(node) = NativeLoopEnd::parse(line) {
            statements.push(Box::new(node));
        } else if let Some(node) = NativeArith::parse(line) {
            statements.push(Box::new(node));
        } else if let Some(node) = NativeAssign::parse(line) {
            statements.push(Box::new(node));
        } else if let Some(node) = NativeLinkAssign::parse(line) {
            statements.push(Box::new(node));
        } else if let Some(node) = NativeCall::parse(line) {
            statements.push(Box::new(node));
        } else {
            statements.push(Box::new(Text::new(line)));
        }
    }
    Program {
        classes,
        statements,
    }
}
