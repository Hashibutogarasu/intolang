use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

const INTO: &str = env!("CARGO_BIN_EXE_intolang");

const PROGRAM: &str = r#"import "intolang://builtin/std";
import "ps://process";
import "ps://events";
import "cpp://standard/time";

now into self = Process.current();
now into stop = 0;

now into onTerm = self.signal.on(Signal.SIGTERM, (signal) => {
    stop = 1;
});

now into watcher = thread {
    while (stop == 0) {
        dispatch(self);
        sleep(10);
    }
};

now into task = watcher.run();
println("ready");

init {
    while (stop == 0) {
        sleep(5);
    }
}

await task;
println("stopped");
exit(self, 3);
"#;

struct Project {
    root: PathBuf,
}

impl Project {
    fn new(entry: &str) -> Self {
        let root =
            std::env::temp_dir().join(format!("intolang-inex-signal-test-{}", std::process::id()));
        std::fs::create_dir_all(root.join("src/main")).unwrap();
        std::fs::write(
            root.join("into-package.json"),
            "{\"name\":\"watch\",\"alias\":\"watch\",\"version\":\"0.1.0\",\"LICENCE\":\"MIT\",\"runs\":\"src/main/watch.into\"}",
        )
        .unwrap();
        std::fs::write(root.join("src/main/watch.into"), entry).unwrap();
        Self { root }
    }

    fn inex(&self) -> PathBuf {
        self.root
            .join("dist")
            .join(std::env::consts::OS)
            .join(std::env::consts::ARCH)
            .join("watch.inex")
    }
}

impl Drop for Project {
    fn drop(&mut self) {
        std::fs::remove_dir_all(&self.root).ok();
    }
}

fn signal(pid: u32, name: &str) {
    Command::new("kill")
        .arg(format!("-{name}"))
        .arg(pid.to_string())
        .status()
        .unwrap();
}

#[test]
fn a_program_in_the_vm_leaves_its_loops_and_exits_when_it_is_terminated() {
    let project = Project::new(PROGRAM);
    let built = Command::new(INTO)
        .args([Path::new("build"), Path::new("--vm"), &project.root])
        .output()
        .unwrap();
    assert!(built.status.success(), "{built:?}");

    let mut child = Command::new(INTO)
        .arg(project.inex())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    let pid = child.id();
    let watchdog = std::thread::spawn(move || {
        std::thread::sleep(Duration::from_secs(120));
        signal(pid, "KILL");
    });

    let mut lines = BufReader::new(child.stdout.take().unwrap()).lines();
    assert_eq!(lines.next().unwrap().unwrap(), "ready");
    signal(pid, "TERM");

    let rest: Vec<String> = lines.map(Result::unwrap).collect();
    let started = Instant::now();
    let status = child.wait().unwrap();
    assert!(started.elapsed() < Duration::from_secs(60));
    assert_eq!(rest, vec!["stopped".to_owned()]);
    assert_eq!(status.code(), Some(3));
    drop(watchdog);
}
