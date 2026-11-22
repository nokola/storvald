# How to work in this repository

## What was asked

- A question gets an answer, then stop. Only "fix it", "do it", "go" or
  an explicit ask starts a change.
- Fix only what was listed. After a list of slips, "improve" means those
  items. Every other word stays as the user wrote it. Offer a wider
  rewrite in words; do not do it.
- "Write A" = paste A verbatim.
- Review = list what to change and the proposed text. Apply when told.
- After a misread, restate the new reading and get a yes first.

## Commits

The user commits and pushes. Leave the change in the working tree and
say what is in it. Hand over a commit message when asked.

## Proof

- Green counts only what ran. Compare the number of tests with the last
  run: a drop means tests were skipped, not fewer tests.
- A check has three outcomes: found, absent, did not run. When every row
  says the same, suspect the tool first.
- Report what you ran, what you did not run, and the counts.
- An exit code alone is never green: read the result lines.

## Files

- Reread a file before you rewrite it whole: another session may have
  changed it. Prefer small edits over a whole-file write.
- Text files are LF. After a tool that writes CRLF, convert, and check
  with `git ls-files --eol`.

## Build and test

- One crate while you work: `cargo test -p <crate>`.
- Everything before you report: `cargo test --workspace --no-fail-fast`.
- With the GPU tests, on a machine that has a GPU:
  `cargo test --workspace --no-fail-fast -- --include-ignored`.
- The crates as crates.io gets them: `sh scripts/test-packages.sh`.
- On a Windows PC, set `WGPU_BACKEND=vulkan` for test runs. Many
  Direct3D 12 devices in one process have frozen a test run.
- Run a long build or a full test run where the user can watch it.

## Looking at a screen

- Never photograph the user's desktop. Never send mouse or key input to
  it.
- Drive pages with no window (`turbodot::Program`): a touch as numbers,
  `Program::ui().outline()` for the rectangles.
- To see pixels, draw the pages to a picture (`turbodot::Gpu::draw_program`,
  `Picture::save_png`) and read the file.

## Who else builds this code

FanRust, a shipping app, builds `gogo-ui`, `gogo-text` and `gogo-wgpu`
from this repository. A change to their public API breaks its build. Say
so before you make one. Nothing of that app may come into this
repository.
