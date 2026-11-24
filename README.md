# Storvald perf counts

What gogo and TurboDot cost in the measured program (`perf/` on `main`), counted on one of GitHub's Linux machines at every push to `main` and once a day.

- **Instructions**: counted by Valgrind while the step runs. Same code and same compiler give the same count within a small spread, on any machine.
- **Allocations, nodes placed, nodes measured**: exact. `perf/expected_counts.txt` on `main` holds them, and a test fails when one differs.
- Every run is rows in [history.csv](history.csv): date, commit, compiler, machine, scenario, metric, value.

## Last run

2026-11-24T03:36:37Z, commit [3a9d6ce](https://github.com/nokola/storvald/commit/3a9d6ce0de58defe8a88f40f5ee8545f376999af), rustc 1.100.0, machine `github-ubuntu26-20261116.202.1`. 2 runs so far.

| Scenario | What is counted | Instructions | Against run before | Allocations | Nodes placed | Nodes measured |
|---|---|---:|---:|---:|---:|---:|
| `compile_controls_page` | gogo compiles the controls page's file | 3,144,209 | +0.0 % | 3,518 | 0 | 0 |
| `start_program` | program is made and runs its first frame | 1,170,812 | -0.0 % | 346 | 6 | 8 |
| `open_controls_page` | tap that opens the controls page: page is made | 119,285 | +0.0 % | 66 | 0 | 0 |
| `first_frame_of_controls_page` | first frame of the controls page: its first layout | 4,886,214 | -0.0 % | 1,820 | 115 | 229 |
| `open_list_page` | tap that opens the list page: page and its 100 rows are made | 1,810,027 | +0.0 % | 2,560 | 0 | 0 |
| `first_frame_of_list_page` | first frame of the list page: its first layout | 46,377,244 | -0.0 % | 6,757 | 413 | 910 |
| `open_list_page_again` | same tap, at page's second open | 1,822,827 | +0.0 % | 2,555 | 0 | 0 |
| `first_frame_of_list_page_again` | first frame of the list page at its second open | 33,096,874 | -0.0 % | 1,308 | 413 | 910 |
| `draw_list_of_list_page` | draw list of the list page, walked whole | 11,765,765 | +0.0 % | 7 | 0 | 0 |
| `still_frame` | frame of the controls page while nothing changes | 3,134 | +0.0 % | 0 | 0 | 0 |
| `mouse_move` | mouse moves onto another row of the controls page, and the frame after | 191,177 | +0.0 % | 0 | 0 | 0 |
| `scroll_step` | finger pans the list one step, and the frame after | 17,339,917 | +0.0 % | 422 | 407 | 0 |
| `slider_drag_step` | finger drags a slider one step, and the frame after | 143,875 | +0.0 % | 0 | 0 | 0 |
| `text_change` | code writes a new text to one label, and the frame after | 523,207 | +0.0 % | 124 | 109 | 7 |
| `slide_frame` | frame while the controls page slides in | 3,781 | +0.0 % | 0 | 0 | 0 |
| `leave_and_come_back` | controls page leaves and first page comes back, to the end of both anims | 250,821 | +0.0 % | 1 | 0 | 0 |

## Numbers that moved

A number that differs from the run before: an instruction count by more than 1 %, an exact count at all. Newest first, at most 60. "Also changed" names what else differs between the two runs; empty means the code alone can have moved the number.

None so far.
