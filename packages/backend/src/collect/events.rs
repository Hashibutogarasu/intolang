use ast::Program;

use crate::context::EventInfo;

/// Collect the module level events of `program`.
pub fn collect_events(program: &Program) -> Vec<(String, EventInfo)> {
    program
        .events()
        .map(|event| {
            (
                event.name.clone(),
                EventInfo {
                    params: event.params.clone(),
                    body: event.body.clone(),
                },
            )
        })
        .collect()
}
