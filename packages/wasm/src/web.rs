//! What JavaScript sees.

use std::cell::RefCell;

use wasm_bindgen::prelude::wasm_bindgen;

use crate::{Session, read_builtin, run};

thread_local! {
    static SESSION: RefCell<Session> = RefCell::new(Session::new());
}

/// Run a program; the answer is `{"stdout", "error"}` as JSON.
#[wasm_bindgen(js_name = run)]
pub fn run_program(source: &str, stdin: &str) -> String {
    serde_json::to_string(&run(source, stdin)).expect("an outcome is JSON")
}

/// Answer a request of `ipc` given as JSON.
#[wasm_bindgen(js_name = handle)]
pub fn handle(request: &str) -> String {
    SESSION.with(|session| session.borrow_mut().handle(request))
}

/// Take a notification of `ipc` given as JSON; the answer is the array of
/// notifications it causes.
#[wasm_bindgen(js_name = notify)]
pub fn notify(notification: &str) -> String {
    SESSION.with(|session| session.borrow_mut().notify(notification))
}

/// The text of a file of the standard package, by the path a definition names.
#[wasm_bindgen(js_name = readBuiltin)]
pub fn read_builtin_file(path: &str) -> Option<String> {
    read_builtin(path)
}
