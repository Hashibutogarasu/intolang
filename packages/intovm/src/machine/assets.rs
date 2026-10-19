use anyhow::{Result, anyhow, bail};
use bundle::BundlePath;
use std::path::PathBuf;
use std::rc::Rc;

use super::Vm;
use crate::inex::Inex;
use crate::native_call::cache_root;
use crate::value::Value;

/// The file that tells the bundle was unpacked completely.
const COMPLETE: &str = ".complete";

/// Unpack the bundle of `inex` once per content and give the directory.
pub(crate) fn unpack_assets(inex: &Inex) -> Result<PathBuf> {
    let directory = cache_root()?.join("assets").join(inex.bundle.digest());
    if directory.join(COMPLETE).is_file() {
        return Ok(directory);
    }
    let work = directory.with_extension(format!("unpacking-{}", std::process::id()));
    std::fs::remove_dir_all(&work).ok();
    std::fs::create_dir_all(&work)?;
    let unpacked = inex
        .unpack(&work)
        .and_then(|()| std::fs::write(work.join(COMPLETE), "").map_err(Into::into));
    if let Err(error) = unpacked {
        std::fs::remove_dir_all(&work).ok();
        return Err(error);
    }
    if std::fs::rename(&work, &directory).is_err() {
        std::fs::remove_dir_all(&work).ok();
    }
    Ok(directory)
}

/// The asset functions of the `asset` package: `require` only tells what to
/// bundle, and `asset` gives the path of an unpacked file.
pub(crate) fn install(vm: &mut Vm) {
    vm.register("asset_require", Rc::new(|_, _| Ok(Value::Unit)));
    vm.register(
        "asset_path",
        Rc::new(|vm, args| {
            let Some(Value::Str(bytes)) = args.first() else {
                bail!("`asset` takes the path of a file");
            };
            let path = String::from_utf8_lossy(bytes).into_owned();
            let root = vm
                .assets
                .as_ref()
                .ok_or_else(|| anyhow!("The program has no assets; call `require` first"))?;
            BundlePath::try_from(path.as_str())?;
            let file = root.join(&path);
            if !file.is_file() {
                bail!("The asset `{path}` is not in the bundle");
            }
            Ok(Value::Str(file.to_string_lossy().into_owned().into_bytes()))
        }),
    );
}
