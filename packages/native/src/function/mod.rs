mod call;
mod definition;
mod reference;
mod result;

pub use call::FunctionCall;
pub use definition::FunctionDefinition;
pub use reference::{FunctionRef, IndirectCall};
pub use result::FunctionReturn;
