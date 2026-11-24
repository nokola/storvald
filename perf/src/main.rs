//! `storvald-perf`: runs the measured scenarios and keeps their counts.
//!
//! - `list`: names of the scenarios.
//! - `run <scenario>`: runs one in this process, prints its counts.
//! - `counts`: runs each in a process of its own, prints every count.
//! - `write-expected`: writes those counts to `expected_counts.txt`.
//! - `record`: runs each under Valgrind, adds a run to `history.csv`,
//!   writes the page made from it and, a scenario a file, its
//!   instructions by function. Linux, with `valgrind` installed.
//! - `check-latest`: fails when an instruction count of the last run
//!   in `history.csv` rose and neither compiler nor machine changed
//!   with it. A count that fell is printed and passes; the exact counts
//!   are held by the test of `expected_counts.txt`.
//! - `functions-moved`: prints the functions whose instructions differ
//!   between two of `record`'s lists of one scenario.

use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode, Output};

use storvald_perf::counts::{INSTRUCTIONS, MEASURED_SYMBOL};
use storvald_perf::functions::{self, FunctionCount, FunctionMove};
use storvald_perf::history::{self, Move, Row};
use storvald_perf::scenarios::{self, SCENARIOS, Scenario};

const USAGE: &str = "usage: storvald-perf list | run <scenario> | counts | write-expected | record --history <file> --page <file> --functions <directory> --date <utc> --commit <hash> --rustc <version> --machine <name> | check-latest --history <file> | functions-moved --before <file> --after <file>";

/// File of the exact counts, beside this crate's `Cargo.toml`.
const EXPECTED_COUNTS_FILE: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/expected_counts.txt");

/// What `expected_counts.txt` says above its counts.
const EXPECTED_COUNTS_HEADER: &str = "# Exact counts of the measured program, one a line: <scenario> <metric> <value>.\n# `cargo test -p storvald-perf` fails when one differs.\n# A change that moves a count on purpose writes this file again:\n#     cargo run -p storvald-perf -- write-expected\n";

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let done: Result<(), String> = match args.first().map(String::as_str) {
        Some("list") => {
            SCENARIOS.iter().for_each(|scenario| println!("{}", scenario.name));
            Ok(())
        }
        Some("run") => run(args.get(1)),
        Some("counts") => counts().map(|lines| print!("{lines}")),
        Some("write-expected") => write_expected(),
        Some("record") => record(&args[1..]),
        Some("check-latest") => check_latest(&args[1..]),
        Some("functions-moved") => functions_moved(&args[1..]),
        Some(other) => Err(format!("no command `{other}`\n{USAGE}")),
        None => Err(USAGE.to_string()),
    };
    match done {
        Ok(()) => ExitCode::SUCCESS,
        Err(why) => {
            eprintln!("{why}");
            ExitCode::FAILURE
        }
    }
}

/// Runs one scenario in this process and prints its counts.
fn run(name: Option<&String>) -> Result<(), String> {
    let name: &String = name.ok_or(USAGE)?;
    let scenario: &Scenario = scenarios::find(name).ok_or_else(|| format!("no scenario `{name}`; `storvald-perf list` names them"))?;
    print!("{}", (scenario.run)().lines(scenario.name));
    Ok(())
}

/// What a finished process printed, or why it counts as failed.
fn printed(output: std::io::Result<Output>, what: &str) -> Result<String, String> {
    let output: Output = output.map_err(|error| format!("{what} did not start: {error}"))?;
    if !output.status.success() {
        return Err(format!("{what} failed ({}):\n{}", output.status, String::from_utf8_lossy(&output.stderr)));
    }
    Ok(String::from_utf8_lossy(&output.stdout).into_owned())
}

/// Runs every scenario, each in a process of its own.
///
/// Returns their count lines, in the scenarios' order.
fn counts() -> Result<String, String> {
    let program: PathBuf = std::env::current_exe().map_err(|error| format!("this program's own path: {error}"))?;
    let mut lines: String = String::new();
    for scenario in SCENARIOS {
        lines.push_str(&printed(Command::new(&program).args(["run", scenario.name]).output(), scenario.name)?);
    }
    Ok(lines)
}

fn write_expected() -> Result<(), String> {
    let text: String = format!("{EXPECTED_COUNTS_HEADER}{}", counts()?);
    std::fs::write(EXPECTED_COUNTS_FILE, text).map_err(|error| format!("{EXPECTED_COUNTS_FILE}: {error}"))?;
    println!("written {EXPECTED_COUNTS_FILE}");
    Ok(())
}

/// Value that follows `--<name>` in `args`.
fn flag<'a>(args: &'a [String], name: &str) -> Result<&'a str, String> {
    let position: Option<usize> = args.iter().position(|arg| arg.strip_prefix("--") == Some(name));
    position.and_then(|index| args.get(index + 1)).map(String::as_str).ok_or_else(|| format!("`--{name} <value>` is missing\n{USAGE}"))
}

/// Instructions Valgrind counted, from the file it wrote.
fn instructions_in(callgrind_file: &Path) -> Result<u64, String> {
    let text: String = std::fs::read_to_string(callgrind_file).map_err(|error| format!("{}: {error}", callgrind_file.display()))?;
    let total: Option<u64> = text.lines().rev().find_map(|line| line.strip_prefix("totals:").or_else(|| line.strip_prefix("summary:"))).and_then(|numbers| numbers.split_whitespace().next()).and_then(|first| first.parse().ok());
    total.ok_or_else(|| format!("{} has no `totals:` line", callgrind_file.display()))
}

/// A scenario's instructions by function as a file's text, from the
/// file Valgrind wrote.
///
/// Needed apart from the total because a list that cannot be made must
/// not lose the run its total: the reason then stands in the list's
/// file and in the job's output.
fn function_list(scenario: &Scenario, commit: &str, callgrind_file: &Path, instructions: u64) -> String {
    let counts: Result<Vec<FunctionCount>, String> = std::fs::read_to_string(callgrind_file).map_err(|error| format!("{}: {error}", callgrind_file.display())).and_then(|callgrind| functions::counts_from_callgrind(&callgrind));
    match counts {
        Ok(counts) => {
            if functions::total(&counts) != instructions {
                eprintln!("{}: its functions sum to {} instructions, Valgrind's total is {instructions}", scenario.name, functions::total(&counts));
            }
            functions::list(scenario.name, commit, &counts)
        }
        Err(why) => {
            eprintln!("{}: no list of its functions: {why}", scenario.name);
            functions::list_without_counts(scenario.name, commit, &why)
        }
    }
}

/// Runs one scenario under Valgrind, which counts the instructions of
/// its counted part alone.
///
/// Returns the scenario's rows (its own counts and the instructions)
/// and the text of its list of instructions by function.
fn rows_of(scenario: &Scenario, program: &Path, commit: &str, row: &dyn Fn(&str, &str, u64) -> Row) -> Result<(Vec<Row>, String), String> {
    let callgrind_file: PathBuf = std::env::temp_dir().join(format!("storvald-perf-{}.callgrind", scenario.name));
    let valgrind = Command::new("valgrind")
        .arg("--tool=callgrind")
        .arg(format!("--callgrind-out-file={}", callgrind_file.display()))
        .arg(format!("--toggle-collect={MEASURED_SYMBOL}"))
        .arg("--collect-atstart=no")
        .arg(program)
        .args(["run", scenario.name])
        .output();
    let lines: String = printed(valgrind, &format!("valgrind on {}", scenario.name))?;
    let mut rows: Vec<Row> = Vec::new();
    for line in lines.lines() {
        let words: Vec<&str> = line.split_whitespace().collect();
        match (words.as_slice(), words.get(2).and_then(|value| value.parse().ok())) {
            ([name, metric, _], Some(value)) if *name == scenario.name => rows.push(row(name, metric, value)),
            _ => return Err(format!("{} printed a line that is no count: {line}", scenario.name)),
        }
    }
    let instructions: u64 = instructions_in(&callgrind_file)?;
    if instructions == 0 {
        return Err(format!("valgrind counted no instruction in {}: it did not find `{MEASURED_SYMBOL}` in the program", scenario.name));
    }
    rows.push(row(scenario.name, INSTRUCTIONS, instructions));
    Ok((rows, function_list(scenario, commit, &callgrind_file, instructions)))
}

fn record(args: &[String]) -> Result<(), String> {
    let (history_file, page_file): (&str, &str) = (flag(args, "history")?, flag(args, "page")?);
    let functions_directory: &Path = Path::new(flag(args, "functions")?);
    let (date, commit, rustc, machine): (&str, &str, &str, &str) = (flag(args, "date")?, flag(args, "commit")?, flag(args, "rustc")?, flag(args, "machine")?);
    let row = |scenario: &str, metric: &str, value: u64| -> Row { Row { date: date.to_string(), commit: commit.to_string(), rustc: rustc.to_string(), machine: machine.to_string(), scenario: scenario.to_string(), metric: metric.to_string(), value } };
    let mut rows: Vec<Row> = match std::fs::read_to_string(history_file) {
        Ok(csv) => history::rows_from_csv(&csv)?,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Vec::new(),
        Err(error) => return Err(format!("{history_file}: {error}")),
    };
    let program: PathBuf = std::env::current_exe().map_err(|error| format!("this program's own path: {error}"))?;
    std::fs::create_dir_all(functions_directory).map_err(|error| format!("{}: {error}", functions_directory.display()))?;
    for scenario in SCENARIOS {
        let (of_scenario, function_list): (Vec<Row>, String) = rows_of(scenario, &program, commit, &row)?;
        of_scenario.iter().for_each(|row| println!("{} {} {}", row.scenario, row.metric, row.value));
        rows.extend(of_scenario);
        let list_file: PathBuf = functions_directory.join(format!("{}.txt", scenario.name));
        std::fs::write(&list_file, function_list).map_err(|error| format!("{}: {error}", list_file.display()))?;
    }
    std::fs::write(history_file, history::csv_from_rows(&rows)).map_err(|error| format!("{history_file}: {error}"))?;
    std::fs::write(page_file, history::page(&rows, SCENARIOS, env!("CARGO_PKG_REPOSITORY"))).map_err(|error| format!("{page_file}: {error}"))?;
    Ok(())
}

fn check_latest(args: &[String]) -> Result<(), String> {
    let history_file: &str = flag(args, "history")?;
    let csv: String = std::fs::read_to_string(history_file).map_err(|error| format!("{history_file}: {error}"))?;
    let moved: Vec<Move> = history::latest_moves(&history::rows_from_csv(&csv)?);
    for one in &moved {
        println!("{} {} {}: {} to {}{}", one.scenario, one.metric, if one.is_rise() { "rose" } else { "fell" }, one.before, one.after, if one.also_changed.is_empty() { String::new() } else { format!(" ({})", one.also_changed) });
    }
    // An exact count that moved is in `expected_counts.txt` already: the
    // commit that moved it wrote the file again, or the test failed.
    let risen_by_the_code: usize = moved.iter().filter(|one| one.metric == INSTRUCTIONS && one.is_rise() && one.also_changed.is_empty()).count();
    if risen_by_the_code > 0 {
        return Err(format!("{risen_by_the_code} instruction counts rose against the run before, on the same compiler and machine"));
    }
    println!("no instruction count rose by the code alone");
    Ok(())
}

/// Prints the functions whose instructions differ between two lists of
/// one scenario, largest difference first.
///
/// Needed when a scenario's total rose: the two lists are the files
/// `record` wrote for the run before and for the run that rose.
fn functions_moved(args: &[String]) -> Result<(), String> {
    let counts_in = |name: &str| -> Result<Vec<FunctionCount>, String> {
        let file: &str = flag(args, name)?;
        let text: String = std::fs::read_to_string(file).map_err(|error| format!("{file}: {error}"))?;
        functions::counts_from_list(&text).map_err(|why| format!("{file}: {why}"))
    };
    let (before, after): (Vec<FunctionCount>, Vec<FunctionCount>) = (counts_in("before")?, counts_in("after")?);
    let moved: Vec<FunctionMove> = functions::moves(&before, &after);
    for one in &moved {
        println!("{:+} {}: {} to {}", one.difference(), one.function, one.before, one.after);
    }
    println!("{:+} in all: {} to {}, {} functions differ", functions::total(&after) as i64 - functions::total(&before) as i64, functions::total(&before), functions::total(&after), moved.len());
    Ok(())
}
