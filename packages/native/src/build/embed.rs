use anyhow::{Context, Result, bail};
use std::path::{Path, PathBuf};
use types::Language;

use super::compile::toolchain;

/// Compile an object next to `output` that holds `contents` as the symbols
/// `into_bundle_data` and `into_bundle_size`, and `digest` as the text
/// `into_bundle_digest`, which the asset functions of a program read.
pub fn embed_bundle(contents: &[u8], digest: &str, output: &Path) -> Result<PathBuf> {
    let identifier = std::process::id();
    let data = output.with_file_name(format!(".intolang-bundle-{identifier}.bin"));
    let assembly = output.with_file_name(format!(".intolang-bundle-{identifier}.S"));
    let object = output.with_file_name(format!(".intolang-bundle-{identifier}.o"));
    std::fs::write(&data, contents)
        .with_context(|| format!("Could not write {}", data.display()))?;
    let source = format!(
        "    .section .rodata\n    .balign 16\n    .global into_bundle_data\ninto_bundle_data:\n    .incbin \"{}\"\n    .balign 8\n    .global into_bundle_size\ninto_bundle_size:\n    .quad {}\n    .global into_bundle_digest\ninto_bundle_digest:\n    .asciz \"{digest}\"\n    .section .note.GNU-stack,\"\",@progbits\n",
        data.display(),
        contents.len()
    );
    std::fs::write(&assembly, source)
        .with_context(|| format!("Could not write {}", assembly.display()))?;
    let (variable, default) = toolchain(Language::C);
    let command = std::env::var(variable).unwrap_or_else(|_| default.into());
    let result = std::process::Command::new(command)
        .arg("-c")
        .arg(&assembly)
        .arg("-o")
        .arg(&object)
        .output()
        .with_context(|| format!("Could not assemble {}", assembly.display()));
    std::fs::remove_file(&data).ok();
    std::fs::remove_file(&assembly).ok();
    let result = result?;
    if !result.status.success() {
        bail!(
            "Embedding the bundle failed:\n{}",
            String::from_utf8_lossy(&result.stderr)
        );
    }
    Ok(object)
}
