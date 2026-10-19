use anyhow::{Result, anyhow};
use std::rc::Rc;

use super::table::{Handle, ProcessState};
use crate::machine::Vm;
use crate::value::{Instance, Value};

const NULL: Value = Value::Int(0);

fn new_handle(vm: &mut Vm, class: &str, fields: Vec<Value>, pid: i32, current: bool) -> Value {
    let reference = vm.allocate(Instance {
        class: class.to_owned(),
        fields,
    });
    vm.processes
        .handles
        .insert(reference, Handle { pid, current });
    Value::Ref(reference)
}

/// The pid a `Process` or `Signal` handle stands for; `None` for the null
/// handle a failed call returns.
fn pid_of(vm: &Vm, value: &Value) -> Result<Option<i32>> {
    match value {
        Value::Ref(reference) => vm
            .processes
            .handles
            .get(reference)
            .map(|handle| Some(handle.pid))
            .ok_or_else(|| anyhow!("The value is not a process handle")),
        Value::Int(0) => Ok(None),
        other => Err(anyhow!("{other:?} is not a process handle")),
    }
}

fn int(value: &Value) -> Result<i32> {
    value
        .int()
        .map(|number| number as i32)
        .ok_or_else(|| anyhow!("{value:?} is not an integer"))
}

fn text(value: &Value) -> String {
    match value {
        Value::Str(bytes) => String::from_utf8_lossy(bytes).into_owned(),
        _ => String::new(),
    }
}

fn argument(args: &[Value], index: usize) -> Result<&Value> {
    args.get(index)
        .ok_or_else(|| anyhow!("The native call is missing argument {index}"))
}

fn release(vm: &mut Vm, args: &[Value]) -> Result<Value> {
    if let Value::Ref(reference) = argument(args, 0)?
        && let Some(handle) = vm.processes.handles.remove(reference)
        && handle.current
    {
        vm.processes.current_taken = false;
    }
    Ok(Value::Unit)
}

/// The handle of the current process, which can be taken once until it is
/// released.
fn take_current(vm: &mut Vm, class: &str) -> Value {
    if vm.processes.current_taken {
        return NULL;
    }
    vm.processes.current_taken = true;
    let pid = vm.processes.current_pid;
    new_handle(vm, class, vec![Value::Int(i64::from(pid))], pid, true)
}

fn spawn_child(vm: &mut Vm, class: &str, args: &[Value]) -> Result<Value> {
    let pid = vm
        .processes
        .spawn(&text(argument(args, 0)?), &text(argument(args, 1)?));
    Ok(new_handle(
        vm,
        class,
        vec![Value::Int(i64::from(pid))],
        pid,
        false,
    ))
}

/// The functions `DummyProcess` binds to, which only the VM has.
fn install_dummy_process(vm: &mut Vm) {
    vm.register(
        "dummy_process_current",
        Rc::new(|vm, _| Ok(take_current(vm, "DummyProcess"))),
    );
    vm.register(
        "dummy_process_spawn",
        Rc::new(|vm, args| spawn_child(vm, "DummyProcess", args)),
    );
    vm.register(
        "dummy_process_alive",
        Rc::new(|vm, args| {
            let pid = int(argument(args, 0)?)?;
            vm.processes.refresh();
            let alive = vm
                .processes
                .get(pid)
                .is_some_and(|process| process.state == ProcessState::Running);
            Ok(Value::Int(i64::from(alive)))
        }),
    );
    vm.register(
        "dummy_process_advance",
        Rc::new(|vm, args| {
            let milliseconds = int(argument(args, 0)?)?;
            vm.processes
                .advance(u64::try_from(milliseconds).unwrap_or(0));
            Ok(Value::Unit)
        }),
    );
}

/// Run the `ps` package's C functions and the clock against the virtual
/// process table unless the VM runs a program for real, so that tests never
/// start, signal or end a real process and never wait for real time.
pub(crate) fn install(vm: &mut Vm, real_processes: bool) {
    install_dummy_process(vm);
    if !real_processes {
        install_virtual_processes(vm);
        install_virtual_time(vm);
    }
}

fn install_virtual_time(vm: &mut Vm) {
    vm.register(
        "standard_sleep",
        Rc::new(|vm, args| {
            let milliseconds = int(argument(args, 0)?)?;
            vm.processes
                .advance(u64::try_from(milliseconds).unwrap_or(0));
            Ok(Value::Unit)
        }),
    );
    vm.register(
        "standard_monotonic_milliseconds",
        Rc::new(|vm, _| Ok(Value::Int(vm.processes.clock_ms as i64))),
    );
}

fn install_virtual_processes(vm: &mut Vm) {
    vm.register(
        "ps_process_current",
        Rc::new(|vm, _| Ok(take_current(vm, "Process"))),
    );
    vm.register(
        "ps_process_attach",
        Rc::new(|vm, args| {
            let pid = int(argument(args, 0)?)?;
            if pid <= 0 {
                return Ok(NULL);
            }
            Ok(new_handle(
                vm,
                "Process",
                vec![Value::Int(i64::from(pid))],
                pid,
                false,
            ))
        }),
    );
    vm.register(
        "ps_process_spawn",
        Rc::new(|vm, args| spawn_child(vm, "Process", args)),
    );
    vm.register(
        "ps_process_signal",
        Rc::new(|vm, args| {
            let Some(pid) = pid_of(vm, argument(args, 0)?)? else {
                return Ok(NULL);
            };
            Ok(new_handle(vm, "Signal", Vec::new(), pid, false))
        }),
    );
    vm.register(
        "ps_process_usage",
        Rc::new(|vm, _| {
            let micros = vm.processes.clock_ms as i64 * 1000;
            Ok(Value::Ref(vm.allocate(Instance {
                class: "Usage".to_owned(),
                fields: vec![Value::Int(micros), Value::Int(0), Value::Int(1024)],
            })))
        }),
    );
    vm.register("ps_usage_release", Rc::new(|_, _| Ok(Value::Unit)));
    vm.register(
        "ps_process_kill",
        Rc::new(|vm, args| {
            let status = match pid_of(vm, argument(args, 0)?)? {
                Some(pid) => vm.processes.send(pid, super::actions::SIGKILL),
                None => -1,
            };
            Ok(Value::Int(i64::from(status)))
        }),
    );
    vm.register(
        "ps_process_reap",
        Rc::new(|vm, args| {
            let status = match pid_of(vm, argument(args, 0)?)? {
                Some(pid) => vm.processes.reap(pid)?,
                None => -1,
            };
            Ok(Value::Int(i64::from(status)))
        }),
    );
    vm.register("ps_process_release", Rc::new(|vm, args| release(vm, args)));
    vm.register("ps_process_detach", Rc::new(|vm, args| release(vm, args)));
    vm.register(
        "ps_signal_send",
        Rc::new(|vm, args| {
            let number = int(argument(args, 1)?)?;
            let status = match pid_of(vm, argument(args, 0)?)? {
                Some(pid) => vm.processes.send(pid, number),
                None => -1,
            };
            Ok(Value::Int(i64::from(status)))
        }),
    );
    vm.register(
        "ps_signal_watch",
        Rc::new(|vm, args| {
            vm.processes.watch(int(argument(args, 0)?)?);
            Ok(Value::Unit)
        }),
    );
    vm.register(
        "ps_signal_next",
        Rc::new(|vm, _| Ok(Value::Int(i64::from(vm.processes.next_signal())))),
    );
    vm.register(
        "ps_exit_request",
        Rc::new(|vm, args| {
            vm.processes.requested_code = Some(int(argument(args, 0)?)?);
            Ok(Value::Unit)
        }),
    );
}
