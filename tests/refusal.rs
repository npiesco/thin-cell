//! `try_new` hands its data back, allocating nothing, when the allocator refuses.

use std::{
    alloc::{GlobalAlloc, Layout, System},
    cell::Cell,
};

struct Refusing;

thread_local! {
    static REFUSING: Cell<bool> = const { Cell::new(false) };
    static MADE: Cell<usize> = const { Cell::new(0) };
}

// SAFETY: every call is forwarded to the system allocator, or refused with null.
unsafe impl GlobalAlloc for Refusing {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        if REFUSING.try_with(Cell::get).unwrap_or(false) {
            return std::ptr::null_mut();
        }
        let _ = MADE.try_with(|made| made.set(made.get() + 1));
        // SAFETY: the caller's layout is forwarded unchanged.
        unsafe { System.alloc(layout) }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        // SAFETY: the block came from the system allocator with this layout.
        unsafe { System.dealloc(ptr, layout) }
    }
}

#[global_allocator]
static ALLOCATOR: Refusing = Refusing;

#[test]
fn unsync_try_new_refused_hands_the_data_back() {
    REFUSING.with(|refusing| refusing.set(true));
    let refused = thin_cell::unsync::ThinCell::try_new([7u8; 32]);
    REFUSING.with(|refusing| refusing.set(false));
    assert_eq!(refused.err(), Some([7u8; 32]));
}

#[test]
fn sync_try_new_refused_hands_the_data_back() {
    REFUSING.with(|refusing| refusing.set(true));
    let refused = thin_cell::sync::ThinCell::try_new([7u8; 32]);
    REFUSING.with(|refusing| refusing.set(false));
    assert_eq!(refused.err(), Some([7u8; 32]));
}

#[test]
fn try_new_makes_exactly_one_block() {
    let before = MADE.with(Cell::get);
    let cell = thin_cell::unsync::ThinCell::try_new(5u64).ok().unwrap();
    assert_eq!(MADE.with(Cell::get) - before, 1);
    assert_eq!(*cell.borrow(), 5);
    drop(cell);
}
