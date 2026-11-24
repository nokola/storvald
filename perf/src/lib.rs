//! Measured program of gogo and TurboDot: three pages that scenarios
//! open, move and close while what they cost is counted.
//!
//! - `scenarios`: what is measured, one function each.
//! - `counts`: counts of one scenario, and the part of it that is counted.
//! - `pages`: the three pages' code. Their pictures are in `ui/`.
//!
//! `storvald-perf` (`src/main.rs`) runs them. `tests/expected_counts.rs`
//! holds every exact count to `expected_counts.txt`.
//!
//! Only what is measured lives in this crate. What keeps the counts
//! (`history`, `functions`) is `storvald-perf`'s own: this crate holds
//! the copies of gogo's and TurboDot's generic code that the scenarios
//! run, and other code beside them changes how the compiler lays them
//! out, which moves an instruction count with no change to what is
//! measured (a slide frame counted 9 more when `functions` was added
//! here).

pub mod counts;
pub mod pages;
pub mod scenarios;

turbodot::built_pages!();

use turbodot::alloc_count::CountingAlloc;

#[global_allocator]
static ALLOC: CountingAlloc = CountingAlloc;

/// Program-wide state. Every page reaches it as `host.app`.
#[derive(Default)]
pub struct AppState {
    /// Count of value changes the controls page's first slider told its
    /// page. A scenario reads it: its counted step moved the slider.
    pub slider_change_count: u32,
}
