//! A counting allocator: how many heap allocations a program made, and how
//! many bytes it holds.
//!
//! Needed because "a frame allocates nothing" is a goal, and a goal without
//! a number does not go red: a `Vec` built inside a frame shows here as a
//! count in the test that draws the frame, long before a user sees a
//! stutter.
//!
//! A program or a test file installs it once:
//!
//! ```text
//! #[global_allocator]
//! static ALLOC: turbodot::alloc_count::CountingAlloc = turbodot::alloc_count::CountingAlloc;
//!
//! let before: u64 = turbodot::alloc_count::allocations();
//! ui.frame(400.0, 300.0, 0.016);
//! assert_eq!(turbodot::alloc_count::allocations() - before, 0);
//! ```
//!
//! WARNING: the counts are for the whole process, every thread. A test that
//! compares two readings must be the only test of its file, because cargo
//! runs the tests of one file at the same time.
//!
//! In a program that does not install it, every count stays 0.

use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicI64, AtomicU64, Ordering};

/// The system allocator with a count on every call: one atomic add per
/// allocation and one per free.
pub struct CountingAlloc;

static ALLOCATIONS: AtomicU64 = AtomicU64::new(0);

/// The bytes held right now, per size class (`SIZE_CLASS_LIMITS_BYTES`).
static LIVE_BYTES: [AtomicI64; 4] = [AtomicI64::new(0), AtomicI64::new(0), AtomicI64::new(0), AtomicI64::new(0)];

/// The largest allocation of each size class: up to 4 KB, up to 64 KB, up
/// to 4 MB, and anything bigger.
///
/// Needed because a total that grows does not say what grows: the class
/// that grows names the kind of object (many small ones, or a few
/// pictures).
pub const SIZE_CLASS_LIMITS_BYTES: [usize; 4] = [4 * 1024, 64 * 1024, 4 * 1024 * 1024, usize::MAX];

/// The place of `size` in `SIZE_CLASS_LIMITS_BYTES`.
fn size_class(size: usize) -> usize {
    SIZE_CLASS_LIMITS_BYTES.iter().position(|&limit| size <= limit).unwrap_or(3)
}

/// The allocations since the process started. A growth of a block
/// (`realloc`) counts as one.
pub fn allocations() -> u64 {
    ALLOCATIONS.load(Ordering::Relaxed)
}

/// The bytes the program holds right now: the sizes it asked for, without
/// the allocator's own rounding.
pub fn live_bytes() -> i64 {
    LIVE_BYTES.iter().map(|class| class.load(Ordering::Relaxed)).sum()
}

/// The bytes held right now per size class, in the order of
/// `SIZE_CLASS_LIMITS_BYTES`.
pub fn live_bytes_by_class() -> [i64; 4] {
    [0, 1, 2, 3].map(|i| LIVE_BYTES[i].load(Ordering::Relaxed))
}

/// How many of the last big allocations and frees `take_big_events` hands
/// out before the oldest are lost.
const BIG_EVENT_COUNT: usize = 64;

/// The last allocations and frees over 4 MB, each as a signed size.
///
/// Needed because a size names a big object well enough to find it: a
/// decoded picture of 1080 x 1920 is 8.3 MB. Atomics only, because an
/// allocator can neither allocate nor log.
static BIG_EVENTS: [AtomicI64; BIG_EVENT_COUNT] = [const { AtomicI64::new(0) }; BIG_EVENT_COUNT];
static BIG_EVENTS_WRITTEN: AtomicU64 = AtomicU64::new(0);
static BIG_EVENTS_READ: AtomicU64 = AtomicU64::new(0);

/// The objects over 4 MB that are alive right now.
static BIG_LIVE_COUNT: AtomicI64 = AtomicI64::new(0);

/// Notes one allocation (a positive size) or one free (a negative size)
/// over 4 MB.
fn note_big(signed_size: i64) {
    let i: u64 = BIG_EVENTS_WRITTEN.fetch_add(1, Ordering::Relaxed);
    BIG_EVENTS[(i % BIG_EVENT_COUNT as u64) as usize].store(signed_size, Ordering::Relaxed);
    BIG_LIVE_COUNT.fetch_add(if signed_size > 0 { 1 } else { -1 }, Ordering::Relaxed);
}

/// Writes the allocations and frees over 4 MB since the last call into
/// `out`, oldest first: a positive size for an allocation, a negative one
/// for a free. Clears `out` first.
///
/// Returns how many were lost because more than 64 happened between two
/// calls.
pub fn take_big_events(out: &mut Vec<i64>) -> u64 {
    out.clear();
    let written: u64 = BIG_EVENTS_WRITTEN.load(Ordering::Relaxed);
    let mut read: u64 = BIG_EVENTS_READ.load(Ordering::Relaxed);
    let lost: u64 = (written - read).saturating_sub(BIG_EVENT_COUNT as u64);
    read += lost;
    while read < written {
        out.push(BIG_EVENTS[(read % BIG_EVENT_COUNT as u64) as usize].load(Ordering::Relaxed));
        read += 1;
    }
    BIG_EVENTS_READ.store(read, Ordering::Relaxed);
    lost
}

/// The objects over 4 MB that are alive right now.
pub fn big_live_count() -> i64 {
    BIG_LIVE_COUNT.load(Ordering::Relaxed)
}

unsafe impl GlobalAlloc for CountingAlloc {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        ALLOCATIONS.fetch_add(1, Ordering::Relaxed);
        let class: usize = size_class(layout.size());
        LIVE_BYTES[class].fetch_add(layout.size() as i64, Ordering::Relaxed);
        if class == 3 {
            note_big(layout.size() as i64);
        }
        unsafe { System.alloc(layout) }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        let class: usize = size_class(layout.size());
        LIVE_BYTES[class].fetch_sub(layout.size() as i64, Ordering::Relaxed);
        if class == 3 {
            note_big(-(layout.size() as i64));
        }
        unsafe { System.dealloc(ptr, layout) }
    }

    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        ALLOCATIONS.fetch_add(1, Ordering::Relaxed);
        let (old_class, new_class): (usize, usize) = (size_class(layout.size()), size_class(new_size));
        LIVE_BYTES[old_class].fetch_sub(layout.size() as i64, Ordering::Relaxed);
        LIVE_BYTES[new_class].fetch_add(new_size as i64, Ordering::Relaxed);
        if old_class == 3 {
            note_big(-(layout.size() as i64));
        }
        if new_class == 3 {
            note_big(new_size as i64);
        }
        unsafe { System.realloc(ptr, layout, new_size) }
    }
}
