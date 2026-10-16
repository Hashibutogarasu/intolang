use std::{
    env, fs,
    path::{Path, PathBuf},
};

fn copy_dir(source: &Path, target: &Path) -> std::io::Result<()> {
    fs::create_dir_all(target)?;
    for entry in fs::read_dir(source)? {
        let entry = entry?;
        let to = target.join(entry.file_name());
        if entry.file_type()?.is_dir() {
            copy_dir(&entry.path(), &to)?;
        } else {
            fs::copy(entry.path(), to)?;
        }
    }
    Ok(())
}

fn main() {
    let manifest_dir = PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").unwrap());
    let workspace = manifest_dir.join("../..");
    let resources = manifest_dir.join("res");
    let bundled = PathBuf::from(env::var_os("OUT_DIR").unwrap()).join("res");

    println!("cargo:rerun-if-changed={}", resources.display());
    println!(
        "cargo:rerun-if-changed={}",
        workspace.join("packages/intolang").display()
    );
    println!(
        "cargo:rerun-if-changed={}",
        workspace.join("packages/system/modules").display()
    );
    println!(
        "cargo:rerun-if-changed={}",
        workspace
            .join("packages/system/into-package.json")
            .display()
    );
    println!(
        "cargo:rerun-if-changed={}",
        workspace.join("packages").display()
    );
    copy_dir(&resources, &bundled).expect("copy compiler resources");
    copy_dir(
        &workspace.join("packages/intolang"),
        &bundled.join("packages/intolang"),
    )
    .expect("embed intolang builtin package");
    for entry in fs::read_dir(workspace.join("packages")).expect("read workspace packages") {
        let entry = entry.expect("read package directory entry");
        if !entry.file_type().expect("read package entry type").is_dir() {
            continue;
        }
        if matches!(entry.file_name().to_str(), Some("intolang" | "system")) {
            continue;
        }
        let manifest = entry.path().join("into-package.json");
        let Ok(contents) = fs::read(&manifest) else {
            continue;
        };
        let Ok(manifest): std::result::Result<serde_json::Value, _> =
            serde_json::from_slice(&contents)
        else {
            continue;
        };
        let Some(alias) = manifest.get("alias").and_then(serde_json::Value::as_str) else {
            continue;
        };
        copy_dir(&entry.path(), &bundled.join("packages").join(alias))
            .expect("embed third-party package");
    }
    let system_manifest = workspace.join("packages/system/into-package.json");
    let system: serde_json::Value =
        serde_json::from_slice(&fs::read(&system_manifest).expect("read system package manifest"))
            .expect("parse system package manifest");
    for (module, path) in system
        .get("modules")
        .and_then(serde_json::Value::as_object)
        .expect("system package modules")
    {
        copy_dir(
            &workspace
                .join("packages/system")
                .join(path.as_str().expect("module path")),
            &bundled.join("packages").join(module),
        )
        .expect("embed system language module package");
    }

    println!("cargo:rustc-env=COMPILER_BUNDLED_RES={}", bundled.display());
}
