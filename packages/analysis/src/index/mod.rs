mod build;
mod resolve;
mod signature;
mod symbol;

pub use resolve::{ModulePart, Resolved, Scope, resolve};
pub use symbol::{Index, NativeRef, Symbol};
