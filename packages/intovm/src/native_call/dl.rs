use anyhow::{Result, anyhow, bail};
use std::ffi::{CStr, CString, c_char, c_int, c_void};
use std::path::Path;

const RTLD_NOW: c_int = 2;
const RTLD_GLOBAL: c_int = 0x100;

unsafe extern "C" {
    fn dlopen(filename: *const c_char, flags: c_int) -> *mut c_void;
    fn dlsym(handle: *mut c_void, symbol: *const c_char) -> *mut c_void;
    fn dlerror() -> *const c_char;
}

/// A shared library the program loaded while it runs. It stays loaded until
/// the process ends.
#[derive(Debug)]
pub struct Library {
    handle: *mut c_void,
}

impl Library {
    pub fn open(path: &Path) -> Result<Self> {
        let name = CString::new(path.to_string_lossy().as_bytes())?;
        let handle = unsafe { dlopen(name.as_ptr(), RTLD_NOW | RTLD_GLOBAL) };
        if handle.is_null() {
            let reason = unsafe { dlerror() };
            let reason = if reason.is_null() {
                "unknown error".to_owned()
            } else {
                unsafe { CStr::from_ptr(reason) }
                    .to_string_lossy()
                    .into_owned()
            };
            bail!(
                "Could not load the native library {}: {reason}",
                path.display()
            );
        }
        Ok(Self { handle })
    }

    /// The address of the function `symbol`, if the library defines it.
    pub fn symbol(&self, symbol: &str) -> Result<Option<*const c_void>> {
        let name = CString::new(symbol).map_err(|_| anyhow!("`{symbol}` is not a symbol"))?;
        let address = unsafe { dlsym(self.handle, name.as_ptr()) };
        Ok((!address.is_null()).then_some(address as *const c_void))
    }
}
