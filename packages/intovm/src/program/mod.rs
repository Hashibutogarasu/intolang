mod instruction;
mod load;

pub use instruction::{Cond, Instr, Program};
pub use load::decode_hex;
