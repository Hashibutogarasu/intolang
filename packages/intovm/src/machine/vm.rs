use anyhow::{Result, anyhow, bail};
use native::LinkCall;
use std::collections::HashMap;
use std::io::Write;
use std::rc::Rc;

use super::arith::{compare, constant, convert, is_float, operate};
use super::events::EventTable;
use super::frame::Frame;
use super::memory::Binding;
use super::process::{ChildBehavior, ProcessState, ProcessTable, install};
use super::threads::Thread;
use super::trace::TraceEntry;
use crate::inex::Inex;
use crate::native_call::{self, Library};
use crate::program::{Cond, Instr, Program};
use crate::value::{Instance, Value};

/// The native function that lets other threads run while this one waits,
/// because the virtual threads only switch at points like this one.
const SLEEP: &str = "standard_sleep";

/// How many frames may be open at once before a call fails.
const MAX_CALL_DEPTH: usize = 100_000;

/// A native function the VM runs instead of calling the C symbol.
pub type Intrinsic = Rc<dyn Fn(&mut Vm, &[Value]) -> Result<Value>>;

/// What a VM starts with.
#[derive(Debug, Clone)]
pub struct VmOptions {
    /// The pid of the virtual process the program is.
    pub current_pid: i32,
    /// The virtual clock when the program starts, in milliseconds.
    pub clock_ms: u64,
    /// What the programs `spawn` can start do, by path. A path that is not
    /// listed behaves as `true`, `false` or `sleep` when it names one of
    /// them and as a program that exits at once with 0 otherwise.
    pub programs: HashMap<String, ChildBehavior>,
    /// Whether the program output also goes to the standard output of the
    /// process the VM runs in as the program prints it.
    pub echo_stdout: bool,
    /// Whether the `ps` functions work on the real operating system instead
    /// of the virtual process table.
    pub real_processes: bool,
}

impl Default for VmOptions {
    fn default() -> Self {
        Self {
            current_pid: 4242,
            clock_ms: 0,
            programs: HashMap::new(),
            echo_stdout: false,
            real_processes: false,
        }
    }
}

/// Runs a lowered message without generating machine code.
pub struct Vm {
    pub(super) program: Rc<Program>,
    pub(super) globals: HashMap<String, Value>,
    pub(super) frames: Vec<Frame>,
    pub(super) heap: Vec<Instance>,
    pub(super) intrinsics: HashMap<String, Intrinsic>,
    pub(super) events: HashMap<String, EventTable>,
    pub(super) threads: Vec<Thread>,
    pub(super) current: usize,
    pub(super) bindings: HashMap<String, Binding>,
    pub(super) in_init: bool,
    pub(super) trace: Vec<TraceEntry>,
    pub(super) processes: ProcessTable,
    pub(super) native: Option<Rc<Library>>,
    pub(super) retained_strings: Vec<std::ffi::CString>,
    options: VmOptions,
    stdout: Vec<u8>,
}

impl Vm {
    pub fn new(options: VmOptions) -> Self {
        let mut vm = Self {
            processes: ProcessTable::new(
                options.current_pid,
                options.clock_ms,
                options.programs.clone(),
            ),
            native: None,
            retained_strings: Vec::new(),
            options,
            program: Rc::new(Program::default()),
            globals: HashMap::new(),
            frames: vec![Frame::main()],
            heap: Vec::new(),
            intrinsics: HashMap::new(),
            events: HashMap::new(),
            threads: vec![Thread::default()],
            current: 0,
            bindings: HashMap::new(),
            in_init: false,
            trace: Vec::new(),
            stdout: Vec::new(),
        };
        let real_processes = vm.options.real_processes;
        install(&mut vm, real_processes);
        super::text::install(&mut vm);
        vm
    }

    pub fn load(&mut self, message: &str) -> Result<()> {
        self.program = Rc::new(Program::load(message)?);
        self.processes = ProcessTable::new(
            self.options.current_pid,
            self.options.clock_ms,
            self.options.programs.clone(),
        );
        self.heap.clear();
        self.globals.clear();
        self.frames = vec![Frame::main()];
        self.threads = vec![Thread::default()];
        self.current = 0;
        self.events.clear();
        self.bindings.clear();
        self.in_init = false;
        self.trace.clear();
        Ok(())
    }

    /// What the program did, in order: destructions and emitted events.
    pub fn trace(&self) -> &[TraceEntry] {
        &self.trace
    }

    pub fn register(&mut self, symbol: &str, intrinsic: Intrinsic) {
        self.intrinsics.insert(symbol.to_owned(), intrinsic);
    }

    pub fn stdout(&self) -> String {
        String::from_utf8_lossy(&self.stdout).into_owned()
    }

    pub fn write(&mut self, bytes: &[u8]) {
        self.stdout.extend_from_slice(bytes);
        if self.options.echo_stdout {
            let mut out = std::io::stdout().lock();
            out.write_all(bytes).ok();
            out.flush().ok();
        }
    }

    pub fn instance(&self, reference: usize) -> Result<&Instance> {
        self.heap
            .get(reference)
            .ok_or_else(|| anyhow!("Dangling reference {reference}"))
    }

    pub fn run(&mut self) -> Result<()> {
        let program = self.program.clone();
        loop {
            if self.processes.terminated {
                return Ok(());
            }
            let Some(frame) = self.frames.last() else {
                if self.thread_finished()? {
                    self.processes.finish();
                    return Ok(());
                }
                continue;
            };
            let Some(instr) = program.instructions.get(frame.pc) else {
                self.frames.pop();
                continue;
            };
            self.step(&program, instr)?;
        }
    }

    /// Load the program of an `.inex` file.
    pub fn load_inex(&mut self, inex: &Inex) -> Result<()> {
        self.load(&inex.message)?;
        if !inex.sources.is_empty() {
            self.native = Some(Rc::new(native_call::load(inex)?));
        }
        Ok(())
    }

    /// The code the program would have ended the operating system process
    /// with: its exit code, or 128 plus the number of the signal that ended it.
    pub fn exit_status(&self) -> i32 {
        match self.processes.get(self.processes.current_pid()) {
            Some(process) => match process.state {
                ProcessState::Exited(code) => code,
                ProcessState::Signaled(signal) => 128 + signal,
                ProcessState::Running => 0,
            },
            None => 0,
        }
    }

    /// The virtual processes and the virtual clock.
    pub fn processes(&self) -> &ProcessTable {
        &self.processes
    }

    /// Move the virtual clock forward.
    pub fn advance_clock(&mut self, milliseconds: u64) {
        self.processes.advance(milliseconds);
    }

    /// Queue or deliver a signal to a virtual process, as if another process
    /// had sent it. Returns 0, or -1 when the process does not exist.
    pub fn send_signal(&mut self, pid: i32, signal: i32) -> i32 {
        self.processes.send(pid, signal)
    }

    fn advance(&mut self) {
        if let Some(frame) = self.frames.last_mut() {
            frame.pc += 1;
        }
    }

    fn jump(&mut self, target: usize) {
        if let Some(frame) = self.frames.last_mut() {
            frame.pc = target;
        }
    }

    fn step(&mut self, program: &Program, instr: &Instr) -> Result<()> {
        match instr {
            Instr::Text(text) => self.write(text.as_bytes()),
            Instr::Nop => {}
            Instr::Assign {
                target,
                ty,
                operand,
            } => {
                let value = self.operand(operand, ty)?;
                self.store(target, value);
            }
            Instr::Arith {
                target,
                left,
                operation,
                operand,
                ty,
            } => {
                let left = self.load_slot(left)?;
                let right = self.operand(operand, ty)?;
                let value = operate(ty, operation, &left, &right)?;
                self.store(target, value);
            }
            Instr::Compare(assign) => {
                let comparison = &assign.comparison;
                let left = self.load_slot(&comparison.left)?;
                let right = self.operand(&comparison.operand, &comparison.type_name)?;
                let holds = compare(comparison.operator, &left, &right)?;
                self.store(&assign.target, Value::Int(i64::from(holds)));
            }
            Instr::FieldLoad(load) => {
                let holder = self.load_slot(&load.slot)?;
                let value = self
                    .read_instance_field(&holder, load.index)
                    .map_err(|error| anyhow!("`{}`: {error}", load.slot))?;
                self.store(&load.target, value);
            }
            Instr::FieldAssign(assign) => {
                let holder = self.load_slot(&assign.slot)?;
                let value = self.operand(&assign.operand, &assign.ty)?;
                self.write_instance_field(&holder, assign.index, value)
                    .map_err(|error| anyhow!("`{}`: {error}", assign.slot))?;
            }
            Instr::Cast(cast) => {
                let value = self.load_slot(&cast.source)?;
                let converted = convert(&cast.from, &cast.to, &value)?;
                self.store(&cast.target, converted);
            }
            Instr::Print {
                variable,
                ty,
                newline,
            } => {
                let value = self.load_slot(variable)?;
                self.print(ty, &value, *newline)?;
            }
            Instr::PrintConst {
                ty,
                operand,
                newline,
            } => {
                let value = self.operand(operand, ty)?;
                self.print(ty, &value, *newline)?;
            }
            Instr::FieldPrint {
                variable,
                index,
                newline,
            } => {
                let holder = self.load_slot(variable)?;
                let class = self
                    .class_of(&holder)
                    .map_err(|_| anyhow!("`{variable}` holds no instance"))?;
                let value = self.read_instance_field(&holder, *index)?;
                let ty = program
                    .classes
                    .get(&class)
                    .and_then(|class| class.fields.get(*index))
                    .map(|field| field.ty.clone())
                    .unwrap_or_default();
                self.print(&ty, &value, *newline)?;
            }
            Instr::Construct { target, class } => {
                let instance = self.construct(program, class)?;
                let reference = self.allocate(instance);
                self.store(target, Value::Ref(reference));
            }
            Instr::Alias { target, source } => {
                let value = self.load_slot(source)?;
                self.store(target, value);
            }
            Instr::Copy { target, source } => {
                let holder = self.load_slot(source)?;
                let class = self
                    .class_of(&holder)
                    .map_err(|_| anyhow!("`{source}` holds no instance"))?;
                let copy = match holder {
                    Value::Native { ptr, .. } => {
                        let size = self.layout_of(&class).size;
                        let mut bytes = vec![0u64; size / 8 + 1];
                        unsafe {
                            std::ptr::copy_nonoverlapping(
                                ptr as *const u8,
                                bytes.as_mut_ptr() as *mut u8,
                                size,
                            )
                        };
                        let ptr = Box::leak(bytes.into_boxed_slice()).as_ptr() as usize;
                        Value::Native { ptr, class }
                    }
                    other => {
                        let Value::Ref(reference) = other else {
                            bail!("`{source}` holds no instance");
                        };
                        let instance = self.instance(reference)?.clone();
                        Value::Ref(self.allocate(instance))
                    }
                };
                self.store(target, copy);
            }
            Instr::NativeCall { symbol, argument } => {
                let argument = self.argument(argument)?;
                self.call_typed(symbol, &[argument], "Void")?;
            }
            Instr::LinkAssign(call) => {
                let result = self.link_call(call)?;
                self.store_result(call, result);
                if call.target == SLEEP {
                    self.advance();
                    self.switch_thread()?;
                    return Ok(());
                }
            }
            Instr::EventOn {
                event,
                listener,
                handler,
            } => self.event_on(event, listener, handler)?,
            Instr::Emit { event, args } => {
                self.advance();
                self.emit(event, args)?;
                return Ok(());
            }
            Instr::EventOff { event, listener } => self.event_off(event, listener)?,
            Instr::EventDestroy { event } => self.event_destroy(event),
            Instr::ThreadRun { task, function } => self.thread_run(task, function)?,
            Instr::Await { task } => {
                if !self.thread_await(task)? {
                    return Ok(());
                }
            }
            Instr::Ownership(change) => self.apply(change)?,
            Instr::Throw(message) => bail!("Into assertion failed: {message}"),
            Instr::Branch { cond, otherwise } => {
                if !self.condition(cond)? {
                    self.jump(*otherwise);
                    return Ok(());
                }
            }
            Instr::Jump(target) => {
                self.jump(*target);
                return Ok(());
            }
            Instr::Return => {
                if let Some(frame) = self.frames.pop()
                    && let Some(target) = frame.returns_to
                {
                    self.store(&target, Value::Int(0));
                }
                return Ok(());
            }
            Instr::CallFn(call) => {
                if self.frames.len() >= MAX_CALL_DEPTH {
                    bail!("Stack overflow in `{}`", call.name);
                }
                let function = program
                    .functions
                    .get(&call.name)
                    .ok_or_else(|| anyhow!("Call of the undefined function `{}`", call.name))?;
                let values = call
                    .args
                    .iter()
                    .map(|arg| self.argument(arg))
                    .collect::<Result<Vec<_>>>()?;
                let locals = function.params.iter().cloned().zip(values).collect();
                self.advance();
                let mut frame = Frame::function(function.entry, locals, Vec::new());
                if !call.target.is_empty() {
                    frame.returns_to = Some(call.target.clone());
                }
                self.frames.push(frame);
                return Ok(());
            }
            Instr::ReturnValue(result) => {
                let value = match &result.operand {
                    Some(operand) => Some(self.operand(operand, &result.ty)?),
                    None => None,
                };
                if let Some(frame) = self.frames.pop()
                    && let (Some(target), Some(value)) = (frame.returns_to, value)
                {
                    self.store(&target, value);
                }
                return Ok(());
            }
        }
        self.advance();
        Ok(())
    }

    fn condition(&mut self, cond: &Cond) -> Result<bool> {
        match cond {
            Cond::Const(value) => Ok(*value),
            Cond::State(check) => self.check_state(check),
            Cond::Compare(comparison) => {
                let left = self.load_slot(&comparison.left)?;
                let right = self.operand(&comparison.operand, &comparison.type_name)?;
                compare(comparison.operator, &left, &right)
            }
            Cond::Call(call) => {
                let result = self.link_call(call)?;
                self.store_result(call, result.clone());
                Ok(result != Value::Int(0))
            }
        }
    }

    fn link_call(&mut self, call: &LinkCall) -> Result<Value> {
        let args = call
            .args
            .iter()
            .map(|arg| self.argument(arg))
            .collect::<Result<Vec<_>>>()?;
        self.call_typed(&call.target, &args, &call.return_type)
    }

    fn store_result(&mut self, call: &LinkCall, result: Value) {
        if !call.assigned_to.is_empty() && !matches!(call.return_type.as_str(), "" | "Void") {
            self.store(&call.assigned_to, result);
        }
    }

    pub fn call(&mut self, symbol: &str, args: &[Value]) -> Result<Value> {
        self.call_typed(symbol, args, "Void")
    }

    /// Run `symbol` as an intrinsic when the VM has one, and as a function of
    /// the native library of the program otherwise.
    fn call_typed(&mut self, symbol: &str, args: &[Value], return_type: &str) -> Result<Value> {
        if let Some(intrinsic) = self.intrinsics.get(symbol).cloned() {
            return intrinsic(self, args);
        }
        self.call_native(symbol, args, return_type)?
            .ok_or_else(|| anyhow!("The native function `{symbol}` is not available in the VM"))
    }

    fn print(&mut self, ty: &str, value: &Value, newline: bool) -> Result<()> {
        let mut text = match value {
            Value::Int(number) if ty == "Char" => u32::try_from(*number)
                .ok()
                .and_then(char::from_u32)
                .unwrap_or(char::REPLACEMENT_CHARACTER)
                .to_string()
                .into_bytes(),
            Value::Int(number) => number.to_string().into_bytes(),
            Value::Float(number) => format!("{number:.6}").into_bytes(),
            Value::Str(bytes) => bytes.clone(),
            Value::Ref(_) | Value::Native { .. } | Value::Unit => {
                bail!("A value of type `{ty}` cannot be printed")
            }
        };
        if newline {
            text.push(b'\n');
        }
        self.write(&text);
        Ok(())
    }

    fn construct(&self, program: &Program, class: &str) -> Result<Instance> {
        let definition = program
            .classes
            .get(class)
            .ok_or_else(|| anyhow!("Unknown native class `{class}`"))?;
        let fields = definition
            .fields
            .iter()
            .map(|field| match &field.default {
                Some(('s', value)) => Ok(Value::Str(crate::program::decode_hex(value)?)),
                Some((_, value)) => constant(&field.ty, value),
                None if is_float(&field.ty) => Ok(Value::Float(0.0)),
                None if matches!(field.ty.as_str(), "String" | "Path") => Ok(Value::Str(vec![])),
                None => Ok(Value::Int(0)),
            })
            .collect::<Result<_>>()?;
        Ok(Instance {
            class: class.to_owned(),
            fields,
        })
    }

    pub fn allocate(&mut self, instance: Instance) -> usize {
        self.heap.push(instance);
        self.heap.len() - 1
    }
}
