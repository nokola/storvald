# Measured program

Small TurboDot program whose pages are opened, moved and closed while what that costs is counted. It answers one question over time: did a change make gogo or TurboDot do more work?

## What is counted

| Count | How | Where it is held |
|---|---|---|
| Allocations, nodes placed, nodes measured | the program counts them itself; exact | `expected_counts.txt`: a test fails when one differs |
| Instructions | Valgrind, on one of GitHub's Linux machines | `history.csv` on the `perf-data` branch |

No times. A time measured on a shared machine swings by more than the changes worth catching.

## Results

Branch [`perf-data`](https://github.com/nokola/storvald/tree/perf-data):

- `README.md`: the page to read. The last run, and every number that moved, with its commit.
- `history.csv`: every run as rows: date, commit, compiler, machine, scenario, metric, value.
- `functions/<scenario>.txt`: the last run's instructions by function, most first. A function's own code only: what it calls is counted at the called function. The file's history on the branch has the runs before; each of the branch's commits names the commit it measured.

`.github/workflows/perf.yml` adds a run at every push to `main` and once a day. A run fails when an instruction count rose against the run before on the same compiler and machine. A count that fell is listed on the page and passes. The exact counts are held by the test, so a run does not judge them again.

## Commands

```text
cargo run -p storvald-perf -- list             names of the scenarios
cargo run -p storvald-perf -- run still_frame  one scenario's counts
cargo run -p storvald-perf -- counts           every scenario's counts
cargo run -p storvald-perf -- write-expected   writes expected_counts.txt again
cargo run -p storvald-perf -- functions-moved --before <file> --after <file>
                                               functions that differ between two lists
```

When a change moves a count on purpose, run `write-expected` and commit the file with the change. The file's history then names the change behind every number.

## When an instruction count rose

A scenario's total says that it runs more instructions, not where. Take the scenario's file under `functions/` from the `perf-data` commit of the run before and from the one of the run that rose, and run `functions-moved` on the two. It prints the functions that differ, largest difference first:

```text
+15 turbodot::program::Program<S>::frame: 412 to 427
+15 in all: 3153 to 3168, 1 functions differ
```

## A new scenario

One function in `src/scenarios.rs` and one line in `SCENARIOS`. The function sets the program up, runs one step inside `counted`, and checks that the step did what its name says. Then run `write-expected`.
