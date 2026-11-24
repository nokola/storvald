//! Instructions of one scenario by function, read from the file Valgrind
//! writes, and the functions that differ between two such lists.
//!
//! Needed because a scenario's total says that a change made it run
//! more instructions and not where: with a list from before and one
//! from after, the function that grew is named.

use std::collections::HashMap;

/// Instructions one function's own code ran in a scenario's counted
/// part. What it calls is counted at the called function.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FunctionCount {
    /// Function's name as Valgrind prints it, without the hash a build
    /// adds to it. Copies of one generic function share a name.
    pub function: String,
    pub instructions: u64,
}

/// Function whose count differs between two lists.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FunctionMove {
    pub function: String,
    /// Zero: the list before has no such function.
    pub before: u64,
    /// Zero: the list after has no such function.
    pub after: u64,
}

impl FunctionMove {
    /// How many instructions more the function ran after. Below zero:
    /// fewer.
    pub fn difference(&self) -> i64 {
        self.after as i64 - self.before as i64
    }
}

/// `symbol` without the hash a build adds to a function's name.
///
/// Needed because that hash changes whenever anything in the function's
/// crate does: with it, every name of a list would differ from the list
/// before, and no two lists could be compared.
///
/// Two spellings end a name: `::h` and sixteen hex digits when Valgrind
/// wrote the name out, `17h`, the digits and `E` when it did not.
fn name_without_hash(symbol: &str) -> &str {
    const HEX_DIGITS_IN_A_HASH: usize = 16;
    let is_hash = |digits: &str| -> bool { digits.len() == HEX_DIGITS_IN_A_HASH && digits.bytes().all(|digit| digit.is_ascii_hexdigit()) };
    if let Some((name, digits)) = symbol.rsplit_once("::h") {
        if is_hash(digits) {
            return name;
        }
    }
    if let Some((name, digits)) = symbol.strip_suffix('E').and_then(|rest| rest.rsplit_once("17h")) {
        if is_hash(digits) {
            return name;
        }
    }
    symbol
}

/// Name a `fn=` or `cfn=` line says, from what follows its `=`: a name,
/// `(7) name` that also gives the name a number, or `(7)` alone for a
/// name an earlier line gave that number.
///
/// Returns the name, or why the line names none.
fn function_named<'a>(said: &'a str, names_by_number: &mut HashMap<&'a str, &'a str>) -> Result<&'a str, String> {
    let said: &str = said.trim();
    let Some(numbered) = said.strip_prefix('(') else { return Ok(said) };
    let (number, name): (&str, &str) = numbered.split_once(')').ok_or_else(|| format!("`{said}` opens a number and does not close it"))?;
    let name: &str = name.trim();
    if name.is_empty() {
        return names_by_number.get(number).copied().ok_or_else(|| format!("no line before gave a name to function number {number}"));
    }
    names_by_number.insert(number, name);
    Ok(name)
}

/// Reads the file Valgrind's callgrind tool wrote for one scenario.
///
/// Needed so the job that counts keeps more than a total. Format:
/// <https://valgrind.org/docs/manual/cl-format.html>. A line that starts
/// with a number is instructions of the function the last `fn=` line
/// named; the one line after a `calls=` line is what the called function
/// ran in all, and is left out here.
///
/// Returns each function's own instructions, most first and by name
/// among equals; functions that ran none are left out. An error says
/// which line could not be read.
pub fn counts_from_callgrind(callgrind: &str) -> Result<Vec<FunctionCount>, String> {
    let mut names_by_number: HashMap<&str, &str> = HashMap::new();
    let mut instructions_by_function: HashMap<&str, u64> = HashMap::new();
    let mut function: Option<&str> = None;
    let mut is_cost_of_a_call: bool = false;
    // How many numbers of a line say where in the program, before the
    // first one that counts. One (a source line) unless the file says more.
    let mut position_count: usize = 1;
    for (index, line) in callgrind.lines().enumerate() {
        let refused = |why: String| -> String { format!("callgrind line {}: {why}: {line}", index + 1) };
        if let Some(positions) = line.strip_prefix("positions:") {
            position_count = positions.split_whitespace().count().max(1);
        } else if let Some(said) = line.strip_prefix("fn=") {
            function = Some(function_named(said, &mut names_by_number).map_err(refused)?);
        } else if let Some(said) = line.strip_prefix("cfn=") {
            function_named(said, &mut names_by_number).map_err(refused)?;
        } else if line.starts_with("calls=") {
            is_cost_of_a_call = true;
        } else if line.starts_with(|first: char| first.is_ascii_digit() || matches!(first, '+' | '-' | '*')) {
            if std::mem::take(&mut is_cost_of_a_call) {
                continue;
            }
            let Some(count) = line.split_whitespace().nth(position_count) else { continue };
            let count: u64 = count.parse().map_err(|_| refused(format!("`{count}` is no count")))?;
            let function: &str = function.ok_or_else(|| refused("instructions before any `fn=` line".to_string()))?;
            *instructions_by_function.entry(name_without_hash(function)).or_default() += count;
        }
    }
    let mut counts: Vec<FunctionCount> = instructions_by_function.into_iter().filter(|(_, instructions)| *instructions > 0).map(|(function, instructions)| FunctionCount { function: function.to_string(), instructions }).collect();
    counts.sort_by(|a, b| b.instructions.cmp(&a.instructions).then_with(|| a.function.cmp(&b.function)));
    Ok(counts)
}

/// Sum of a list's counts: a scenario's instructions, when the list
/// holds every function.
pub fn total(counts: &[FunctionCount]) -> u64 {
    counts.iter().map(|count| count.instructions).sum()
}

/// A scenario's list as the `perf-data` branch keeps it: lines of `#`
/// that say what it is, then `<instructions> <function>` a line, each
/// ended by a line feed.
///
/// No date in it, so two runs of one commit that count the same write
/// the same file.
pub fn list(scenario: &str, commit: &str, counts: &[FunctionCount]) -> String {
    let mut text: String = format!("# `{scenario}` at commit {commit}: instructions by function, {} in all.\n# A function's own code only: what it calls is counted at the called function.\n", total(counts));
    for count in counts {
        text.push_str(&format!("{} {}\n", count.instructions, count.function));
    }
    text
}

/// A scenario's list that says why it has no counts.
pub fn list_without_counts(scenario: &str, commit: &str, why: &str) -> String {
    format!("# `{scenario}` at commit {commit}: no list.\n# {}\n", why.replace('\n', " "))
}

/// Reads a list `list` wrote.
///
/// Returns its counts in the file's order, or which line is no count.
pub fn counts_from_list(text: &str) -> Result<Vec<FunctionCount>, String> {
    let mut counts: Vec<FunctionCount> = Vec::new();
    for (index, line) in text.lines().enumerate() {
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let count: Option<(u64, &str)> = line.split_once(' ').and_then(|(instructions, function)| Some((instructions.parse().ok()?, function)));
        match count {
            Some((instructions, function)) => counts.push(FunctionCount { function: function.to_string(), instructions }),
            None => return Err(format!("list line {} is not `<instructions> <function>`: {line}", index + 1)),
        }
    }
    Ok(counts)
}

/// Functions whose count differs between two lists of one scenario.
///
/// Returns them with the largest difference first, a rise before a fall
/// of the same size, then by name.
pub fn moves(before: &[FunctionCount], after: &[FunctionCount]) -> Vec<FunctionMove> {
    let mut both_by_function: HashMap<&str, (u64, u64)> = HashMap::new();
    for count in before {
        both_by_function.entry(&count.function).or_default().0 += count.instructions;
    }
    for count in after {
        both_by_function.entry(&count.function).or_default().1 += count.instructions;
    }
    let mut moved: Vec<FunctionMove> = both_by_function.into_iter().filter(|(_, (before, after))| before != after).map(|(function, (before, after))| FunctionMove { function: function.to_string(), before, after }).collect();
    moved.sort_by(|a, b| b.difference().abs().cmp(&a.difference().abs()).then_with(|| b.difference().cmp(&a.difference())).then_with(|| a.function.cmp(&b.function)));
    moved
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A file as Valgrind writes it: names numbered at first use, two
    /// copies of one generic function, a call whose line follows a
    /// `calls=` line.
    const CALLGRIND: &str = "# callgrind format
version: 1
creator: callgrind-3.22.0
pid: 4242
cmd:  target/release/storvald-perf run still_frame
part: 1

desc: I1 cache:
desc: Trigger: Program termination

positions: line
events: Ir
summary: 1230

ob=(1) /work/target/release/storvald-perf
fl=(1) ???
fn=(1) storvald_perf_measured
0 20
cfn=(2) gogo_ui::runtime::Runtime::layout::h0123456789abcdef
calls=1 0
0 1210
fn=(2)
0 400
+3 300
cfn=(3) _ZN7gogo_ui4page7PropKey4word17hfedcba9876543210E
calls=2 0
* 9999
fn=(3)
0 10
fn=(4) gogo_ui::runtime::Runtime::layout::h1111111111111111
0 500
fn=(5) never_ran
0 0

totals: 1230
";

    fn count(function: &str, instructions: u64) -> FunctionCount {
        FunctionCount { function: function.to_string(), instructions }
    }

    #[test]
    fn a_function_s_count_is_its_own_code_and_not_what_it_calls() {
        let counts: Vec<FunctionCount> = counts_from_callgrind(CALLGRIND).expect("the file reads");
        assert_eq!(counts, [count("gogo_ui::runtime::Runtime::layout", 1200), count("storvald_perf_measured", 20), count("_ZN7gogo_ui4page7PropKey4word", 10)], "most first; the two copies of `layout` are one name; the lines after `calls=` are left out; a function that ran nothing is left out");
        assert_eq!(total(&counts), 1230, "the sum is the file's `totals:`");
    }

    #[test]
    fn a_file_with_two_places_a_line_counts_the_number_after_them() {
        let counts: Vec<FunctionCount> = counts_from_callgrind("positions: instr line\nevents: Ir\nfn=(1) one\n0x1000 12 7\n+4 * 5\n+2 *\n").expect("the file reads");
        assert_eq!(counts, [count("one", 12)], "the last line says a place and no count");
    }

    #[test]
    fn a_name_keeps_what_is_no_hash() {
        assert_eq!(name_without_hash("core::hash::Hasher::write::h0123456789abcdef"), "core::hash::Hasher::write");
        assert_eq!(name_without_hash("core::hash::Hasher::write"), "core::hash::Hasher::write");
        assert_eq!(name_without_hash("a::handle"), "a::handle", "`::h` with no sixteen hex digits after it");
        assert_eq!(name_without_hash("a::h0123456789abcdeg"), "a::h0123456789abcdeg", "the last letter is no hex digit");
        assert_eq!(name_without_hash("_ZN1a1b17h0123456789abcdefE"), "_ZN1a1b");
    }

    #[test]
    fn a_file_that_cannot_be_read_says_its_line() {
        let no_function: String = counts_from_callgrind("events: Ir\n0 5\n").expect_err("instructions of no function");
        assert!(no_function.contains("line 2") && no_function.contains("before any `fn=`"), "{no_function}");
        let no_name: String = counts_from_callgrind("events: Ir\nfn=(7)\n0 5\n").expect_err("a number no line gave a name");
        assert!(no_name.contains("line 2") && no_name.contains("function number 7"), "{no_name}");
        let no_count: String = counts_from_callgrind("events: Ir\nfn=one\n0 many\n").expect_err("a count that is no number");
        assert!(no_count.contains("line 3") && no_count.contains("`many`"), "{no_count}");
    }

    #[test]
    fn a_list_comes_back_from_its_text() {
        let counts: Vec<FunctionCount> = vec![count("<gogo_ui::page::PropKey as core::cmp::PartialEq>::eq", 40), count("one", 2)];
        let text: String = list("still_frame", "abc1234", &counts);
        assert_eq!(text, "# `still_frame` at commit abc1234: instructions by function, 42 in all.\n# A function's own code only: what it calls is counted at the called function.\n40 <gogo_ui::page::PropKey as core::cmp::PartialEq>::eq\n2 one\n");
        assert_eq!(counts_from_list(&text), Ok(counts));
        let refused: String = counts_from_list("# a list\nmany one\n").expect_err("a line that is no count");
        assert!(refused.contains("line 2") && refused.contains("many one"), "{refused}");
        assert_eq!(counts_from_list(&list_without_counts("still_frame", "abc1234", "valgrind\nfailed")), Ok(Vec::new()), "a list that says why it has no counts reads as empty");
    }

    #[test]
    fn the_functions_that_differ_are_named_largest_difference_first() {
        let before: Vec<FunctionCount> = vec![count("same", 100), count("grew", 100), count("fell", 100), count("gone", 30)];
        let after: Vec<FunctionCount> = vec![count("same", 100), count("grew", 150), count("fell", 50), count("new", 7)];
        let moved: Vec<(String, i64)> = moves(&before, &after).into_iter().map(|one| (one.function.clone(), one.difference())).collect();
        assert_eq!(moved, [("grew".to_string(), 50), ("fell".to_string(), -50), ("gone".to_string(), -30), ("new".to_string(), 7)]);
    }
}
