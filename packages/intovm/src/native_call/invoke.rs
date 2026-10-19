use std::ffi::c_void;
use std::mem::transmute;

/// How a native function returns its result.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Returns {
    Integer,
    Float32,
    Float64,
}

/// What a native function returned.
#[derive(Debug, Clone, Copy)]
pub enum Returned {
    Integer(usize),
    Float(f64),
}

macro_rules! dispatch {
    ($function:expr, $arguments:expr, $result:ty) => {
        match $arguments {
            [] => transmute::<_, extern "C" fn() -> $result>($function)(),
            [a] => transmute::<_, extern "C" fn(usize) -> $result>($function)(*a),
            [a, b] => transmute::<_, extern "C" fn(usize, usize) -> $result>($function)(*a, *b),
            [a, b, c] => {
                transmute::<_, extern "C" fn(usize, usize, usize) -> $result>($function)(*a, *b, *c)
            }
            [a, b, c, d] => transmute::<_, extern "C" fn(usize, usize, usize, usize) -> $result>(
                $function,
            )(*a, *b, *c, *d),
            [a, b, c, d, e] => transmute::<
                _,
                extern "C" fn(usize, usize, usize, usize, usize) -> $result,
            >($function)(*a, *b, *c, *d, *e),
            [a, b, c, d, e, f] => transmute::<
                _,
                extern "C" fn(usize, usize, usize, usize, usize, usize) -> $result,
            >($function)(*a, *b, *c, *d, *e, *f),
            _ => return None,
        }
    };
}

/// The most arguments a native call can have.
pub const MAX_ARGUMENTS: usize = 6;

/// Call the native function at `function` with integer-register arguments.
/// `None` when the call has more arguments than the VM can pass.
///
/// # Safety
///
/// `function` must be the address of a C function that takes that many
/// integer or pointer arguments and returns what `returns` says.
pub unsafe fn invoke(
    function: *const c_void,
    arguments: &[usize],
    returns: Returns,
) -> Option<Returned> {
    unsafe {
        Some(match returns {
            Returns::Integer => Returned::Integer(dispatch!(function, arguments, usize)),
            Returns::Float32 => Returned::Float(f64::from(dispatch!(function, arguments, f32))),
            Returns::Float64 => Returned::Float(dispatch!(function, arguments, f64)),
        })
    }
}
