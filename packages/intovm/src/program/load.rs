use errors::{Errors, Result};
use native::{
    CastAssign, CompareAssign, Comparison, FieldAssign, FieldLoad, FunctionCall,
    FunctionDefinition, FunctionRef, FunctionReturn, IndirectCall, LinkCall, MemoryLoad,
    MemoryStore, OwnershipChange, StateCheck,
};
use parser::{Hex, Record};
use types::MessageTag;

use super::instruction::{Cond, Function, Instr, Program};
use crate::value::{ClassDef, FieldDef};

/// A construct whose end the loader has not reached yet.
enum Open {
    If { branch: usize, jump: Option<usize> },
    Loop { start: usize, breaks: Vec<usize> },
    Handler { skip: usize },
}

fn field_def(spec: &str) -> FieldDef {
    let mut segments = spec.splitn(4, ':');
    let name = segments.next().unwrap_or_default().to_owned();
    let ty = segments.next().unwrap_or("String").to_owned();
    let kind = segments.next().unwrap_or("-");
    let value = segments.next().unwrap_or_default().to_owned();
    let default = match kind {
        "-" | "" => None,
        _ => Some((kind.chars().next().unwrap_or('-'), value)),
    };
    FieldDef { name, ty, default }
}

impl Program {
    /// Build the program of a lowered message: every directive line becomes
    /// its instruction and any other line is literal output.
    pub fn load(message: &str) -> Result<Self> {
        let mut program = Program::default();
        let mut open: Vec<Open> = Vec::new();
        for line in message.split_inclusive('\n') {
            let Ok(record) = line.parse::<Record>() else {
                program.instructions.push(Instr::Text(line.to_owned()));
                continue;
            };
            program.push_record(&record, &mut open)?;
        }
        if !open.is_empty() {
            return Err(errors::Error::from(Errors::MessageUnfinishedBlock));
        }
        Ok(program)
    }

    fn here(&self) -> usize {
        self.instructions.len()
    }

    fn branch(&mut self, cond: Cond, open: &mut Vec<Open>) {
        open.push(Open::If {
            branch: self.here(),
            jump: None,
        });
        self.instructions.push(Instr::Branch { cond, otherwise: 0 });
    }

    fn loop_branch(&mut self, cond: Cond, open: &mut Vec<Open>) {
        open.push(Open::Loop {
            start: self.here(),
            breaks: Vec::new(),
        });
        self.instructions.push(Instr::Branch { cond, otherwise: 0 });
    }

    fn patch(&mut self, at: usize, target: usize) {
        match &mut self.instructions[at] {
            Instr::Branch { otherwise, .. } => *otherwise = target,
            Instr::Jump(destination) => *destination = target,
            _ => {}
        }
    }

    fn push_record(&mut self, record: &Record, open: &mut Vec<Open>) -> Result<()> {
        match record.tag() {
            MessageTag::Class => {
                let Some((name, fields)) = record.fields().split_first() else {
                    return Err(errors::Error::from(Errors::malformed_record(
                        "NATIVE_CLASS",
                    )));
                };
                self.classes.insert(
                    name.clone(),
                    ClassDef {
                        fields: fields.iter().map(|field| field_def(field)).collect(),
                    },
                );
            }
            MessageTag::Module => self.instructions.push(Instr::Nop),
            MessageTag::Native => self.instructions.push(Instr::NativeCall {
                symbol: record.get(0).to_owned(),
                argument: format!("{}:{}", record.get(1), record.get(2)).parse()?,
            }),
            MessageTag::Assign => self.instructions.push(Instr::Assign {
                target: record.get(0).to_owned(),
                ty: record.get(1).to_owned(),
                operand: record.get(2).parse()?,
            }),
            MessageTag::Arith => self.instructions.push(Instr::Arith {
                target: record.get(0).to_owned(),
                left: record.get(1).to_owned(),
                operation: record.get(2).to_owned(),
                operand: record.get(3).parse()?,
                ty: record.get(4).to_owned(),
            }),
            MessageTag::Compare => self
                .instructions
                .push(Instr::Compare(CompareAssign::try_from(record)?)),
            MessageTag::Load => self
                .instructions
                .push(Instr::Load(MemoryLoad::try_from(record)?)),
            MessageTag::Store => self
                .instructions
                .push(Instr::Store(MemoryStore::try_from(record)?)),
            MessageTag::FieldLoad => self
                .instructions
                .push(Instr::FieldLoad(FieldLoad::try_from(record)?)),
            MessageTag::FieldAssign => self
                .instructions
                .push(Instr::FieldAssign(FieldAssign::try_from(record)?)),
            MessageTag::Cast => self
                .instructions
                .push(Instr::Cast(CastAssign::try_from(record)?)),
            MessageTag::Print => self.instructions.push(Instr::Print {
                variable: record.get(0).to_owned(),
                ty: record.get(1).to_owned(),
                newline: record.get(2) == "1",
            }),
            MessageTag::PrintConst => self.instructions.push(Instr::PrintConst {
                ty: record.get(0).to_owned(),
                operand: record.get(1).parse()?,
                newline: record.get(2) == "1",
            }),
            MessageTag::FieldPrint => self.instructions.push(Instr::FieldPrint {
                variable: record.get(0).to_owned(),
                index: record.get(2).parse()?,
                newline: record.get(4) == "1",
            }),
            MessageTag::LinkAssign => self
                .instructions
                .push(Instr::LinkAssign(LinkCall::try_from(record)?)),
            MessageTag::Construct => self.instructions.push(Instr::Construct {
                target: record.get(0).to_owned(),
                class: record.get(1).to_owned(),
            }),
            MessageTag::Alias => self.instructions.push(Instr::Alias {
                target: record.get(0).to_owned(),
                source: record.get(1).to_owned(),
            }),
            MessageTag::Copy => self.instructions.push(Instr::Copy {
                target: record.get(0).to_owned(),
                source: record.get(1).to_owned(),
            }),
            MessageTag::IfBegin => self.branch(Cond::Compare(Comparison::try_from(record)?), open),
            MessageTag::IfCall => self.branch(Cond::Call(LinkCall::try_from(record)?), open),
            MessageTag::IfConst => self.branch(Cond::Const(record.get(0) == "1"), open),
            MessageTag::IfState => self.branch(Cond::State(StateCheck::try_from(record)?), open),
            MessageTag::Ownership => self
                .instructions
                .push(Instr::Ownership(OwnershipChange::try_from(record)?)),
            MessageTag::Throw => self.instructions.push(Instr::Throw(
                String::from_utf8_lossy(&decode_hex(record.get(0))?).into_owned(),
            )),
            MessageTag::IfElse => {
                let jump = self.here();
                let Some(Open::If { branch, jump: slot }) = open.last_mut() else {
                    return Err(errors::Error::from(Errors::NoMatching {
                        record: "NATIVE_IF_ELSE".to_string(),
                        expected: "if".to_string(),
                    }));
                };
                let branch = *branch;
                *slot = Some(jump);
                self.instructions.push(Instr::Jump(0));
                let otherwise = self.here();
                self.patch(branch, otherwise);
            }
            MessageTag::IfEnd => {
                let Some(Open::If { branch, jump }) = open.pop() else {
                    return Err(errors::Error::from(Errors::NoMatching {
                        record: "NATIVE_IF_END".to_string(),
                        expected: "if".to_string(),
                    }));
                };
                let end = self.here();
                match jump {
                    Some(jump) => self.patch(jump, end),
                    None => self.patch(branch, end),
                }
                self.instructions.push(Instr::Nop);
            }
            MessageTag::WhileBegin => {
                self.loop_branch(Cond::Compare(Comparison::try_from(record)?), open)
            }
            MessageTag::LoopBegin => {
                self.loop_branch(Cond::Call(LinkCall::try_from(record)?), open)
            }
            MessageTag::LoopEnd => {
                let Some(Open::Loop { start, breaks }) = open.pop() else {
                    return Err(errors::Error::from(Errors::NoMatching {
                        record: "NATIVE_LOOP_END".to_string(),
                        expected: "loop".to_string(),
                    }));
                };
                self.instructions.push(Instr::Jump(start));
                let exit = self.here();
                self.patch(start, exit);
                for at in breaks {
                    self.patch(at, exit);
                }
            }
            MessageTag::Break => {
                let at = self.here();
                let Some(Open::Loop { breaks, .. }) = open
                    .iter_mut()
                    .rev()
                    .find(|open| matches!(open, Open::Loop { .. }))
                else {
                    return Err(errors::Error::from(Errors::RecordBreakOutsideLoop));
                };
                breaks.push(at);
                self.instructions.push(Instr::Jump(0));
            }
            MessageTag::HandlerBegin => {
                let captures = record
                    .get(1)
                    .split(',')
                    .filter(|name| !name.is_empty())
                    .map(str::to_owned)
                    .collect();
                let params = record
                    .fields()
                    .iter()
                    .skip(2)
                    .map(|field| {
                        field
                            .split_once(':')
                            .map(|(name, _)| name.to_owned())
                            .ok_or_else(|| {
                                errors::Error::from(Errors::MalformedHandlerParameter {
                                    field: field.to_string(),
                                })
                            })
                    })
                    .collect::<Result<_>>()?;
                let skip = self.here();
                self.instructions.push(Instr::Jump(0));
                self.functions.insert(
                    record.get(0).to_owned(),
                    Function {
                        entry: self.here(),
                        params,
                        captures,
                    },
                );
                open.push(Open::Handler { skip });
            }
            MessageTag::FunctionBegin => {
                let definition = FunctionDefinition::try_from(record)?;
                let skip = self.here();
                self.instructions.push(Instr::Jump(0));
                self.functions.insert(
                    definition.name,
                    Function {
                        entry: self.here(),
                        params: definition
                            .params
                            .into_iter()
                            .map(|(name, _)| name)
                            .collect(),
                        captures: Vec::new(),
                    },
                );
                open.push(Open::Handler { skip });
            }
            MessageTag::FunctionRef => self
                .instructions
                .push(Instr::FunctionRef(FunctionRef::try_from(record)?)),
            MessageTag::CallIndirect => self
                .instructions
                .push(Instr::CallIndirect(IndirectCall::try_from(record)?)),
            MessageTag::CallFn => self
                .instructions
                .push(Instr::CallFn(FunctionCall::try_from(record)?)),
            MessageTag::Return => self
                .instructions
                .push(Instr::ReturnValue(FunctionReturn::try_from(record)?)),
            MessageTag::HandlerEnd => {
                let Some(Open::Handler { skip }) = open.pop() else {
                    return Err(errors::Error::from(Errors::NoMatching {
                        record: "NATIVE_HANDLER_END".to_string(),
                        expected: "handler".to_string(),
                    }));
                };
                self.instructions.push(Instr::Return);
                let after = self.here();
                self.patch(skip, after);
            }
            MessageTag::EventOn => self.instructions.push(Instr::EventOn {
                event: record.get(0).to_owned(),
                listener: record.get(1).to_owned(),
                handler: record.get(2).to_owned(),
            }),
            MessageTag::Emit => self.instructions.push(Instr::Emit {
                event: record.get(0).to_owned(),
                args: record
                    .fields()
                    .iter()
                    .skip(1)
                    .map(|field| field.parse())
                    .collect::<std::result::Result<Vec<_>, errors::Error>>()?,
            }),
            MessageTag::EventOff => self.instructions.push(Instr::EventOff {
                event: record.get(0).to_owned(),
                listener: record.get(1).to_owned(),
            }),
            MessageTag::EventDestroy => self.instructions.push(Instr::EventDestroy {
                event: record.get(0).to_owned(),
            }),
            MessageTag::ThreadRun => self.instructions.push(Instr::ThreadRun {
                task: record.get(0).to_owned(),
                function: record.get(1).to_owned(),
            }),
            MessageTag::Await => self.instructions.push(Instr::Await {
                task: record.get(0).to_owned(),
            }),
        }
        Ok(())
    }
}

/// The bytes a hex-encoded default stands for.
pub fn decode_hex(value: &str) -> Result<Vec<u8>> {
    Ok(value.parse::<Hex>()?.try_into()?)
}
