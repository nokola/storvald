# Storvald perf counts

What gogo and TurboDot cost in the measured program (`perf/` on `main`), counted on one of GitHub's Linux machines at every push to `main` and once a day.

- **Instructions**: counted by Valgrind while the step runs. Same code and same compiler give the same count within a small spread, on any machine.
- **Allocations, nodes placed, nodes measured**: exact. `perf/expected_counts.txt` on `main` holds them, and a test fails when one differs.
- Every run is rows in [history.csv](history.csv): date, commit, compiler, machine, scenario, metric, value.
- [functions/](functions) holds the last run's instructions by function, a scenario a file. The file's history has the runs before. After a rise, `storvald-perf functions-moved --before <file> --after <file>` names the functions that grew.

## Last run

2026-11-27T09:04:28Z, commit [2bbb7ae](https://github.com/nokola/storvald/commit/2bbb7ae735082a639116f9b7f0906cdb4207cb5d), rustc 1.100.0, machine `github-ubuntu26`. 50 runs so far.

| Scenario | What is counted | Instructions | Against run before | Allocations | Nodes placed | Nodes measured |
|---|---|---:|---:|---:|---:|---:|
| `compile_controls_page` | gogo compiles the controls page's file | 3,162,897 | -0.1 % | 3,517 | 0 | 0 |
| `start_program` | program is made and runs its first frame | 1,061,967 | -0.0 % | 341 | 6 | 8 |
| `open_controls_page` | tap that opens the controls page: page is made | 121,323 | +0.0 % | 67 | 0 | 0 |
| `first_frame_of_controls_page` | first frame of the controls page: its first layout | 4,942,386 | -0.0 % | 1,664 | 115 | 229 |
| `open_list_page` | tap that opens the list page: page and its 100 rows are made | 1,815,796 | +0.1 % | 2,561 | 0 | 0 |
| `first_frame_of_list_page` | first frame of the list page: its first layout | 21,703,951 | -0.0 % | 6,047 | 413 | 910 |
| `open_list_page_again` | same tap, at page's second open | 1,826,620 | +0.1 % | 2,556 | 0 | 0 |
| `first_frame_of_list_page_again` | first frame of the list page at its second open | 8,251,252 | -0.0 % | 599 | 413 | 910 |
| `draw_list_of_list_page` | draw list of the list page, walked whole | 2,702,037 | +0.0 % | 7 | 0 | 0 |
| `still_frame` | frame of the controls page while nothing changes | 3,173 | +0.0 % | 0 | 0 | 0 |
| `mouse_move` | mouse moves onto another row of the controls page, and the frame after | 113,726 | +0.0 % | 0 | 0 | 0 |
| `scroll_step` | finger pans the list one step, and the frame after | 3,347,751 | +0.0 % | 0 | 0 | 0 |
| `slider_drag_step` | finger drags a slider one step, and the frame after | 62,970 | -0.0 % | 0 | 0 | 0 |
| `text_change` | code writes a new text to one label, and the frame after | 68,989 | +0.0 % | 3 | 0 | 2 |
| `slide_frame` | frame while the controls page slides in | 3,819 | +0.0 % | 0 | 0 | 0 |
| `leave_and_come_back` | controls page leaves and first page comes back, to the end of both anims | 118,945 | +0.0 % | 0 | 0 | 0 |

## Numbers that moved

A number that differs from the run before: an instruction count by more than 0.2 %, an exact count at all. Newest first, at most 60. "Also changed" names what else differs between the two runs; empty means the code alone can have moved the number.

| Run | Commit | Scenario | Metric | Before | After | Change | Also changed |
|---|---|---|---|---:|---:|---:|---|
| 2026-11-27T02:15:38Z | [c521c0c](https://github.com/nokola/storvald/commit/c521c0c4d4c5de7c5b383d6884a186eb15c6a1d9) | `start_program` | instructions | 1,177,330 | 1,062,111 | -9.8 % |  |
| 2026-11-27T02:15:38Z | [c521c0c](https://github.com/nokola/storvald/commit/c521c0c4d4c5de7c5b383d6884a186eb15c6a1d9) | `start_program` | allocations | 343 | 341 | -0.6 % |  |
| 2026-11-26T22:01:14Z | [cf3dc54](https://github.com/nokola/storvald/commit/cf3dc5434b6c5189f0af76af7dc64c88e2ce02e9) | `first_frame_of_list_page` | instructions | 21,551,969 | 21,704,622 | +0.7 % |  |
| 2026-11-26T22:01:14Z | [cf3dc54](https://github.com/nokola/storvald/commit/cf3dc5434b6c5189f0af76af7dc64c88e2ce02e9) | `first_frame_of_controls_page` | instructions | 4,905,233 | 4,942,394 | +0.8 % |  |
| 2026-11-26T22:01:14Z | [cf3dc54](https://github.com/nokola/storvald/commit/cf3dc5434b6c5189f0af76af7dc64c88e2ce02e9) | `start_program` | instructions | 1,173,952 | 1,177,323 | +0.3 % |  |
| 2026-11-25T23:55:06Z | [a44cc70](https://github.com/nokola/storvald/commit/a44cc70e3422e9e45ecf394c4f8f66508e4952a4) | `leave_and_come_back` | instructions | 118,359 | 118,893 | +0.5 % |  |
| 2026-11-25T23:55:06Z | [a44cc70](https://github.com/nokola/storvald/commit/a44cc70e3422e9e45ecf394c4f8f66508e4952a4) | `slide_frame` | instructions | 3,805 | 3,820 | +0.4 % |  |
| 2026-11-25T23:55:06Z | [a44cc70](https://github.com/nokola/storvald/commit/a44cc70e3422e9e45ecf394c4f8f66508e4952a4) | `text_change` | instructions | 68,762 | 68,975 | +0.3 % |  |
| 2026-11-25T23:55:06Z | [a44cc70](https://github.com/nokola/storvald/commit/a44cc70e3422e9e45ecf394c4f8f66508e4952a4) | `slider_drag_step` | instructions | 62,701 | 62,928 | +0.4 % |  |
| 2026-11-25T23:55:06Z | [a44cc70](https://github.com/nokola/storvald/commit/a44cc70e3422e9e45ecf394c4f8f66508e4952a4) | `open_list_page_again` | instructions | 1,835,005 | 1,826,878 | -0.4 % |  |
| 2026-11-25T23:55:06Z | [a44cc70](https://github.com/nokola/storvald/commit/a44cc70e3422e9e45ecf394c4f8f66508e4952a4) | `open_list_page` | instructions | 1,820,684 | 1,813,445 | -0.4 % |  |
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
