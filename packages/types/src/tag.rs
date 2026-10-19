/// Prefix byte of every compiler directive line in a lowered message.
pub const RECORD_SEPARATOR: char = '\u{1e}';

/// Separator between the fields of a directive line.
pub const FIELD_SEPARATOR: char = '\t';

/// The directive tags shared by the backend (producer) and the compiler
/// (consumer).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MessageTag {
    Native,
    Class,
    Assign,
    Arith,
    Compare,
    Cast,
    Print,
    PrintConst,
    FieldPrint,
    FieldAssign,
    FieldLoad,
    LinkAssign,
    Construct,
    Module,
    IfBegin,
    IfCall,
    IfConst,
    IfElse,
    IfEnd,
    WhileBegin,
    LoopBegin,
    LoopEnd,
    Break,
    Alias,
    Copy,
    HandlerBegin,
    HandlerEnd,
    FunctionBegin,
    FunctionRef,
    CallFn,
    CallIndirect,
    Return,
    EventOn,
    Emit,
    EventOff,
    EventDestroy,
    ThreadRun,
    Await,
    Ownership,
    IfState,
    Throw,
}
