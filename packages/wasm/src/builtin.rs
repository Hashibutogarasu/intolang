use std::path::Path;
use std::rc::Rc;

use package::{MemoryFs, Vfs};

include!(concat!(env!("OUT_DIR"), "/builtin.rs"));

/// Where the packages stand in the files the module reads.
pub const PACKAGES_DIR: &str = "/virtual/packages";

/// The directory of the standard package inside [`PACKAGES_DIR`].
const BUILTIN_DIR: &str = "intolang";

thread_local! {
    static FILES: Rc<MemoryFs> = Rc::new(
        EMBEDDED
            .iter()
            .map(|(path, contents)| {
                (
                    Path::new(PACKAGES_DIR).join(BUILTIN_DIR).join(path),
                    *contents,
                )
            })
            .collect(),
    );
}

/// The files the playground reads: the standard package.
pub fn builtin_files() -> Rc<dyn Vfs> {
    FILES.with(|files| files.clone() as Rc<dyn Vfs>)
}

/// The text of a file of the standard package, by the path a definition names.
pub fn read_builtin(path: &str) -> Option<String> {
    FILES.with(|files| files.read_to_string(Path::new(path)).ok())
}
