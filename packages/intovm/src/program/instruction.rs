use native::{Comparison, LinkCall, NativeArg, Operand, OwnershipChange, StateCheck};
use std::collections::HashMap;

use crate::value::ClassDef;

/// The test a [`Instr::Branch`] makes.
#[derive(Debug, Clone)]
pub enum Cond {
    Compare(Comparison),
    Call(LinkCall),
    Const(bool),
    State(StateCheck),
}

/// One executable step of a lowered message. Structured control flow is
/// resolved to jumps when the message is loaded.
#[derive(Debug, Clone)]
pub enum Instr {
    Text(String),
    Assign {
        target: String,
        ty: String,
        operand: Operand,
    },
    Arith {
        target: String,
        left: String,
        operation: String,
        operand: Operand,
        ty: String,
    },
    Print {
        variable: String,
        ty: String,
        newline: bool,
    },
    PrintConst {
        ty: String,
        operand: Operand,
        newline: bool,
    },
    FieldPrint {
        variable: String,
        index: usize,
        newline: bool,
    },
    Construct {
        target: String,
        class: String,
    },
    Alias {
        target: String,
        source: String,
    },
    Copy {
        target: String,
        source: String,
    },
    NativeCall {
        symbol: String,
        argument: NativeArg,
    },
    LinkAssign(LinkCall),
    EventOn {
        event: String,
        listener: String,
        handler: String,
    },
    Emit {
        event: String,
        args: Vec<NativeArg>,
    },
    EventOff {
        event: String,
        listener: String,
    },
    EventDestroy {
        event: String,
    },
    ThreadRun {
        task: String,
        function: String,
    },
    Await {
        task: String,
    },
    Ownership(OwnershipChange),
    /// Fail the run with this message.
    Throw(String),
    Branch {
        cond: Cond,
        otherwise: usize,
    },
    Jump(usize),
    Return,
    Nop,
}

/// A handler function the message defines between `NATIVE_HANDLER_BEGIN` and
/// `NATIVE_HANDLER_END`.
#[derive(Debug, Clone)]
pub struct Function {
    pub entry: usize,
    pub params: Vec<String>,
    pub captures: Vec<String>,
}

#[derive(Debug, Clone, Default)]
pub struct Program {
    pub instructions: Vec<Instr>,
    pub classes: HashMap<String, ClassDef>,
    pub functions: HashMap<String, Function>,
}
