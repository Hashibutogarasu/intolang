use anyhow::Result;
use parser::Record;
use std::collections::BTreeSet;
use types::MessageTag;

use crate::arg::NativeArg;
use crate::call::LinkCall;

/// The native function the call `require("directory")` is, which the program
/// bundles the directory for.
const REQUIRE: &str = "asset_require";

/// The directories the `require` calls of `message` name, once each, in the
/// order of their names.
pub fn required_assets(message: &str) -> Result<Vec<String>> {
    let mut directories = BTreeSet::new();
    for line in message.lines() {
        let Ok(record) = line.parse::<Record>() else {
            continue;
        };
        if record.tag() != MessageTag::LinkAssign {
            continue;
        }
        let call = LinkCall::try_from(&record)?;
        if call.target != REQUIRE {
            continue;
        }
        if let [NativeArg::Str(bytes)] = call.args.as_slice() {
            directories.insert(String::from_utf8_lossy(bytes).into_owned());
        }
    }
    Ok(directories.into_iter().collect())
}
