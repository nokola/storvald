//! Rows kept from run to run (`history.csv`), and the page made from
//! them (`README.md` beside it). Both live on the `perf-data` branch.

use storvald_perf::counts::INSTRUCTIONS;
use storvald_perf::scenarios::Scenario;

/// First line of `history.csv`.
pub const HEADER: &str = "date,commit,rustc,machine,scenario,metric,value";

/// How far an instruction count may differ from the run before and not
/// be listed as moved, in percent. Exact counts have no such room.
///
/// Two runs of the same code on GitHub (2026-11-24) differed by 0.013 %
/// at most, and nine scenarios of sixteen not at all.
pub const INSTRUCTIONS_SPREAD_PERCENT: f64 = 0.2;

/// At most this many moved numbers on the page, newest first.
const MOVES_ON_PAGE: usize = 60;

/// One number of one run.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Row {
    /// When the run started, UTC: `2026-11-24T05:17:02Z`.
    pub date: String,
    /// Commit that was measured, whole.
    pub commit: String,
    /// Compiler's version: `1.91.0`.
    pub rustc: String,
    /// Machine that ran it, with its system's version.
    pub machine: String,
    pub scenario: String,
    pub metric: String,
    pub value: u64,
}

/// Reads `history.csv`.
///
/// Returns its rows, or which line is no row.
pub fn rows_from_csv(csv: &str) -> Result<Vec<Row>, String> {
    let mut rows: Vec<Row> = Vec::new();
    for (index, line) in csv.lines().enumerate() {
        if line.is_empty() || (index == 0 && line == HEADER) {
            continue;
        }
        let cells: Vec<&str> = line.split(',').collect();
        let value: Option<u64> = cells.get(6).and_then(|cell| cell.parse().ok());
        match (cells.len(), value) {
            (7, Some(value)) => rows.push(Row { date: cells[0].to_string(), commit: cells[1].to_string(), rustc: cells[2].to_string(), machine: cells[3].to_string(), scenario: cells[4].to_string(), metric: cells[5].to_string(), value }),
            _ => return Err(format!("history line {} is not `{HEADER}`: {line}", index + 1)),
        }
    }
    Ok(rows)
}

/// `history.csv` of `rows`: header, then a line a row, each ended by a
/// line feed.
pub fn csv_from_rows(rows: &[Row]) -> String {
    let mut csv: String = format!("{HEADER}\n");
    for row in rows {
        csv.push_str(&format!("{},{},{},{},{},{},{}\n", row.date, row.commit, row.rustc, row.machine, row.scenario, row.metric, row.value));
    }
    csv
}

/// Number of one run that differs from the run before.
#[derive(Clone, Debug, PartialEq)]
pub struct Move {
    pub date: String,
    pub commit: String,
    pub scenario: String,
    pub metric: String,
    pub before: u64,
    pub after: u64,
    /// What else differs between the two runs: compiler, machine. Empty:
    /// the code alone can have moved the number.
    pub also_changed: String,
}

impl Move {
    /// Whether the number went up: more instructions, allocations or
    /// nodes than the run before.
    ///
    /// Needed because the job that counts fails on a rise alone. A
    /// change that makes a scenario cheaper moves its numbers too, and
    /// a job red for that teaches its reader to pass over red.
    pub fn is_rise(&self) -> bool {
        self.after > self.before
    }
}

/// Rows of each run, oldest run first. A run is the rows of one date
/// and commit.
fn runs(rows: &[Row]) -> Vec<&[Row]> {
    rows.chunk_by(|a, b| a.date == b.date && a.commit == b.commit).collect()
}

/// Numbers of `run` that differ from `before`: an instruction count by
/// more than `INSTRUCTIONS_SPREAD_PERCENT`, an exact count at all.
fn moves_between(before: &[Row], run: &[Row]) -> Vec<Move> {
    let (old, new): (&Row, &Row) = (&before[0], &run[0]);
    let mut also_changed: Vec<String> = Vec::new();
    if old.rustc != new.rustc {
        also_changed.push(format!("rustc {} to {}", old.rustc, new.rustc));
    }
    if old.machine != new.machine {
        also_changed.push(format!("machine {} to {}", old.machine, new.machine));
    }
    let also_changed: String = also_changed.join(", ");
    let mut moves: Vec<Move> = Vec::new();
    for row in run {
        let Some(was) = before.iter().find(|old| old.scenario == row.scenario && old.metric == row.metric) else { continue };
        let is_moved: bool = if row.metric == INSTRUCTIONS { change_percent(was.value, row.value).is_none_or(|percent| percent.abs() > INSTRUCTIONS_SPREAD_PERCENT) } else { row.value != was.value };
        if is_moved && row.value != was.value {
            moves.push(Move { date: row.date.clone(), commit: row.commit.clone(), scenario: row.scenario.clone(), metric: row.metric.clone(), before: was.value, after: row.value, also_changed: also_changed.clone() });
        }
    }
    moves
}

/// Every moved number in `rows`, oldest first.
pub fn moves(rows: &[Row]) -> Vec<Move> {
    let runs: Vec<&[Row]> = runs(rows);
    runs.windows(2).flat_map(|pair| moves_between(pair[0], pair[1])).collect()
}

/// Moved numbers of the last run against the run before it.
pub fn latest_moves(rows: &[Row]) -> Vec<Move> {
    let runs: Vec<&[Row]> = runs(rows);
    match runs.len() {
        0 | 1 => Vec::new(),
        count => moves_between(runs[count - 2], runs[count - 1]),
    }
}

/// By how much `after` differs from `before`, in percent. `None`: there
/// was nothing before, so no percent says it.
fn change_percent(before: u64, after: u64) -> Option<f64> {
    (before != 0).then(|| (after as f64 - before as f64) / before as f64 * 100.0)
}

/// `1234567` as `1,234,567`.
fn grouped(value: u64) -> String {
    let digits: String = value.to_string();
    let mut text: String = String::new();
    for (index, digit) in digits.chars().enumerate() {
        if index > 0 && (digits.len() - index) % 3 == 0 {
            text.push(',');
        }
        text.push(digit);
    }
    text
}

/// A change as the page writes it: `+1.8 %`, `-0.4 %`, `new` when there
/// was nothing before.
fn change_text(before: u64, after: u64) -> String {
    match change_percent(before, after) {
        Some(percent) => format!("{percent:+.1} %"),
        None => "new".to_string(),
    }
}

/// Commit as a link to it, by its first seven letters.
fn commit_link(repository: &str, commit: &str) -> String {
    format!("[{}]({repository}/commit/{commit})", &commit[..commit.len().min(7)])
}

/// The page people read: numbers of the last run with how far each
/// instruction count is from the run before, then the numbers that
/// moved, newest first.
///
/// Returns the page as Markdown, lines ended by a line feed.
pub fn page(rows: &[Row], scenarios: &[Scenario], repository: &str) -> String {
    let mut page: String = String::from("# Storvald perf counts\n\n");
    page.push_str("What gogo and TurboDot cost in the measured program (`perf/` on `main`), counted on one of GitHub's Linux machines at every push to `main` and once a day.\n\n");
    page.push_str("- **Instructions**: counted by Valgrind while the step runs. Same code and same compiler give the same count within a small spread, on any machine.\n");
    page.push_str("- **Allocations, nodes placed, nodes measured**: exact. `perf/expected_counts.txt` on `main` holds them, and a test fails when one differs.\n");
    page.push_str("- Every run is rows in [history.csv](history.csv): date, commit, compiler, machine, scenario, metric, value.\n");
    page.push_str("- [functions/](functions) holds the last run's instructions by function, a scenario a file. The file's history has the runs before. After a rise, `storvald-perf functions-moved --before <file> --after <file>` names the functions that grew.\n\n");
    let runs: Vec<&[Row]> = runs(rows);
    let Some(latest) = runs.last() else {
        page.push_str("No run yet.\n");
        return page;
    };
    let before: Option<&[Row]> = runs.len().checked_sub(2).map(|index| runs[index]);
    let value_in = |run: &[Row], scenario: &str, metric: &str| -> Option<u64> { run.iter().find(|row| row.scenario == scenario && row.metric == metric).map(|row| row.value) };
    let cell = |scenario: &str, metric: &str| -> String { value_in(latest, scenario, metric).map(grouped).unwrap_or_default() };

    page.push_str(&format!("## Last run\n\n{}, commit {}, rustc {}, machine `{}`. {} runs so far.\n\n", latest[0].date, commit_link(repository, &latest[0].commit), latest[0].rustc, latest[0].machine, runs.len()));
    page.push_str("| Scenario | What is counted | Instructions | Against run before | Allocations | Nodes placed | Nodes measured |\n|---|---|---:|---:|---:|---:|---:|\n");
    for scenario in scenarios {
        let against_before: String = match (before.and_then(|run| value_in(run, scenario.name, INSTRUCTIONS)), value_in(latest, scenario.name, INSTRUCTIONS)) {
            (Some(was), Some(now)) => change_text(was, now),
            _ => String::new(),
        };
        page.push_str(&format!("| `{}` | {} | {} | {} | {} | {} | {} |\n", scenario.name, scenario.what, cell(scenario.name, INSTRUCTIONS), against_before, cell(scenario.name, "allocations"), cell(scenario.name, "nodes_placed"), cell(scenario.name, "nodes_measured")));
    }

    page.push_str(&format!("\n## Numbers that moved\n\nA number that differs from the run before: an instruction count by more than {INSTRUCTIONS_SPREAD_PERCENT} %, an exact count at all. Newest first, at most {MOVES_ON_PAGE}. \"Also changed\" names what else differs between the two runs; empty means the code alone can have moved the number.\n\n"));
    let moves: Vec<Move> = moves(rows);
    if moves.is_empty() {
        page.push_str("None so far.\n");
        return page;
    }
    page.push_str("| Run | Commit | Scenario | Metric | Before | After | Change | Also changed |\n|---|---|---|---|---:|---:|---:|---|\n");
    for moved in moves.iter().rev().take(MOVES_ON_PAGE) {
        page.push_str(&format!("| {} | {} | `{}` | {} | {} | {} | {} | {} |\n", moved.date, commit_link(repository, &moved.commit), moved.scenario, moved.metric, grouped(moved.before), grouped(moved.after), change_text(moved.before, moved.after), moved.also_changed));
    }
    page
}

#[cfg(test)]
mod tests {
    use super::*;
    use storvald_perf::counts::Counts;

    #[test]
    fn a_number_that_fell_is_a_move_and_not_a_rise() {
        let rows: Vec<Row> = vec![
            row("2026-11-24T05:00:00Z", "aaaaaaaaaaaa", "1.91.0", "scroll_step", INSTRUCTIONS, 17_339_924),
            row("2026-11-24T05:00:00Z", "aaaaaaaaaaaa", "1.91.0", "text_change", INSTRUCTIONS, 163_128),
            row("2026-11-25T05:00:00Z", "bbbbbbbbbbbb", "1.91.0", "scroll_step", INSTRUCTIONS, 13_303_823),
            row("2026-11-25T05:00:00Z", "bbbbbbbbbbbb", "1.91.0", "text_change", INSTRUCTIONS, 170_000),
        ];
        let moved: Vec<Move> = latest_moves(&rows);
        let risen: Vec<&str> = moved.iter().filter(|one| one.is_rise()).map(|one| one.scenario.as_str()).collect();
        assert_eq!(moved.len(), 2, "both numbers moved: {moved:?}");
        assert_eq!(risen, ["text_change"], "one of them rose");
    }

    fn row(date: &str, commit: &str, rustc: &str, scenario: &str, metric: &str, value: u64) -> Row {
        Row { date: date.to_string(), commit: commit.to_string(), rustc: rustc.to_string(), machine: "github-ubuntu24".to_string(), scenario: scenario.to_string(), metric: metric.to_string(), value }
    }

    /// Three runs of one scenario: second moves an exact count and stays
    /// inside the spread of instructions; third, on a new compiler, moves
    /// instructions past it.
    fn three_runs() -> Vec<Row> {
        vec![
            row("2026-11-24T05:00:00Z", "aaaaaaaaaaaa", "1.91.0", "still_frame", INSTRUCTIONS, 1_000_000),
            row("2026-11-24T05:00:00Z", "aaaaaaaaaaaa", "1.91.0", "still_frame", "allocations", 0),
            row("2026-11-25T05:00:00Z", "bbbbbbbbbbbb", "1.91.0", "still_frame", INSTRUCTIONS, 1_001_000),
            row("2026-11-25T05:00:00Z", "bbbbbbbbbbbb", "1.91.0", "still_frame", "allocations", 2),
            row("2026-11-26T05:00:00Z", "bbbbbbbbbbbb", "1.92.0", "still_frame", INSTRUCTIONS, 1_100_000),
            row("2026-11-26T05:00:00Z", "bbbbbbbbbbbb", "1.92.0", "still_frame", "allocations", 2),
        ]
    }

    #[test]
    fn rows_come_back_from_their_csv() {
        let rows: Vec<Row> = three_runs();
        let csv: String = csv_from_rows(&rows);
        assert!(csv.starts_with("date,commit,rustc,machine,scenario,metric,value\n2026-11-24T05:00:00Z,aaaaaaaaaaaa,1.91.0,github-ubuntu24,still_frame,instructions,1000000\n"), "{csv}");
        assert_eq!(rows_from_csv(&csv), Ok(rows));
        assert_eq!(rows_from_csv(""), Ok(Vec::new()));
        let refused: String = rows_from_csv("date,commit,rustc,machine,scenario,metric,value\nnot a row\n").expect_err("a line that is no row");
        assert!(refused.contains("line 2") && refused.contains("not a row"), "{refused}");
    }

    #[test]
    fn a_number_is_listed_as_moved_past_its_spread() {
        let rows: Vec<Row> = three_runs();
        let all: Vec<Move> = moves(&rows);
        let said: Vec<(&str, &str, u64, u64, &str)> = all.iter().map(|moved| (moved.commit.as_str(), moved.metric.as_str(), moved.before, moved.after, moved.also_changed.as_str())).collect();
        assert_eq!(said, vec![("bbbbbbbbbbbb", "allocations", 0, 2, ""), ("bbbbbbbbbbbb", INSTRUCTIONS, 1_001_000, 1_100_000, "rustc 1.91.0 to 1.92.0")], "0.1 % of instructions is inside the spread; an exact count has none");
        assert_eq!(latest_moves(&rows), all[1..].to_vec());
        assert_eq!(latest_moves(&rows[..2]), Vec::new(), "a first run has no run before it");
    }

    #[test]
    fn the_page_shows_the_last_run_and_what_moved() {
        let scenarios: [Scenario; 1] = [Scenario { name: "still_frame", what: "frame while nothing changes", run: Counts::default }];
        let text: String = page(&three_runs(), &scenarios, "https://example.org/repo");
        assert!(text.contains("2026-11-26T05:00:00Z, commit [bbbbbbb](https://example.org/repo/commit/bbbbbbbbbbbb), rustc 1.92.0, machine `github-ubuntu24`. 3 runs so far."), "{text}");
        assert!(text.contains("| `still_frame` | frame while nothing changes | 1,100,000 | +9.9 % | 2 |  |  |\n"), "{text}");
        let newest: usize = text.find("| 2026-11-26T05:00:00Z | [bbbbbbb](https://example.org/repo/commit/bbbbbbbbbbbb) | `still_frame` | instructions | 1,001,000 | 1,100,000 | +9.9 % | rustc 1.91.0 to 1.92.0 |").expect("the newest move");
        let older: usize = text.find("| `still_frame` | allocations | 0 | 2 | new |  |").expect("the older move");
        assert!(newest < older, "newest first");
        assert!(page(&[], &scenarios, "https://example.org/repo").contains("No run yet."));
    }
}
