//! `malloc`, `calloc` and `free` over the heap of Rust, for a target with no C
//! library. Each block keeps its size just before the address it hands out.

use std::alloc::{Layout, alloc, alloc_zeroed, dealloc};
use std::ffi::c_void;

const HEADER: usize = 16;

fn layout(size: usize) -> Option<Layout> {
    Layout::from_size_align(size.checked_add(HEADER)?, HEADER).ok()
}

pub(super) unsafe fn malloc(size: usize) -> *mut c_void {
    let Some(layout) = layout(size) else {
        return std::ptr::null_mut();
    };
    let block = unsafe { alloc(layout) };
    unsafe { block_start(block, size) }
}

pub(super) unsafe fn calloc(count: usize, size: usize) -> *mut c_void {
    let Some(size) = count.checked_mul(size) else {
        return std::ptr::null_mut();
    };
    let Some(layout) = layout(size) else {
        return std::ptr::null_mut();
    };
    let block = unsafe { alloc_zeroed(layout) };
    unsafe { block_start(block, size) }
}

pub(super) unsafe fn free(pointer: *mut c_void) {
    if pointer.is_null() {
        return;
    }
    let block = unsafe { (pointer as *mut u8).sub(HEADER) };
    let size = unsafe { *(block as *const usize) };
    if let Some(layout) = layout(size) {
        unsafe { dealloc(block, layout) };
    }
}

unsafe fn block_start(block: *mut u8, size: usize) -> *mut c_void {
    if block.is_null() {
        return std::ptr::null_mut();
    }
    unsafe {
        *(block as *mut usize) = size;
        block.add(HEADER) as *mut c_void
    }
}
