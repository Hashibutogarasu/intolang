/// The prefix of the native functions that only the Into VM implements, such
/// as the ones `DummyProcess` binds to.
const VM_ONLY_PREFIX: &str = "dummy_process_";

/// Whether `symbol` names a function that exists in the Into VM only, so that
/// compiled code cannot call it.
pub fn is_vm_only(symbol: &str) -> bool {
    symbol.starts_with(VM_ONLY_PREFIX)
}
