use native::NativeLink;
use std::collections::HashMap;

use crate::context::{Overload, UserFunction};

/// Group the functions that share a name and differ in the types of their
/// parameters; they are stored as `name$Type$Type`.
pub fn collect_overloads(
    functions: &HashMap<String, UserFunction>,
    links: &HashMap<String, NativeLink>,
) -> HashMap<String, Vec<Overload>> {
    let mut overloads: HashMap<String, Vec<Overload>> = HashMap::new();
    let declared = functions
        .iter()
        .map(|(key, function)| (key, function.params.as_slice()))
        .chain(
            links
                .iter()
                .map(|(key, link)| (key, link.params.as_slice())),
        );
    for (key, params) in declared {
        let Some((base, _)) = key.split_once('$') else {
            continue;
        };
        overloads
            .entry(base.to_owned())
            .or_default()
            .push(Overload {
                key: key.clone(),
                params: params
                    .iter()
                    .map(|param| param.ty.as_ref().map(ToString::to_string))
                    .collect(),
                required: params
                    .iter()
                    .filter(|param| param.default_argument().is_none())
                    .count(),
            });
    }
    for candidates in overloads.values_mut() {
        candidates.sort_by(|left, right| left.key.cmp(&right.key));
    }
    overloads
}
