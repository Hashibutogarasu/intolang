use std::{env, fs, path::PathBuf, process::Command};

fn main() {
    let rustc = env::var_os("RUSTC").unwrap_or_else(|| "rustc".into());
    let sysroot = Command::new(rustc).arg("--print").arg("sysroot").output()
        .expect("query rustc sysroot");
    assert!(sysroot.status.success(), "rustc --print sysroot failed");
    let sysroot = String::from_utf8(sysroot.stdout).expect("rustc sysroot is UTF-8");
    let host = env::var("HOST").expect("Cargo HOST is set");
    let binary = PathBuf::from(sysroot.trim()).join("lib/rustlib").join(host).join("bin/rust-lld");
    let out = PathBuf::from(env::var_os("OUT_DIR").unwrap()).join("rust-lld");
    fs::copy(&binary, &out).unwrap_or_else(|error| {
        panic!("could not embed rust-lld from {}: {error}", binary.display())
    });
    println!("cargo:rerun-if-changed={}", binary.display());
    println!("cargo:rustc-env=LLD_EMBEDDED_BINARY={}", out.display());
}
