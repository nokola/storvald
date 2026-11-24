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

`.github/workflows/perf.yml` adds a run at every push to `main` and once a day. A run fails when a number moved against the run before on the same compiler and machine.

## Commands

```text
cargo run -p storvald-perf -- list             names of the scenarios
cargo run -p storvald-perf -- run still_frame  one scenario's counts
cargo run -p storvald-perf -- counts           every scenario's counts
cargo run -p storvald-perf -- write-expected   writes expected_counts.txt again
```

When a change moves a count on purpose, run `write-expected` and commit the file with the change. The file's history then names the change behind every number.

## A new scenario

One function in `src/scenarios.rs` and one line in `SCENARIOS`. The function sets the program up, runs one step inside `counted`, and checks that the step did what its name says. Then run `write-expected`.
