use std::fmt::Write as _;
use std::fs;
use std::path::{Path, PathBuf};

/// The files of the builtin package the playground carries: the manifest and
/// every module. The C sources are left out, the VM provides their functions.
const PACKAGE: &str = "../intolang";
const MANIFEST: &str = "into-package.json";
const MODULES: &str = "src/main";

fn main() {
    let package = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join(PACKAGE)
        .canonicalize()
        .expect("the builtin package");
    println!("cargo:rerun-if-changed={}", package.display());

    let mut files = vec![package.join(MANIFEST)];
    modules(&package.join(MODULES), &mut files);
    files.sort();

    let mut table = String::from("pub const EMBEDDED: &[(&str, &str)] = &[\n");
    for file in files {
        let relative = file.strip_prefix(&package).expect("inside the package");
        writeln!(
            table,
            "    ({:?}, include_str!({:?})),",
            relative.to_string_lossy(),
            file.to_string_lossy()
        )
        .unwrap();
    }
    table.push_str("];\n");

    let out = PathBuf::from(std::env::var_os("OUT_DIR").expect("OUT_DIR")).join("builtin.rs");
    fs::write(out, table).expect("write the table of builtin files");
}

fn modules(directory: &Path, files: &mut Vec<PathBuf>) {
    for entry in fs::read_dir(directory).expect("read the modules") {
        let path = entry.expect("read a module entry").path();
        if path.is_dir() {
            modules(&path, files);
        } else if path
            .extension()
            .is_some_and(|extension| extension == "into")
        {
            files.push(path);
        }
    }
}
