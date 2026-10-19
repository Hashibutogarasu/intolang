use anyhow::{anyhow, bail};
use bundle::BundlePath;
use std::rc::Rc;

use super::Vm;
use super::binary::make;
use crate::value::Value;

/// The asset functions of the `asset` package: `require` and `asset` only tell
/// what to bundle, `asset` giving its path back, and `Assets.read` reads a
/// file of the bundle into a `Binary`.
pub(crate) fn install(vm: &mut Vm) {
    vm.register("asset_require", Rc::new(|_, _| Ok(Value::Unit)));
    vm.register(
        "asset_path",
        Rc::new(|_, args| {
            args.first()
                .cloned()
                .ok_or_else(|| anyhow!("`asset` takes a path"))
        }),
    );
    vm.register(
        "asset_read",
        Rc::new(|vm, args| {
            let Some(Value::Str(bytes)) = args.first() else {
                bail!("`Assets.read` takes the path of a file");
            };
            let path = String::from_utf8_lossy(bytes).into_owned();
            let bundle = vm
                .assets
                .as_ref()
                .ok_or_else(|| anyhow!("The program has no assets; call `require` first"))?;
            BundlePath::try_from(path.as_str())?;
            let contents = bundle
                .read(&path)
                .ok_or_else(|| anyhow!("The asset `{path}` is not in the bundle"))?;
            Ok(Value::Int(make(contents) as i64))
        }),
    );
}
