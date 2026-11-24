# Storvald perf counts

What gogo and TurboDot cost in the measured program (`perf/` on `main`), counted on one of GitHub's Linux machines at every push to `main` and once a day.

- **Instructions**: counted by Valgrind while the step runs. Same code and same compiler give the same count within a small spread, on any machine.
- **Allocations, nodes placed, nodes measured**: exact. `perf/expected_counts.txt` on `main` holds them, and a test fails when one differs.
- Every run is rows in [history.csv](history.csv): date, commit, compiler, machine, scenario, metric, value.

## Last run

2026-11-24T06:12:13Z, commit [66da7cc](https://github.com/nokola/storvald/commit/66da7cc70973c1a56c50eec46dcd1b57e78200cd), rustc 1.100.0, machine `github-ubuntu26`. 5 runs so far.

| Scenario | What is counted | Instructions | Against run before | Allocations | Nodes placed | Nodes measured |
|---|---|---:|---:|---:|---:|---:|
| `compile_controls_page` | gogo compiles the controls page's file | 3,139,440 | -0.2 % | 3,518 | 0 | 0 |
| `start_program` | program is made and runs its first frame | 1,172,401 | +0.1 % | 348 | 6 | 8 |
| `open_controls_page` | tap that opens the controls page: page is made | 119,445 | +0.1 % | 66 | 0 | 0 |
| `first_frame_of_controls_page` | first frame of the controls page: its first layout | 4,861,742 | -0.5 % | 1,668 | 115 | 229 |
| `open_list_page` | tap that opens the list page: page and its 100 rows are made | 1,811,012 | +0.1 % | 2,560 | 0 | 0 |
| `first_frame_of_list_page` | first frame of the list page: its first layout | 46,271,773 | -0.2 % | 6,047 | 413 | 910 |
| `open_list_page_again` | same tap, at page's second open | 1,820,118 | -0.1 % | 2,555 | 0 | 0 |
| `first_frame_of_list_page_again` | first frame of the list page at its second open | 32,979,171 | -0.4 % | 598 | 413 | 910 |
| `draw_list_of_list_page` | draw list of the list page, walked whole | 11,765,755 | -0.0 % | 7 | 0 | 0 |
| `still_frame` | frame of the controls page while nothing changes | 3,132 | -0.2 % | 0 | 0 | 0 |
| `mouse_move` | mouse moves onto another row of the controls page, and the frame after | 191,256 | +0.0 % | 0 | 0 | 0 |
| `scroll_step` | finger pans the list one step, and the frame after | 13,303,823 | -23.3 % | 0 | 0 | 0 |
| `slider_drag_step` | finger drags a slider one step, and the frame after | 144,051 | +0.1 % | 0 | 0 | 0 |
| `text_change` | code writes a new text to one label, and the frame after | 163,128 | -68.8 % | 3 | 0 | 2 |
| `slide_frame` | frame while the controls page slides in | 3,772 | -0.3 % | 0 | 0 | 0 |
| `leave_and_come_back` | controls page leaves and first page comes back, to the end of both anims | 252,277 | +0.6 % | 1 | 0 | 0 |

## Numbers that moved

A number that differs from the run before: an instruction count by more than 0.2 %, an exact count at all. Newest first, at most 60. "Also changed" names what else differs between the two runs; empty means the code alone can have moved the number.

| Run | Commit | Scenario | Metric | Before | After | Change | Also changed |
|---|---|---|---|---:|---:|---:|---|
| 2026-11-24T06:12:13Z | [66da7cc](https://github.com/nokola/storvald/commit/66da7cc70973c1a56c50eec46dcd1b57e78200cd) | `leave_and_come_back` | instructions | 250,860 | 252,277 | +0.6 % |  |
| 2026-11-24T06:12:13Z | [66da7cc](https://github.com/nokola/storvald/commit/66da7cc70973c1a56c50eec46dcd1b57e78200cd) | `slide_frame` | instructions | 3,784 | 3,772 | -0.3 % |  |
| 2026-11-24T06:12:13Z | [66da7cc](https://github.com/nokola/storvald/commit/66da7cc70973c1a56c50eec46dcd1b57e78200cd) | `text_change` | instructions | 523,210 | 163,128 | -68.8 % |  |
| 2026-11-24T06:12:13Z | [66da7cc](https://github.com/nokola/storvald/commit/66da7cc70973c1a56c50eec46dcd1b57e78200cd) | `text_change` | nodes_measured | 7 | 2 | -71.4 % |  |
| 2026-11-24T06:12:13Z | [66da7cc](https://github.com/nokola/storvald/commit/66da7cc70973c1a56c50eec46dcd1b57e78200cd) | `text_change` | nodes_placed | 109 | 0 | -100.0 % |  |
| 2026-11-24T06:12:13Z | [66da7cc](https://github.com/nokola/storvald/commit/66da7cc70973c1a56c50eec46dcd1b57e78200cd) | `text_change` | allocations | 124 | 3 | -97.6 % |  |
| 2026-11-24T06:12:13Z | [66da7cc](https://github.com/nokola/storvald/commit/66da7cc70973c1a56c50eec46dcd1b57e78200cd) | `scroll_step` | instructions | 17,339,924 | 13,303,823 | -23.3 % |  |
| 2026-11-24T06:12:13Z | [66da7cc](https://github.com/nokola/storvald/commit/66da7cc70973c1a56c50eec46dcd1b57e78200cd) | `scroll_step` | nodes_placed | 407 | 0 | -100.0 % |  |
| 2026-11-24T06:12:13Z | [66da7cc](https://github.com/nokola/storvald/commit/66da7cc70973c1a56c50eec46dcd1b57e78200cd) | `scroll_step` | allocations | 422 | 0 | -100.0 % |  |
| 2026-11-24T06:12:13Z | [66da7cc](https://github.com/nokola/storvald/commit/66da7cc70973c1a56c50eec46dcd1b57e78200cd) | `first_frame_of_list_page_again` | instructions | 33,097,325 | 32,979,171 | -0.4 % |  |
| 2026-11-24T06:12:13Z | [66da7cc](https://github.com/nokola/storvald/commit/66da7cc70973c1a56c50eec46dcd1b57e78200cd) | `first_frame_of_list_page_again` | allocations | 1,308 | 598 | -54.3 % |  |
| 2026-11-24T06:12:13Z | [66da7cc](https://github.com/nokola/storvald/commit/66da7cc70973c1a56c50eec46dcd1b57e78200cd) | `first_frame_of_list_page` | instructions | 46,377,509 | 46,271,773 | -0.2 % |  |
| 2026-11-24T06:12:13Z | [66da7cc](https://github.com/nokola/storvald/commit/66da7cc70973c1a56c50eec46dcd1b57e78200cd) | `first_frame_of_list_page` | allocations | 6,757 | 6,047 | -10.5 % |  |
| 2026-11-24T06:12:13Z | [66da7cc](https://github.com/nokola/storvald/commit/66da7cc70973c1a56c50eec46dcd1b57e78200cd) | `first_frame_of_controls_page` | instructions | 4,886,554 | 4,861,742 | -0.5 % |  |
| 2026-11-24T06:12:13Z | [66da7cc](https://github.com/nokola/storvald/commit/66da7cc70973c1a56c50eec46dcd1b57e78200cd) | `first_frame_of_controls_page` | allocations | 1,820 | 1,668 | -8.4 % |  |
| 2026-11-24T06:12:13Z | [66da7cc](https://github.com/nokola/storvald/commit/66da7cc70973c1a56c50eec46dcd1b57e78200cd) | `start_program` | allocations | 346 | 348 | +0.6 % |  |
