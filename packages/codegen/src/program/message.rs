use errors::Result;
use parser::Record;
use types::MessageTag;

use crate::native::{
    Class, NativeAlias, NativeArith, NativeAssign, NativeAwait, NativeBreak, NativeCall,
    NativeCallFn, NativeCallIndirect, NativeCast, NativeCompare, NativeConstruct, NativeCopy,
    NativeEmit, NativeEventDestroy, NativeEventOff, NativeEventOn, NativeFieldAssign,
    NativeFieldLoad, NativeFieldPrint, NativeFunctionBegin, NativeFunctionRef, NativeHandlerBegin,
    NativeHandlerEnd, NativeIfBegin, NativeIfCall, NativeIfConst, NativeIfElse, NativeIfEnd,
    NativeIfState, NativeLinkAssign, NativeLoad, NativeLoopBegin, NativeLoopEnd, NativeModule,
    NativeOwnership, NativePrint, NativePrintConst, NativeReturn, NativeStore, NativeThreadRun,
    NativeThrow, NativeWhileBegin,
};
use crate::{Codegen, Text};

/// A lowered message split into native class definitions and executable
/// statements.
pub struct Program {
    pub classes: Vec<Class>,
    pub statements: Vec<Box<dyn Codegen>>,
}

/// Build the ordered AST of a lowered message: every compiler directive line
/// becomes the node its tag names, and any other line is literal text.
pub(crate) fn parse(message: &str) -> Result<Program> {
    let mut classes = Vec::new();
    let mut statements: Vec<Box<dyn Codegen>> = Vec::new();
    for line in message.split_inclusive('\n') {
        let Ok(record) = line.parse::<Record>() else {
            statements.push(Box::new(Text::new(line)));
            continue;
        };
        let node: Box<dyn Codegen> = match record.tag() {
            MessageTag::Class => {
                classes.push(Class::try_from(&record)?);
                continue;
            }
            MessageTag::Native => Box::new(NativeCall::try_from(&record)?),
            MessageTag::Assign => Box::new(NativeAssign::try_from(&record)?),
            MessageTag::Arith => Box::new(NativeArith::try_from(&record)?),
            MessageTag::Compare => Box::new(NativeCompare::try_from(&record)?),
            MessageTag::Cast => Box::new(NativeCast::try_from(&record)?),
            MessageTag::Print => Box::new(NativePrint::try_from(&record)?),
            MessageTag::PrintConst => Box::new(NativePrintConst::try_from(&record)?),
            MessageTag::FieldPrint => Box::new(NativeFieldPrint::try_from(&record)?),
            MessageTag::FieldAssign => Box::new(NativeFieldAssign::try_from(&record)?),
            MessageTag::FieldLoad => Box::new(NativeFieldLoad::try_from(&record)?),
            MessageTag::Load => Box::new(NativeLoad::try_from(&record)?),
            MessageTag::Store => Box::new(NativeStore::try_from(&record)?),
            MessageTag::LinkAssign => Box::new(NativeLinkAssign::try_from(&record)?),
            MessageTag::Construct => Box::new(NativeConstruct::try_from(&record)?),
            MessageTag::Module => Box::new(NativeModule::try_from(&record)?),
            MessageTag::IfBegin => Box::new(NativeIfBegin::try_from(&record)?),
            MessageTag::IfCall => Box::new(NativeIfCall::try_from(&record)?),
            MessageTag::IfConst => Box::new(NativeIfConst::try_from(&record)?),
            MessageTag::IfElse => Box::new(NativeIfElse::from(&record)),
            MessageTag::IfEnd => Box::new(NativeIfEnd::from(&record)),
            MessageTag::WhileBegin => Box::new(NativeWhileBegin::try_from(&record)?),
            MessageTag::LoopBegin => Box::new(NativeLoopBegin::try_from(&record)?),
            MessageTag::LoopEnd => Box::new(NativeLoopEnd::from(&record)),
            MessageTag::Break => Box::new(NativeBreak::from(&record)),
            MessageTag::Alias => Box::new(NativeAlias::try_from(&record)?),
            MessageTag::Copy => Box::new(NativeCopy::try_from(&record)?),
            MessageTag::HandlerBegin => Box::new(NativeHandlerBegin::try_from(&record)?),
            MessageTag::HandlerEnd => Box::new(NativeHandlerEnd::from(&record)),
            MessageTag::FunctionBegin => Box::new(NativeFunctionBegin::try_from(&record)?),
            MessageTag::FunctionRef => Box::new(NativeFunctionRef::try_from(&record)?),
            MessageTag::CallFn => Box::new(NativeCallFn::try_from(&record)?),
            MessageTag::CallIndirect => Box::new(NativeCallIndirect::try_from(&record)?),
            MessageTag::Return => Box::new(NativeReturn::try_from(&record)?),
            MessageTag::EventOn => Box::new(NativeEventOn::try_from(&record)?),
            MessageTag::Emit => Box::new(NativeEmit::try_from(&record)?),
            MessageTag::EventOff => Box::new(NativeEventOff::try_from(&record)?),
            MessageTag::EventDestroy => Box::new(NativeEventDestroy::try_from(&record)?),
            MessageTag::ThreadRun => Box::new(NativeThreadRun::try_from(&record)?),
            MessageTag::Await => Box::new(NativeAwait::try_from(&record)?),
            MessageTag::Ownership => Box::new(NativeOwnership::try_from(&record)?),
            MessageTag::IfState => Box::new(NativeIfState::try_from(&record)?),
            MessageTag::Throw => Box::new(NativeThrow::try_from(&record)?),
        };
        statements.push(node);
    }
    Ok(Program {
        classes,
        statements,
    })
}
