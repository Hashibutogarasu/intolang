use std::cell::RefCell;
use std::rc::Rc;

use super::{RealFs, Vfs};

thread_local! {
    static CURRENT: RefCell<Option<Rc<dyn Vfs>>> = const { RefCell::new(None) };
}

/// The files the code on this thread reads: those given to the closest
/// `with_vfs`, and the disk when there is none.
pub fn current() -> Rc<dyn Vfs> {
    CURRENT.with(|current| current.borrow().clone().unwrap_or_else(|| Rc::new(RealFs)))
}

/// Whether the files read now are the disk of the computer.
pub fn is_real() -> bool {
    current().is_real()
}

/// Run `body` with `files` as the files every package function reads.
pub fn with_vfs<T>(files: Rc<dyn Vfs>, body: impl FnOnce() -> T) -> T {
    let previous = CURRENT.with(|current| current.replace(Some(files)));
    let result = body();
    CURRENT.with(|current| *current.borrow_mut() = previous);
    result
}
