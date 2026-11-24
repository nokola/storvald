//! Measured program of gogo and TurboDot: three pages that scenarios
//! open, move and close while what they cost is counted.
//!
//! - `scenarios`: what is measured, one function each.
//! - `counts`: counts of one scenario, and the part of it that is counted.
//! - `pages`: the three pages' code. Their pictures are in `ui/`.
//! - `history`: rows kept from run to run, and the page made from them.
//! - `functions`: a scenario's instructions by function, and the
//!   functions that differ between two such lists.
//!
//! `storvald-perf` (`src/main.rs`) runs them. `tests/expected_counts.rs`
//! holds every exact count to `expected_counts.txt`.

pub mod counts;
pub mod functions;
pub mod history;
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
