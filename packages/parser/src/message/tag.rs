use anyhow::{Error, Result, bail};
use std::fmt;
use std::str::FromStr;
use types::MessageTag;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TagLexeme(MessageTag);

impl FromStr for TagLexeme {
    type Err = Error;

    fn from_str(text: &str) -> Result<Self> {
        Ok(Self(match text {
            "NATIVE" => MessageTag::Native,
            "NATIVE_CLASS" => MessageTag::Class,
            "NATIVE_ASSIGN" => MessageTag::Assign,
            "NATIVE_ARITH" => MessageTag::Arith,
            "NATIVE_PRINT" => MessageTag::Print,
            "NATIVE_PRINT_CONST" => MessageTag::PrintConst,
            "NATIVE_FIELD_PRINT" => MessageTag::FieldPrint,
            "NATIVE_LINK_ASSIGN" => MessageTag::LinkAssign,
            "NATIVE_CONSTRUCT" => MessageTag::Construct,
            "NATIVE_MODULE" => MessageTag::Module,
            "NATIVE_IF_BEGIN" => MessageTag::IfBegin,
            "NATIVE_IF_CALL" => MessageTag::IfCall,
            "NATIVE_IF_CONST" => MessageTag::IfConst,
            "NATIVE_IF_ELSE" => MessageTag::IfElse,
            "NATIVE_IF_END" => MessageTag::IfEnd,
            "NATIVE_WHILE_BEGIN" => MessageTag::WhileBegin,
            "NATIVE_LOOP_BEGIN" => MessageTag::LoopBegin,
            "NATIVE_LOOP_END" => MessageTag::LoopEnd,
            "NATIVE_BREAK" => MessageTag::Break,
            "NATIVE_ALIAS" => MessageTag::Alias,
            "NATIVE_COPY" => MessageTag::Copy,
            "NATIVE_HANDLER_BEGIN" => MessageTag::HandlerBegin,
            "NATIVE_HANDLER_END" => MessageTag::HandlerEnd,
            "NATIVE_EVENT_ON" => MessageTag::EventOn,
            "NATIVE_EMIT" => MessageTag::Emit,
            "NATIVE_EVENT_OFF" => MessageTag::EventOff,
            "NATIVE_EVENT_DESTROY" => MessageTag::EventDestroy,
            "NATIVE_THREAD_RUN" => MessageTag::ThreadRun,
            "NATIVE_AWAIT" => MessageTag::Await,
            "NATIVE_OWNERSHIP" => MessageTag::Ownership,
            "NATIVE_IF_STATE" => MessageTag::IfState,
            "NATIVE_THROW" => MessageTag::Throw,
            other => bail!("Unknown message tag `{other}`"),
        }))
    }
}

impl TryFrom<&str> for TagLexeme {
    type Error = Error;

    fn try_from(text: &str) -> Result<Self> {
        text.parse()
    }
}

impl From<MessageTag> for TagLexeme {
    fn from(tag: MessageTag) -> Self {
        Self(tag)
    }
}

impl From<TagLexeme> for MessageTag {
    fn from(lexeme: TagLexeme) -> Self {
        lexeme.0
    }
}

impl fmt::Display for TagLexeme {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self.0 {
            MessageTag::Native => "NATIVE",
            MessageTag::Class => "NATIVE_CLASS",
            MessageTag::Assign => "NATIVE_ASSIGN",
            MessageTag::Arith => "NATIVE_ARITH",
            MessageTag::Print => "NATIVE_PRINT",
            MessageTag::PrintConst => "NATIVE_PRINT_CONST",
            MessageTag::FieldPrint => "NATIVE_FIELD_PRINT",
            MessageTag::LinkAssign => "NATIVE_LINK_ASSIGN",
            MessageTag::Construct => "NATIVE_CONSTRUCT",
            MessageTag::Module => "NATIVE_MODULE",
            MessageTag::IfBegin => "NATIVE_IF_BEGIN",
            MessageTag::IfCall => "NATIVE_IF_CALL",
            MessageTag::IfConst => "NATIVE_IF_CONST",
            MessageTag::IfElse => "NATIVE_IF_ELSE",
            MessageTag::IfEnd => "NATIVE_IF_END",
            MessageTag::WhileBegin => "NATIVE_WHILE_BEGIN",
            MessageTag::LoopBegin => "NATIVE_LOOP_BEGIN",
            MessageTag::LoopEnd => "NATIVE_LOOP_END",
            MessageTag::Break => "NATIVE_BREAK",
            MessageTag::Alias => "NATIVE_ALIAS",
            MessageTag::Copy => "NATIVE_COPY",
            MessageTag::HandlerBegin => "NATIVE_HANDLER_BEGIN",
            MessageTag::HandlerEnd => "NATIVE_HANDLER_END",
            MessageTag::EventOn => "NATIVE_EVENT_ON",
            MessageTag::Emit => "NATIVE_EMIT",
            MessageTag::EventOff => "NATIVE_EVENT_OFF",
            MessageTag::EventDestroy => "NATIVE_EVENT_DESTROY",
            MessageTag::ThreadRun => "NATIVE_THREAD_RUN",
            MessageTag::Await => "NATIVE_AWAIT",
            MessageTag::Ownership => "NATIVE_OWNERSHIP",
            MessageTag::IfState => "NATIVE_IF_STATE",
            MessageTag::Throw => "NATIVE_THROW",
        })
    }
}
