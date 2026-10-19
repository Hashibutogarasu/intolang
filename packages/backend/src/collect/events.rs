use ast::Program;

use crate::context::EventInfo;

/// The name of the event that converts values; its handlers are cast
/// handlers, not events to listen to.
const CAST_EVENT: &str = "onCast";

/// Collect the events declared inside the classes of `program`, as the class
/// they belong to, their name and what they declare.
pub fn collect_class_events(program: &Program) -> Vec<(String, String, EventInfo)> {
    program
        .classes()
        .flat_map(|class| {
            class
                .events
                .iter()
                .filter(|event| event.name != CAST_EVENT)
                .map(|event| {
                    (
                        class.name.clone(),
                        event.name.clone(),
                        EventInfo {
                            params: event.params.clone(),
                            body: event.body.clone(),
                        },
                    )
                })
        })
        .collect()
}

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
