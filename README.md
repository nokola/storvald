# Storvald perf counts

What gogo and TurboDot cost in the measured program (`perf/` on `main`), counted on one of GitHub's Linux machines at every push to `main` and once a day.

- **Instructions**: counted by Valgrind while the step runs. Same code and same compiler give the same count within a small spread, on any machine.
- **Allocations, nodes placed, nodes measured**: exact. `perf/expected_counts.txt` on `main` holds them, and a test fails when one differs.
- Every run is rows in [history.csv](history.csv): date, commit, compiler, machine, scenario, metric, value.

## Last run

2026-11-24T20:18:30Z, commit [01cbe3c](https://github.com/nokola/storvald/commit/01cbe3c0134b31cfe1daff3a78680aae5c97ad33), rustc 1.100.0, machine `github-ubuntu26`. 15 runs so far.

| Scenario | What is counted | Instructions | Against run before | Allocations | Nodes placed | Nodes measured |
|---|---|---:|---:|---:|---:|---:|
| `compile_controls_page` | gogo compiles the controls page's file | 3,142,121 | -0.3 % | 3,517 | 0 | 0 |
| `start_program` | program is made and runs its first frame | 1,174,748 | -0.0 % | 348 | 6 | 8 |
| `open_controls_page` | tap that opens the controls page: page is made | 120,960 | -0.0 % | 67 | 0 | 0 |
| `first_frame_of_controls_page` | first frame of the controls page: its first layout | 4,901,006 | -0.0 % | 1,670 | 115 | 229 |
| `open_list_page` | tap that opens the list page: page and its 100 rows are made | 1,809,507 | -0.4 % | 2,561 | 0 | 0 |
| `first_frame_of_list_page` | first frame of the list page: its first layout | 21,548,151 | +0.0 % | 6,057 | 413 | 910 |
| `open_list_page_again` | same tap, at page's second open | 1,819,980 | -0.5 % | 2,556 | 0 | 0 |
| `first_frame_of_list_page_again` | first frame of the list page at its second open | 8,240,166 | +0.0 % | 608 | 413 | 910 |
| `draw_list_of_list_page` | draw list of the list page, walked whole | 2,701,854 | +0.0 % | 7 | 0 | 0 |
| `still_frame` | frame of the controls page while nothing changes | 3,153 | -0.1 % | 0 | 0 | 0 |
| `mouse_move` | mouse moves onto another row of the controls page, and the frame after | 193,393 | -0.0 % | 0 | 0 | 0 |
| `scroll_step` | finger pans the list one step, and the frame after | 3,455,333 | -0.0 % | 0 | 0 | 0 |
| `slider_drag_step` | finger drags a slider one step, and the frame after | 144,102 | +0.0 % | 0 | 0 | 0 |
| `text_change` | code writes a new text to one label, and the frame after | 162,726 | -0.0 % | 3 | 0 | 2 |
| `slide_frame` | frame while the controls page slides in | 3,792 | -0.3 % | 0 | 0 | 0 |
| `leave_and_come_back` | controls page leaves and first page comes back, to the end of both anims | 254,293 | -0.0 % | 1 | 0 | 0 |

## Numbers that moved

A number that differs from the run before: an instruction count by more than 0.2 %, an exact count at all. Newest first, at most 60. "Also changed" names what else differs between the two runs; empty means the code alone can have moved the number.

| Run | Commit | Scenario | Metric | Before | After | Change | Also changed |
|---|---|---|---|---:|---:|---:|---|
| 2026-11-24T20:18:30Z | [01cbe3c](https://github.com/nokola/storvald/commit/01cbe3c0134b31cfe1daff3a78680aae5c97ad33) | `slide_frame` | instructions | 3,802 | 3,792 | -0.3 % |  |
| 2026-11-24T20:18:30Z | [01cbe3c](https://github.com/nokola/storvald/commit/01cbe3c0134b31cfe1daff3a78680aae5c97ad33) | `open_list_page_again` | instructions | 1,829,504 | 1,819,980 | -0.5 % |  |
| 2026-11-24T20:18:30Z | [01cbe3c](https://github.com/nokola/storvald/commit/01cbe3c0134b31cfe1daff3a78680aae5c97ad33) | `open_list_page` | instructions | 1,817,402 | 1,809,507 | -0.4 % |  |
| 2026-11-24T20:18:30Z | [01cbe3c](https://github.com/nokola/storvald/commit/01cbe3c0134b31cfe1daff3a78680aae5c97ad33) | `compile_controls_page` | instructions | 3,152,291 | 3,142,121 | -0.3 % |  |
| 2026-11-24T20:01:23Z | [6bcb78b](https://github.com/nokola/storvald/commit/6bcb78ba3d057b5de291f642ff5049899c71c20f) | `compile_controls_page` | instructions | 3,160,771 | 3,152,291 | -0.3 % |  |
| 2026-11-24T19:43:44Z | [cbc4051](https://github.com/nokola/storvald/commit/cbc40519fd51535bc53ada74f0fb21c1953ba3e9) | `scroll_step` | instructions | 3,448,078 | 3,455,336 | +0.2 % |  |
| 2026-11-24T19:43:44Z | [cbc4051](https://github.com/nokola/storvald/commit/cbc40519fd51535bc53ada74f0fb21c1953ba3e9) | `open_list_page_again` | instructions | 1,821,638 | 1,829,504 | +0.4 % |  |
| 2026-11-24T19:43:44Z | [cbc4051](https://github.com/nokola/storvald/commit/cbc40519fd51535bc53ada74f0fb21c1953ba3e9) | `open_list_page` | instructions | 1,811,357 | 1,817,402 | +0.3 % |  |
| 2026-11-24T19:43:44Z | [cbc4051](https://github.com/nokola/storvald/commit/cbc40519fd51535bc53ada74f0fb21c1953ba3e9) | `compile_controls_page` | instructions | 3,152,087 | 3,160,771 | +0.3 % |  |
| 2026-11-24T18:19:16Z | [035682e](https://github.com/nokola/storvald/commit/035682e69ab1adca99c88b19de485926764e2e80) | `leave_and_come_back` | instructions | 255,101 | 253,961 | -0.4 % |  |
| 2026-11-24T18:19:16Z | [035682e](https://github.com/nokola/storvald/commit/035682e69ab1adca99c88b19de485926764e2e80) | `slide_frame` | instructions | 3,886 | 3,798 | -2.3 % |  |
| 2026-11-24T18:19:16Z | [035682e](https://github.com/nokola/storvald/commit/035682e69ab1adca99c88b19de485926764e2e80) | `still_frame` | instructions | 3,202 | 3,159 | -1.3 % |  |
| 2026-11-24T18:02:40Z | [29b46a5](https://github.com/nokola/storvald/commit/29b46a58732d1e8f489032fbd62f429a7823117f) | `leave_and_come_back` | instructions | 254,321 | 255,101 | +0.3 % |  |
| 2026-11-24T18:02:40Z | [29b46a5](https://github.com/nokola/storvald/commit/29b46a58732d1e8f489032fbd62f429a7823117f) | `slide_frame` | instructions | 3,801 | 3,886 | +2.2 % |  |
| 2026-11-24T18:02:40Z | [29b46a5](https://github.com/nokola/storvald/commit/29b46a58732d1e8f489032fbd62f429a7823117f) | `still_frame` | instructions | 3,154 | 3,202 | +1.5 % |  |
| 2026-11-24T18:02:40Z | [29b46a5](https://github.com/nokola/storvald/commit/29b46a58732d1e8f489032fbd62f429a7823117f) | `first_frame_of_list_page_again` | instructions | 8,219,613 | 8,240,174 | +0.3 % |  |
| 2026-11-24T18:02:40Z | [29b46a5](https://github.com/nokola/storvald/commit/29b46a58732d1e8f489032fbd62f429a7823117f) | `open_list_page_again` | instructions | 1,819,738 | 1,823,498 | +0.2 % |  |
| 2026-11-24T18:02:40Z | [29b46a5](https://github.com/nokola/storvald/commit/29b46a58732d1e8f489032fbd62f429a7823117f) | `open_list_page` | instructions | 1,809,394 | 1,813,064 | +0.2 % |  |
| 2026-11-24T18:02:40Z | [29b46a5](https://github.com/nokola/storvald/commit/29b46a58732d1e8f489032fbd62f429a7823117f) | `start_program` | allocations | 350 | 348 | -0.6 % |  |
| 2026-11-24T18:02:40Z | [29b46a5](https://github.com/nokola/storvald/commit/29b46a58732d1e8f489032fbd62f429a7823117f) | `compile_controls_page` | instructions | 3,144,478 | 3,151,450 | +0.2 % |  |
| 2026-11-24T14:07:45Z | [d646974](https://github.com/nokola/storvald/commit/d64697456f2a2bb2e5e5c3670d928a136914f952) | `leave_and_come_back` | instructions | 253,159 | 254,321 | +0.5 % |  |
| 2026-11-24T14:07:45Z | [d646974](https://github.com/nokola/storvald/commit/d64697456f2a2bb2e5e5c3670d928a136914f952) | `slide_frame` | instructions | 3,784 | 3,801 | +0.4 % |  |
| 2026-11-24T14:07:45Z | [d646974](https://github.com/nokola/storvald/commit/d64697456f2a2bb2e5e5c3670d928a136914f952) | `mouse_move` | instructions | 192,657 | 193,183 | +0.3 % |  |
| 2026-11-24T14:07:45Z | [d646974](https://github.com/nokola/storvald/commit/d64697456f2a2bb2e5e5c3670d928a136914f952) | `still_frame` | instructions | 3,130 | 3,154 | +0.8 % |  |
| 2026-11-24T14:07:45Z | [d646974](https://github.com/nokola/storvald/commit/d64697456f2a2bb2e5e5c3670d928a136914f952) | `open_list_page_again` | instructions | 1,827,375 | 1,819,738 | -0.4 % |  |
| 2026-11-24T14:07:45Z | [d646974](https://github.com/nokola/storvald/commit/d64697456f2a2bb2e5e5c3670d928a136914f952) | `open_list_page` | instructions | 1,815,305 | 1,809,406 | -0.3 % |  |
| 2026-11-24T14:07:45Z | [d646974](https://github.com/nokola/storvald/commit/d64697456f2a2bb2e5e5c3670d928a136914f952) | `open_controls_page` | instructions | 120,216 | 121,036 | +0.7 % |  |
| 2026-11-24T14:07:45Z | [d646974](https://github.com/nokola/storvald/commit/d64697456f2a2bb2e5e5c3670d928a136914f952) | `compile_controls_page` | allocations | 3,518 | 3,517 | -0.0 % |  |
| 2026-11-24T07:56:14Z | [4d37fe1](https://github.com/nokola/storvald/commit/4d37fe1cf2acee1ac6e09d5446547ded4e636937) | `slide_frame` | instructions | 3,770 | 3,784 | +0.4 % |  |
| 2026-11-24T07:09:20Z | [8734e2b](https://github.com/nokola/storvald/commit/8734e2b2481e4c65f921eb2c5f050fceebb93a01) | `leave_and_come_back` | instructions | 252,277 | 253,261 | +0.4 % |  |
| 2026-11-24T07:09:20Z | [8734e2b](https://github.com/nokola/storvald/commit/8734e2b2481e4c65f921eb2c5f050fceebb93a01) | `text_change` | instructions | 163,128 | 162,263 | -0.5 % |  |
| 2026-11-24T07:09:20Z | [8734e2b](https://github.com/nokola/storvald/commit/8734e2b2481e4c65f921eb2c5f050fceebb93a01) | `slider_drag_step` | instructions | 144,051 | 143,559 | -0.3 % |  |
| 2026-11-24T07:09:20Z | [8734e2b](https://github.com/nokola/storvald/commit/8734e2b2481e4c65f921eb2c5f050fceebb93a01) | `scroll_step` | instructions | 13,303,823 | 3,447,204 | -74.1 % |  |
| 2026-11-24T07:09:20Z | [8734e2b](https://github.com/nokola/storvald/commit/8734e2b2481e4c65f921eb2c5f050fceebb93a01) | `mouse_move` | instructions | 191,256 | 192,657 | +0.7 % |  |
| 2026-11-24T07:09:20Z | [8734e2b](https://github.com/nokola/storvald/commit/8734e2b2481e4c65f921eb2c5f050fceebb93a01) | `draw_list_of_list_page` | instructions | 11,765,755 | 2,701,253 | -77.0 % |  |
| 2026-11-24T07:09:20Z | [8734e2b](https://github.com/nokola/storvald/commit/8734e2b2481e4c65f921eb2c5f050fceebb93a01) | `first_frame_of_list_page_again` | instructions | 32,979,171 | 8,218,328 | -75.1 % |  |
| 2026-11-24T07:09:20Z | [8734e2b](https://github.com/nokola/storvald/commit/8734e2b2481e4c65f921eb2c5f050fceebb93a01) | `first_frame_of_list_page_again` | allocations | 598 | 608 | +1.7 % |  |
| 2026-11-24T07:09:20Z | [8734e2b](https://github.com/nokola/storvald/commit/8734e2b2481e4c65f921eb2c5f050fceebb93a01) | `open_list_page_again` | instructions | 1,820,118 | 1,825,803 | +0.3 % |  |
| 2026-11-24T07:09:20Z | [8734e2b](https://github.com/nokola/storvald/commit/8734e2b2481e4c65f921eb2c5f050fceebb93a01) | `open_list_page_again` | allocations | 2,555 | 2,556 | +0.0 % |  |
| 2026-11-24T07:09:20Z | [8734e2b](https://github.com/nokola/storvald/commit/8734e2b2481e4c65f921eb2c5f050fceebb93a01) | `first_frame_of_list_page` | instructions | 46,271,773 | 21,536,188 | -53.5 % |  |
| 2026-11-24T07:09:20Z | [8734e2b](https://github.com/nokola/storvald/commit/8734e2b2481e4c65f921eb2c5f050fceebb93a01) | `first_frame_of_list_page` | allocations | 6,047 | 6,057 | +0.2 % |  |
| 2026-11-24T07:09:20Z | [8734e2b](https://github.com/nokola/storvald/commit/8734e2b2481e4c65f921eb2c5f050fceebb93a01) | `open_list_page` | allocations | 2,560 | 2,561 | +0.0 % |  |
| 2026-11-24T07:09:20Z | [8734e2b](https://github.com/nokola/storvald/commit/8734e2b2481e4c65f921eb2c5f050fceebb93a01) | `first_frame_of_controls_page` | instructions | 4,861,742 | 4,898,838 | +0.8 % |  |
| 2026-11-24T07:09:20Z | [8734e2b](https://github.com/nokola/storvald/commit/8734e2b2481e4c65f921eb2c5f050fceebb93a01) | `first_frame_of_controls_page` | allocations | 1,668 | 1,670 | +0.1 % |  |
| 2026-11-24T07:09:20Z | [8734e2b](https://github.com/nokola/storvald/commit/8734e2b2481e4c65f921eb2c5f050fceebb93a01) | `open_controls_page` | instructions | 119,445 | 120,170 | +0.6 % |  |
| 2026-11-24T07:09:20Z | [8734e2b](https://github.com/nokola/storvald/commit/8734e2b2481e4c65f921eb2c5f050fceebb93a01) | `open_controls_page` | allocations | 66 | 67 | +1.5 % |  |
| 2026-11-24T07:09:20Z | [8734e2b](https://github.com/nokola/storvald/commit/8734e2b2481e4c65f921eb2c5f050fceebb93a01) | `start_program` | instructions | 1,172,401 | 1,174,791 | +0.2 % |  |
| 2026-11-24T07:09:20Z | [8734e2b](https://github.com/nokola/storvald/commit/8734e2b2481e4c65f921eb2c5f050fceebb93a01) | `start_program` | allocations | 348 | 350 | +0.6 % |  |
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
