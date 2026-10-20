use errors::{Errors, Result};

use super::table::{ProcessInfo, ProcessState, ProcessTable};

pub const SIGKILL: i32 = 9;
pub const SIGCHLD: i32 = 17;

/// Whether the default action of the signal ends the process.
fn terminates(signal: i32) -> bool {
    signal != SIGCHLD
}

impl ProcessTable {
    /// Start a virtual child of the current process.
    pub(super) fn spawn(&mut self, path: &str, args: &str) -> i32 {
        let pid = self.next_pid;
        self.next_pid += 1;
        let behavior = self.behavior_of(path, args);
        let mut process = ProcessInfo::new(pid, Some(self.current_pid));
        process.path = path.to_owned();
        process.args = args.to_owned();
        process.exit_code = behavior.code;
        process.exits_at = behavior.runs_for_ms.map(|time| self.clock_ms + time);
        self.processes.insert(pid, process);
        self.refresh();
        pid
    }

    /// `kill(pid, signal)`: 0 when the signal was delivered, -1 when no such
    /// process exists.
    pub(in crate::machine) fn send(&mut self, pid: i32, signal: i32) -> i32 {
        self.refresh();
        let current = pid == self.current_pid;
        let Some(process) = self.processes.get_mut(&pid) else {
            return -1;
        };
        if process.reaped {
            return -1;
        }
        if signal == 0 || process.state != ProcessState::Running {
            return 0;
        }
        if signal != SIGKILL && process.watching.contains(&signal) {
            process.queue.push_back(signal);
            return 0;
        }
        if terminates(signal) {
            process.state = ProcessState::Signaled(signal);
            process.exits_at = None;
            if current {
                self.terminated = true;
            }
        }
        0
    }

    /// Wait for the child, moving the virtual clock to the time it exits.
    /// The result is its exit code, or the negated number of the signal that
    /// ended it.
    pub(super) fn reap(&mut self, pid: i32) -> Result<i32> {
        if pid == self.current_pid {
            return Ok(-1);
        }
        self.refresh();
        let Some(process) = self.processes.get(&pid) else {
            return Ok(-1);
        };
        if process.state == ProcessState::Running {
            let Some(time) = process.exits_at else {
                return Err(errors::Error::from(Errors::ProcessDeadlock {
                    pid: pid.to_string(),
                }));
            };
            self.clock_ms = self.clock_ms.max(time);
            self.refresh();
        }
        let current = self.current_pid;
        let process = self
            .processes
            .get_mut(&pid)
            .expect("the process was found above");
        process.reaped = true;
        let status = match process.state {
            ProcessState::Exited(code) => code,
            ProcessState::Signaled(signal) => -signal,
            ProcessState::Running => -1,
        };
        if let Some(parent) = self.processes.get_mut(&current)
            && parent.watching.contains(&SIGCHLD)
        {
            parent.queue.push_back(SIGCHLD);
        }
        Ok(status)
    }

    pub(super) fn watch(&mut self, signal: i32) {
        let current = self.current_pid;
        if let Some(process) = self.processes.get_mut(&current) {
            process.watching.insert(signal);
        }
    }

    pub(super) fn next_signal(&mut self) -> i32 {
        let current = self.current_pid;
        self.processes
            .get_mut(&current)
            .and_then(|process| process.queue.pop_front())
            .unwrap_or(0)
    }

    /// The current process ends once the program is over.
    pub(in crate::machine) fn finish(&mut self) {
        if self.terminated {
            return;
        }
        let code = self.requested_code.unwrap_or(0);
        let current = self.current_pid;
        if let Some(process) = self.processes.get_mut(&current)
            && process.state == ProcessState::Running
        {
            process.state = ProcessState::Exited(code);
        }
    }
}
