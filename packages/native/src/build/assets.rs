use anyhow::Result;
use parser::Record;
use std::collections::BTreeSet;
use types::MessageTag;

use crate::arg::NativeArg;
use crate::call::LinkCall;

/// The native functions of the calls `require("directory")` and
/// `asset("path")`, each of which names what the program bundles.
const BUNDLING: [&str; 2] = ["asset_require", "asset_path"];

/// The directories and files the `require` and `asset` calls of `message`
/// name with a literal path, once each, in the order of their names.
pub fn required_assets(message: &str) -> Result<Vec<String>> {
    let mut paths = BTreeSet::new();
    for line in message.lines() {
        let Ok(record) = line.parse::<Record>() else {
            continue;
        };
        if record.tag() != MessageTag::LinkAssign {
            continue;
        }
        let call = LinkCall::try_from(&record)?;
        if !BUNDLING.contains(&call.target.as_str()) {
            continue;
        }
        if let [NativeArg::Str(bytes)] = call.args.as_slice() {
            paths.insert(String::from_utf8_lossy(bytes).into_owned());
        }
    }
    Ok(paths.into_iter().collect())
}
