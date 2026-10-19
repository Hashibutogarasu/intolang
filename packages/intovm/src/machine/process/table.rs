use std::collections::{BTreeMap, HashMap, HashSet, VecDeque};

/// How a virtual process ended, or that it has not.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProcessState {
    Running,
    Exited(i32),
    Signaled(i32),
}

/// What a spawned program does on the virtual clock.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ChildBehavior {
    pub code: i32,
    /// How long it runs before it exits by itself; `None` is forever.
    pub runs_for_ms: Option<u64>,
}

/// One entry of the virtual process table.
#[derive(Debug, Clone)]
pub struct ProcessInfo {
    pub pid: i32,
    pub parent: Option<i32>,
    pub state: ProcessState,
    pub reaped: bool,
    pub path: String,
    pub args: String,
    pub(super) exits_at: Option<u64>,
    pub(super) exit_code: i32,
    pub(super) queue: VecDeque<i32>,
    pub(super) watching: HashSet<i32>,
}

impl ProcessInfo {
    pub(super) fn new(pid: i32, parent: Option<i32>) -> Self {
        Self {
            pid,
            parent,
            state: ProcessState::Running,
            reaped: false,
            path: String::new(),
            args: String::new(),
            exits_at: None,
            exit_code: 0,
            queue: VecDeque::new(),
            watching: HashSet::new(),
        }
    }

    /// The signals queued for the process, oldest first.
    pub fn pending_signals(&self) -> Vec<i32> {
        self.queue.iter().copied().collect()
    }
}

/// What a `Process` or `Signal` instance on the heap stands for.
#[derive(Debug, Clone, Copy)]
pub(super) struct Handle {
    pub pid: i32,
    pub current: bool,
}

/// The virtual processes, the handles to them and the virtual clock.
#[derive(Debug, Default)]
pub struct ProcessTable {
    pub(super) processes: BTreeMap<i32, ProcessInfo>,
    pub(super) handles: HashMap<usize, Handle>,
    pub(super) programs: HashMap<String, ChildBehavior>,
    pub(super) current_pid: i32,
    pub(super) current_taken: bool,
    pub(super) next_pid: i32,
    pub(super) clock_ms: u64,
    pub(super) requested_code: Option<i32>,
    pub(in crate::machine) terminated: bool,
}

impl ProcessTable {
    pub(in crate::machine) fn new(
        current_pid: i32,
        clock_ms: u64,
        programs: HashMap<String, ChildBehavior>,
    ) -> Self {
        let mut processes = BTreeMap::new();
        processes.insert(current_pid, ProcessInfo::new(current_pid, None));
        Self {
            processes,
            handles: HashMap::new(),
            programs,
            current_pid,
            current_taken: false,
            next_pid: current_pid + 1,
            clock_ms,
            requested_code: None,
            terminated: false,
        }
    }

    pub fn get(&self, pid: i32) -> Option<&ProcessInfo> {
        self.processes.get(&pid)
    }

    /// Every process, by pid.
    pub fn all(&self) -> impl Iterator<Item = &ProcessInfo> {
        self.processes.values()
    }

    pub fn clock(&self) -> u64 {
        self.clock_ms
    }

    pub fn current_pid(&self) -> i32 {
        self.current_pid
    }

    /// Move the virtual clock forward; children whose time is up exit.
    pub fn advance(&mut self, milliseconds: u64) {
        self.clock_ms += milliseconds;
        self.refresh();
    }

    pub(super) fn refresh(&mut self) {
        let clock = self.clock_ms;
        for process in self.processes.values_mut() {
            if process.state == ProcessState::Running
                && process.exits_at.is_some_and(|time| time <= clock)
            {
                process.state = ProcessState::Exited(process.exit_code);
            }
        }
    }

    pub(super) fn behavior_of(&self, path: &str, args: &str) -> ChildBehavior {
        if let Some(behavior) = self.programs.get(path) {
            return *behavior;
        }
        let basename = |path: &str| path.rsplit('/').next().unwrap_or(path).to_owned();
        let mut name = basename(path);
        let mut args = args;
        if name == "env" {
            let (program, rest) = args.split_once(' ').unwrap_or((args, ""));
            name = basename(program);
            args = rest;
        }
        let name = name.as_str();
        let seconds = args
            .split_whitespace()
            .next()
            .and_then(|word| word.parse::<u64>().ok());
        match name {
            "false" => ChildBehavior {
                code: 1,
                runs_for_ms: Some(0),
            },
            "sleep" => ChildBehavior {
                code: 0,
                runs_for_ms: seconds.map(|seconds| seconds * 1000),
            },
            _ => ChildBehavior {
                code: 0,
                runs_for_ms: Some(0),
            },
        }
    }
}
