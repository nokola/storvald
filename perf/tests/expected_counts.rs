//! Holds every exact count of the measured program to
//! `expected_counts.txt`.

use std::process::{Command, Output};

/// Each scenario's allocations and layout counts are the ones
/// `expected_counts.txt` holds.
///
/// Needed because a change that makes a page allocate more, or lay out
/// more nodes, breaks no other test: user sees a slower page on a slow
/// phone, and nobody knows which change did it. A count that moves on
/// purpose moves in the same commit as the file, so the file's history
/// names the change behind every number.
#[test]
fn every_count_is_the_expected_one() {
    let output: Output = Command::new(env!("CARGO_BIN_EXE_storvald-perf")).arg("counts").output().expect("storvald-perf starts");
    assert!(output.status.success(), "storvald-perf counts failed:\n{}", String::from_utf8_lossy(&output.stderr));
    let counted: String = String::from_utf8(output.stdout).expect("counts are plain text");
    let counted: Vec<&str> = counted.lines().collect();
    let expected: &str = include_str!("../expected_counts.txt");
    let expected: Vec<&str> = expected.lines().filter(|line| !line.starts_with('#')).collect();
    assert!(counted.len() >= 48, "every scenario was counted: {} lines", counted.len());

    let mut differences: Vec<String> = Vec::new();
    for line in &counted {
        let (count_name, value): (&str, &str) = line.rsplit_once(' ').expect("a count line ends in its value");
        match expected.iter().find_map(|line| line.strip_prefix(count_name).and_then(|rest| rest.strip_prefix(' '))) {
            Some(expected_value) if expected_value == value => {}
            Some(expected_value) => differences.push(format!("{count_name}: expected {expected_value}, counted {value}")),
            None => differences.push(format!("{count_name}: counted {value}, not in the file")),
        }
    }
    for line in &expected {
        if !counted.contains(line) && !counted.iter().any(|counted| counted.rsplit_once(' ').map(|(count_name, _)| count_name) == line.rsplit_once(' ').map(|(count_name, _)| count_name)) {
            differences.push(format!("{line}: in the file, not counted"));
        }
    }
    assert!(differences.is_empty(), "counts differ from perf/expected_counts.txt:\n  {}\nIf the change is meant to move them, write the file again and commit it with the change:\n  cargo run -p storvald-perf -- write-expected", differences.join("\n  "));
}
