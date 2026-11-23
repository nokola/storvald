# Project instructions

Three parts in one repository:

- **Storvald** (`crates/storvald`): the AI-first dev environment. One
  program, `storvald`, that carries its system skills inside it
  (`crates/storvald/skills`).
- **TurboDot** (`crates/turbodot`): the app and game engine. It runs a gogo
  page in a window, and with no window for a test or an agent.
- **gogo** (`crates/gogo-ui`, `crates/gogo-text`, `crates/gogo-wgpu`): the
  declarative UI language. Published, and used by a shipping app.

This folder is how an agent works here for now. Once Storvald drives an
agent itself, these rules move into it as system skills.

## What every change is measured against (read every time)

1. **Concise explanations.** Most important first. Short. Plain words.
2. **Code that is easy to read, write and maintain.** A file read six
   months later still makes sense, to a person and to an AI.
3. **Ergonomics.** The easy way is the right way.
4. **Active voice.** Name who does what.

The rules, each loaded with this file:

- `rules/language.md`: replies, docs, comments, trackers, commit messages.
- `rules/code.md`: names, types, shape, comments, tests.
- `rules/ergonomics.md`: what ergonomic means here, and who decides.
- `rules/working.md`: what was asked, commits, proof, build and test.

The skills (`skills/`): `explain-comments` for every comment, `doc` for
docs and explanations, `make-todo` for trackers, `noise-free-copy` for
copy people read on a page, `wait-what` when the user says a message did
not land.

## Gogo UI language — main goals (read every time)

Every gogo language/design decision is measured against these:

- **Easy to understand, easy to write, easy to find. Ergonomic.** A `.gogo`
  file can also be read 6 months later and understood.
- **Rust-inspired design**: every construct keyword-announced, one keyword
  per concept, hidden semantics made visible, shorthand legal only as
  defined sugar.
- **Anything goes for UI and design changes** — scope is not the constraint.
  But run design changes by the user BEFORE implementing them.

- User should not have to ask you for your opinion explicitly. Always evaluate what the user is asking you to do, and voice your concerns before proceeding if you don’t think it's a good idea. If possible, propose a better solution, but you can voice concerns even without one. This applies even to direct requests to revert or simplify. Still evaluate whether your original approach was better. The user may be missing important context. If there was a solid reasoning you suggested that approach, push back with reasoning instead of silently complying.”

## Where things are

- gogo's docs, in `docs/`: `gogo-spec.md` (grammar and design rules),
  `REFERENCE.md` (lookup), `COOKBOOK.md` (recipes), `cards/` (one card
  per control, keyword and property), `gogo-errors.md` (one entry per
  diagnostic code). `cargo test` compiles every example in them.
- A fence word after the language marks an example held to another
  account: `partial` (grammar only), `error=G1234` (the compiler must
  refuse it with that code), `warning=G1234`.
- The tracker of Storvald and TurboDot: `docs/todo-ai-dev.md`. The
  user's notes on what Storvald is to be: `docs/TODO-main.md`.
- The first page: `crates/storvald/ui/skills_page.gogo`, with its code in
  `crates/storvald/src/skills_page.rs`. A debug build follows the file: save
  it and the open window shows the change.

## The system skills are in two places

`crates/storvald/skills/<name>/SKILL.md` is the one to edit: the program
carries those. `.claude/skills/<name>/SKILL.md` is the copy an agent reads
here. The test `the_agents_copies_match_the_bundled_skills` fails when
they differ. Copy the file over after an edit.
