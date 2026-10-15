use anyhow::{Context, Result, bail};
use std::path::Path;

/// Link an LLVM-generated object with the rust-lld binary embedded at build time.
pub fn link(object: &Path, output: &Path) -> Result<()> {
    link_with_args(object, output, false)
}

/// Link an LLVM-generated object as a native shared library.
pub fn link_shared(object: &Path, output: &Path) -> Result<()> {
    link_with_args(object, output, true)
}

fn link_with_args(object: &Path, output: &Path, shared: bool) -> Result<()> {
    let lld_dir = std::env::temp_dir().join(format!("intolang-lld-{}", std::process::id()));
    std::fs::create_dir_all(&lld_dir)?;
    let lld_path = lld_dir.join("ld.lld");
    std::fs::write(&lld_path, include_bytes!(env!("LLD_EMBEDDED_BINARY")))?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&lld_path, std::fs::Permissions::from_mode(0o755))?;
    }
    let prefix = format!("-B{}/", lld_dir.display());
    let cc = std::env::var("CC").unwrap_or_else(|_| "cc".into());
    let mut command = std::process::Command::new(cc);
    command.arg(prefix).arg("-fuse-ld=lld");
    if shared { command.arg("-shared"); }
    let result = command.arg(object).arg("-o").arg(output)
        .output().context("could not start C compiler driver")?;
    std::fs::remove_dir_all(lld_dir).ok();
    if !result.status.success() {
        bail!("LLD link failed ({}):\n{}", result.status, String::from_utf8_lossy(&result.stderr));
    }
    Ok(())
}
