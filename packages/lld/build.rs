use std::{env, fs, path::PathBuf, process::Command};

fn main() {
    let rustc = env::var_os("RUSTC").unwrap_or_else(|| "rustc".into());
    let sysroot = Command::new(rustc).arg("--print").arg("sysroot").output()
        .expect("query rustc sysroot");
    assert!(sysroot.status.success(), "rustc --print sysroot failed");
    let sysroot = String::from_utf8(sysroot.stdout).expect("rustc sysroot is UTF-8");
    let host = env::var("HOST").expect("Cargo HOST is set");
    let rust_lld = PathBuf::from(sysroot.trim()).join("lib/rustlib").join(host).join("bin/rust-lld");
    let out = PathBuf::from(env::var_os("OUT_DIR").unwrap()).join("rust-lld");
    if rust_lld.is_file() && fs::copy(&rust_lld, &out).is_ok() {
        println!("cargo:rerun-if-changed={}", rust_lld.display());
    } else {
        let binary = find_lld().unwrap_or_else(|| {
            panic!("could not find rust-lld in the Rust sysroot or ld.lld on PATH")
        });
        fs::write(&out, []).expect("create unused embedded linker placeholder");
        println!("cargo:rerun-if-changed={}", binary.display());
        println!("cargo:rustc-env=LLD_EXTERNAL_BINARY={}", binary.display());
    }
    println!("cargo:rustc-env=LLD_EMBEDDED_BINARY={}", out.display());
}

fn find_lld() -> Option<PathBuf> {
    if let Some(path) = env::var_os("INTOLANG_LLD") {
        let path = PathBuf::from(path);
        if path.is_file() { return Some(path); }
    }
    let path = env::var_os("PATH")?;
    env::split_paths(&path)
        .flat_map(|directory| [directory.join("ld.lld"), directory.join("rust-lld")])
        .find(|candidate| candidate.is_file())
}
