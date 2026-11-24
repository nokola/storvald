//! Counts of one scenario, and the part of a scenario that is counted.

use turbodot::alloc_count;
use turbodot::gogo::layout::{LayoutTotals, layout_totals};

/// Name of `storvald_perf_measured` as Valgrind finds it in the program.
pub const MEASURED_SYMBOL: &str = "storvald_perf_measured";

/// Name of the count Valgrind adds: instructions the counted part ran.
pub const INSTRUCTIONS: &str = "instructions";

/// Names of the counts a scenario reads itself, in the order of
/// `Counts::values`.
pub const METRICS: [&str; 3] = ["allocations", "nodes_placed", "nodes_measured"];

/// What the counted part of one scenario cost.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Counts {
    /// Heap allocations.
    pub allocations: u64,
    /// Nodes layout gave a rectangle again.
    pub nodes_placed: u64,
    /// Nodes layout measured the long way.
    pub nodes_measured: u64,
}

impl Counts {
    /// The counts, in the order of `METRICS`.
    pub fn values(&self) -> [u64; 3] {
        [self.allocations, self.nodes_placed, self.nodes_measured]
    }

    /// One line a count: `<scenario> <metric> <value>`, each ended by a
    /// line feed. `expected_counts.txt` holds these lines.
    pub fn lines(&self, scenario: &str) -> String {
        METRICS.iter().zip(self.values()).map(|(metric, value)| format!("{scenario} {metric} {value}\n")).collect()
    }
}

/// Runs `work` as the counted part of a scenario.
///
/// Needed so a scenario's setup is not counted: a program is started and
/// a page opened before the one frame that is measured.
///
/// Returns what `work` cost.
pub fn counted(work: impl FnOnce()) -> Counts {
    let mut work: Option<_> = Some(work);
    let mut run_once = || (work.take().expect("the counted part runs once"))();
    let allocations_before: u64 = alloc_count::allocations();
    let layout_before: LayoutTotals = layout_totals();
    storvald_perf_measured(&mut run_once);
    let allocations: u64 = alloc_count::allocations() - allocations_before;
    let layout_after: LayoutTotals = layout_totals();
    Counts { allocations, nodes_placed: layout_after.placed - layout_before.placed, nodes_measured: layout_after.measured - layout_before.measured }
}

/// Calls `work`, under a name Valgrind can find.
///
/// Needed because Valgrind counts instructions from where this function
/// is entered to where it returns (`--toggle-collect`), and it finds the
/// function by its name in the program: the name is kept as written, and
/// the function is never folded into its caller.
#[unsafe(no_mangle)]
#[inline(never)]
pub fn storvald_perf_measured(work: &mut dyn FnMut()) {
    work();
}
