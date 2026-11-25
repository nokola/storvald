# Storvald perf counts

What gogo and TurboDot cost in the measured program (`perf/` on `main`), counted on one of GitHub's Linux machines at every push to `main` and once a day.

- **Instructions**: counted by Valgrind while the step runs. Same code and same compiler give the same count within a small spread, on any machine.
- **Allocations, nodes placed, nodes measured**: exact. `perf/expected_counts.txt` on `main` holds them, and a test fails when one differs.
- Every run is rows in [history.csv](history.csv): date, commit, compiler, machine, scenario, metric, value.
- [functions/](functions) holds the last run's instructions by function, a scenario a file. The file's history has the runs before. After a rise, `storvald-perf functions-moved --before <file> --after <file>` names the functions that grew.

## Last run

2026-11-25T22:31:42Z, commit [b358ab2](https://github.com/nokola/storvald/commit/b358ab2dd2b8a208419bef41caec2e35eafd44e3), rustc 1.100.0, machine `github-ubuntu26`. 32 runs so far.

| Scenario | What is counted | Instructions | Against run before | Allocations | Nodes placed | Nodes measured |
|---|---|---:|---:|---:|---:|---:|
| `compile_controls_page` | gogo compiles the controls page's file | 3,162,053 | -0.1 % | 3,517 | 0 | 0 |
| `start_program` | program is made and runs its first frame | 1,173,077 | -0.0 % | 343 | 6 | 8 |
| `open_controls_page` | tap that opens the controls page: page is made | 121,357 | -0.0 % | 67 | 0 | 0 |
| `first_frame_of_controls_page` | first frame of the controls page: its first layout | 4,896,464 | -0.0 % | 1,664 | 115 | 229 |
| `open_list_page` | tap that opens the list page: page and its 100 rows are made | 1,820,684 | -0.0 % | 2,561 | 0 | 0 |
| `first_frame_of_list_page` | first frame of the list page: its first layout | 21,524,668 | +0.0 % | 6,047 | 413 | 910 |
| `open_list_page_again` | same tap, at page's second open | 1,835,005 | +0.0 % | 2,556 | 0 | 0 |
| `first_frame_of_list_page_again` | first frame of the list page at its second open | 8,246,297 | -0.0 % | 599 | 413 | 910 |
| `draw_list_of_list_page` | draw list of the list page, walked whole | 2,702,056 | +0.0 % | 7 | 0 | 0 |
| `still_frame` | frame of the controls page while nothing changes | 3,164 | +0.1 % | 0 | 0 | 0 |
| `mouse_move` | mouse moves onto another row of the controls page, and the frame after | 113,505 | +0.0 % | 0 | 0 | 0 |
| `scroll_step` | finger pans the list one step, and the frame after | 3,347,572 | +0.0 % | 0 | 0 | 0 |
| `slider_drag_step` | finger drags a slider one step, and the frame after | 62,701 | +0.0 % | 0 | 0 | 0 |
| `text_change` | code writes a new text to one label, and the frame after | 68,762 | +0.0 % | 3 | 0 | 2 |
| `slide_frame` | frame while the controls page slides in | 3,805 | +0.1 % | 0 | 0 | 0 |
| `leave_and_come_back` | controls page leaves and first page comes back, to the end of both anims | 118,359 | +0.0 % | 0 | 0 | 0 |

## Numbers that moved

A number that differs from the run before: an instruction count by more than 0.2 %, an exact count at all. Newest first, at most 60. "Also changed" names what else differs between the two runs; empty means the code alone can have moved the number.

| Run | Commit | Scenario | Metric | Before | After | Change | Also changed |
|---|---|---|---|---:|---:|---:|---|
| 2026-11-25T18:13:38Z | [2e6144f](https://github.com/nokola/storvald/commit/2e6144f143345584c7a1feddf2c0468edf3fe383) | `open_list_page_again` | instructions | 1,830,203 | 1,834,238 | +0.2 % |  |
| 2026-11-25T07:28:10Z | [04d7382](https://github.com/nokola/storvald/commit/04d73820d21b5359de763aefbc5b47e6bfe5669b) | `slide_frame` | instructions | 3,819 | 3,804 | -0.4 % |  |
| 2026-11-25T07:28:10Z | [04d7382](https://github.com/nokola/storvald/commit/04d73820d21b5359de763aefbc5b47e6bfe5669b) | `open_list_page_again` | instructions | 1,826,415 | 1,830,203 | +0.2 % |  |
| 2026-11-25T07:28:10Z | [04d7382](https://github.com/nokola/storvald/commit/04d73820d21b5359de763aefbc5b47e6bfe5669b) | `open_list_page` | instructions | 1,813,840 | 1,818,801 | +0.3 % |  |
| 2026-11-25T04:57:50Z | [38ccb85](https://github.com/nokola/storvald/commit/38ccb856e36a4426111a6a674dc986d15677af86) | `leave_and_come_back` | instructions | 117,702 | 118,464 | +0.6 % |  |
| 2026-11-25T04:57:50Z | [38ccb85](https://github.com/nokola/storvald/commit/38ccb856e36a4426111a6a674dc986d15677af86) | `slide_frame` | instructions | 3,801 | 3,819 | +0.5 % |  |
| 2026-11-25T04:57:50Z | [38ccb85](https://github.com/nokola/storvald/commit/38ccb856e36a4426111a6a674dc986d15677af86) | `mouse_move` | instructions | 112,639 | 113,522 | +0.8 % |  |
| 2026-11-25T04:57:50Z | [38ccb85](https://github.com/nokola/storvald/commit/38ccb856e36a4426111a6a674dc986d15677af86) | `open_list_page_again` | instructions | 1,821,614 | 1,826,415 | +0.3 % |  |
| 2026-11-25T04:57:50Z | [38ccb85](https://github.com/nokola/storvald/commit/38ccb856e36a4426111a6a674dc986d15677af86) | `open_controls_page` | instructions | 120,775 | 121,083 | +0.3 % |  |
| 2026-11-25T04:57:50Z | [38ccb85](https://github.com/nokola/storvald/commit/38ccb856e36a4426111a6a674dc986d15677af86) | `compile_controls_page` | instructions | 3,152,327 | 3,164,384 | +0.4 % |  |
| 2026-11-25T02:39:44Z | [ac2d143](https://github.com/nokola/storvald/commit/ac2d143232d109810faed65ae46094536437b7b1) | `leave_and_come_back` | instructions | 254,930 | 117,702 | -53.8 % |  |
| 2026-11-25T02:39:44Z | [ac2d143](https://github.com/nokola/storvald/commit/ac2d143232d109810faed65ae46094536437b7b1) | `leave_and_come_back` | allocations | 1 | 0 | -100.0 % |  |
| 2026-11-25T02:39:44Z | [ac2d143](https://github.com/nokola/storvald/commit/ac2d143232d109810faed65ae46094536437b7b1) | `text_change` | instructions | 162,670 | 68,731 | -57.7 % |  |
| 2026-11-25T02:39:44Z | [ac2d143](https://github.com/nokola/storvald/commit/ac2d143232d109810faed65ae46094536437b7b1) | `slider_drag_step` | instructions | 144,285 | 62,673 | -56.6 % |  |
| 2026-11-25T02:39:44Z | [ac2d143](https://github.com/nokola/storvald/commit/ac2d143232d109810faed65ae46094536437b7b1) | `scroll_step` | instructions | 3,456,416 | 3,347,274 | -3.2 % |  |
| 2026-11-25T02:39:44Z | [ac2d143](https://github.com/nokola/storvald/commit/ac2d143232d109810faed65ae46094536437b7b1) | `mouse_move` | instructions | 193,570 | 112,639 | -41.8 % |  |
| 2026-11-25T02:39:44Z | [ac2d143](https://github.com/nokola/storvald/commit/ac2d143232d109810faed65ae46094536437b7b1) | `first_frame_of_list_page_again` | allocations | 608 | 599 | -1.5 % |  |
| 2026-11-25T02:39:44Z | [ac2d143](https://github.com/nokola/storvald/commit/ac2d143232d109810faed65ae46094536437b7b1) | `open_list_page_again` | instructions | 1,827,727 | 1,821,614 | -0.3 % |  |
| 2026-11-25T02:39:44Z | [ac2d143](https://github.com/nokola/storvald/commit/ac2d143232d109810faed65ae46094536437b7b1) | `first_frame_of_list_page` | allocations | 6,057 | 6,047 | -0.2 % |  |
| 2026-11-25T02:39:44Z | [ac2d143](https://github.com/nokola/storvald/commit/ac2d143232d109810faed65ae46094536437b7b1) | `open_list_page` | instructions | 1,818,643 | 1,811,104 | -0.4 % |  |
| 2026-11-25T02:39:44Z | [ac2d143](https://github.com/nokola/storvald/commit/ac2d143232d109810faed65ae46094536437b7b1) | `first_frame_of_controls_page` | allocations | 1,670 | 1,664 | -0.4 % |  |
| 2026-11-25T02:39:44Z | [ac2d143](https://github.com/nokola/storvald/commit/ac2d143232d109810faed65ae46094536437b7b1) | `open_controls_page` | instructions | 121,208 | 120,775 | -0.4 % |  |
| 2026-11-25T02:39:44Z | [ac2d143](https://github.com/nokola/storvald/commit/ac2d143232d109810faed65ae46094536437b7b1) | `start_program` | allocations | 348 | 343 | -1.4 % |  |
| 2026-11-24T23:41:23Z | [015ecfd](https://github.com/nokola/storvald/commit/015ecfd4f7c78d9b9113470b3530df79d7fe1221) | `open_list_page_again` | instructions | 1,821,085 | 1,829,994 | +0.5 % |  |
| 2026-11-24T23:41:23Z | [015ecfd](https://github.com/nokola/storvald/commit/015ecfd4f7c78d9b9113470b3530df79d7fe1221) | `open_list_page` | instructions | 1,811,686 | 1,820,874 | +0.5 % |  |
| 2026-11-24T22:04:42Z | [656b7cd](https://github.com/nokola/storvald/commit/656b7cd1f0d2c4cc007f92ccf47d6b2c54d9bf39) | `leave_and_come_back` | instructions | 255,369 | 254,485 | -0.3 % |  |
| 2026-11-24T22:04:42Z | [656b7cd](https://github.com/nokola/storvald/commit/656b7cd1f0d2c4cc007f92ccf47d6b2c54d9bf39) | `slide_frame` | instructions | 3,840 | 3,804 | -0.9 % |  |
| 2026-11-24T22:04:42Z | [656b7cd](https://github.com/nokola/storvald/commit/656b7cd1f0d2c4cc007f92ccf47d6b2c54d9bf39) | `scroll_step` | instructions | 3,441,232 | 3,455,345 | +0.4 % |  |
| 2026-11-24T22:04:42Z | [656b7cd](https://github.com/nokola/storvald/commit/656b7cd1f0d2c4cc007f92ccf47d6b2c54d9bf39) | `still_frame` | instructions | 3,171 | 3,163 | -0.3 % |  |
| 2026-11-24T22:04:42Z | [656b7cd](https://github.com/nokola/storvald/commit/656b7cd1f0d2c4cc007f92ccf47d6b2c54d9bf39) | `draw_list_of_list_page` | instructions | 2,687,709 | 2,701,854 | +0.5 % |  |
| 2026-11-24T22:04:42Z | [656b7cd](https://github.com/nokola/storvald/commit/656b7cd1f0d2c4cc007f92ccf47d6b2c54d9bf39) | `first_frame_of_list_page_again` | instructions | 8,209,436 | 8,235,822 | +0.3 % |  |
| 2026-11-24T21:51:34Z | [31fc7d0](https://github.com/nokola/storvald/commit/31fc7d00711c11bfddaa7a25d92edafdc57a4d93) | `leave_and_come_back` | instructions | 254,482 | 255,369 | +0.3 % |  |
| 2026-11-24T21:51:34Z | [31fc7d0](https://github.com/nokola/storvald/commit/31fc7d00711c11bfddaa7a25d92edafdc57a4d93) | `slide_frame` | instructions | 3,804 | 3,840 | +0.9 % |  |
| 2026-11-24T21:51:34Z | [31fc7d0](https://github.com/nokola/storvald/commit/31fc7d00711c11bfddaa7a25d92edafdc57a4d93) | `scroll_step` | instructions | 3,455,331 | 3,441,232 | -0.4 % |  |
| 2026-11-24T21:51:34Z | [31fc7d0](https://github.com/nokola/storvald/commit/31fc7d00711c11bfddaa7a25d92edafdc57a4d93) | `still_frame` | instructions | 3,163 | 3,171 | +0.3 % |  |
| 2026-11-24T21:51:34Z | [31fc7d0](https://github.com/nokola/storvald/commit/31fc7d00711c11bfddaa7a25d92edafdc57a4d93) | `draw_list_of_list_page` | instructions | 2,701,854 | 2,687,709 | -0.5 % |  |
| 2026-11-24T21:51:34Z | [31fc7d0](https://github.com/nokola/storvald/commit/31fc7d00711c11bfddaa7a25d92edafdc57a4d93) | `first_frame_of_list_page_again` | instructions | 8,235,839 | 8,209,436 | -0.3 % |  |
| 2026-11-24T21:49:01Z | [e2b0aa7](https://github.com/nokola/storvald/commit/e2b0aa7919fba6bb5d01d5a5357ed6ea3f201122) | `slide_frame` | instructions | 3,812 | 3,804 | -0.2 % |  |
| 2026-11-24T21:38:11Z | [7a72309](https://github.com/nokola/storvald/commit/7a72309bafe555456af3837d09bfb157aea8d1eb) | `slide_frame` | instructions | 3,803 | 3,812 | +0.2 % |  |
| 2026-11-24T20:55:35Z | [b6a999e](https://github.com/nokola/storvald/commit/b6a999eec64cb873af53cdcc2a07f915b314dd1c) | `slide_frame` | instructions | 3,814 | 3,803 | -0.3 % |  |
| 2026-11-24T20:47:54Z | [7d247ad](https://github.com/nokola/storvald/commit/7d247adbc418bc5c4a8c1fe661ffbd6f6c7d8138) | `slide_frame` | instructions | 3,792 | 3,814 | +0.6 % |  |
| 2026-11-24T20:47:54Z | [7d247ad](https://github.com/nokola/storvald/commit/7d247adbc418bc5c4a8c1fe661ffbd6f6c7d8138) | `still_frame` | instructions | 3,153 | 3,168 | +0.5 % |  |
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
