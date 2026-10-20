use errors::{Errors, Result};
use std::path::{Path, PathBuf};
use types::Language;

use super::compile::toolchain;
use super::failure::{tool_failure, tool_not_run};

/// Compile an object next to `output` that holds `contents` as the symbols
/// `into_bundle_data` and `into_bundle_size`, which the asset functions of a
/// program read.
pub fn embed_bundle(contents: &[u8], output: &Path) -> Result<PathBuf> {
    let identifier = std::process::id();
    let data = output.with_file_name(format!(".intolang-bundle-{identifier}.bin"));
    let assembly = output.with_file_name(format!(".intolang-bundle-{identifier}.S"));
    let object = output.with_file_name(format!(".intolang-bundle-{identifier}.o"));
    std::fs::write(&data, contents).map_err(|error| Errors::write(&data, error))?;
    let source = format!(
        "    .section .rodata\n    .balign 16\n    .global into_bundle_data\ninto_bundle_data:\n    .incbin \"{}\"\n    .balign 8\n    .global into_bundle_size\ninto_bundle_size:\n    .quad {}\n    .section .note.GNU-stack,\"\",@progbits\n",
        data.display(),
        contents.len()
    );
    std::fs::write(&assembly, source).map_err(|error| Errors::write(&assembly, error))?;
    let (variable, default) = toolchain(Language::C);
    let command = std::env::var(variable).unwrap_or_else(|_| default.into());
    let result = std::process::Command::new(&command)
        .arg("-c")
        .arg(&assembly)
        .arg("-o")
        .arg(&object)
        .output();
    std::fs::remove_file(&data).ok();
    std::fs::remove_file(&assembly).ok();
    let result = result.map_err(|error| tool_not_run(&command, &error))?;
    if !result.status.success() {
        return Err(tool_failure(
            Errors::BundleEmbedFailed,
            &command,
            &result.stderr,
        ));
    }
    Ok(object)
}
