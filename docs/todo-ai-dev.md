# AI dev environment — tools for building apps and games with an AI agent (2026-11-12), refreshed 2026-11-22

**Why this file exists.** The user plans a development environment for
apps and games built on the TurboDot engine, made for work in which an AI
agent writes most of the code and a person steers and reviews. This file
holds what is decided, what is open, and the pieces to build, so the plan
survives between sessions. It serves the rule that a design change is
agreed with the user before it is built: every piece the user has not
agreed to is marked PROPOSED and stays unbuilt until the user says so.

**How it was produced.** From the talk of 2026-11-12; the user's words are
quoted where a decision rests on them. "What exists today" was read the
same day from the gogo clone and from FanRust's `shell` and `imaging_wgpu`
crates. NOT done: no design of the steering app (the user has ideas and
brings them later), no code, no run of anything.

**Moved 2026-11-22.** This file came from FanRust's `docs` folder, where a
copy stays as it was on that day. Text written before the move says "the
gogo repo" for this repository, and names FanRust's files from FanRust's
`docs` folder. Read the same day for the refresh: this repository and
FanRust's rules for agent work (AD28). Run the same day: this repository's
whole suite and its packaged crates (AD24). NOT run: any Mac or Linux
computer, any phone.

**Sources of truth.**
- The gogo repository: https://github.com/nokola/storvald (named
  `nokola/gogo` until 2026-11-12; the old address forwards) — local
  clone `D:\repos\GitHub\storvald`.
- The move tracker: FanRust's `docs/todo-gogo-move.md` (GM27 the
  GitHub organization, GM28 the engine, GM31 the renderer's GPU tests).
- Decisions: FanRust's `docs/DECISIONS.md` D122 to D126 (gogo's own
  repository, the pin, page reload, the rename, Direct3D 12 first).
- FanRust's host: its `crates/shell` and
  `crates/imaging_wgpu/src/headless.rs`.
- The user's notes on what Storvald is to be: [TODO-main.md](TODO-main.md).
- The rules for agent work here: [../.claude/CLAUDE.md](../.claude/CLAUDE.md).

Statuses: <span style='color:red'>TODO</span> /
<span style='color:lightgreen'>DONE</span> /
<span style='color:deepskyblue'>DECIDED</span> /
<span style='color:orange'>LATER</span> (deliberate future work) /
<span style='color:gray'>CUT</span> (decided against).
Flip TODO→DONE keeping the original text; the resolution goes in an indented
sub-line.

## Words used here

- **gogo**: the UI language and its three crates (`gogo-ui` the language,
  `gogo-text` the text engine, `gogo-wgpu` the renderer) in the gogo
  repository.
- **The engine (TurboDot)**: the reusable part of a program that opens a
  window, brings up the GPU, hands touches and keys to gogo pages and draws
  them. Today FanRust's `shell` crate does this work, mixed with Fantasia's
  own services. Nothing named TurboDot exists yet.
- **Agent**: an AI program that edits a project's files and runs its
  commands (Claude Code today).
- **The dev environment (Storvald)**: everything in this file: the commands
  an agent runs, the app the user steers with, the project template. Named
  Storvald on 2026-11-12 (AD15); its command is `storvald` (AD5).
- **The steering app**: an app that, in the user's words, "helps steer
  along the AI". Its design is the user's and has not started.
- **PROPOSED**: suggested by Claude on 2026-11-12; the user has not agreed.
  The PROPOSED rows are a list to choose from, not a plan.

## What exists today

Read on 2026-11-12, so that nothing below is built twice. Paths without a
repository named are in the gogo repo.

- **Build errors with codes.** Every gogo compile error carries a code
  (`G2001` and so on). The codes are registered in
  `crates/gogo-ui/src/diag.rs` and explained in `docs/gogo-errors.md`; a
  test keeps the two matched.
- **Docs that compile.** The gogo examples in the docs are compiled by
  `crates/gogo-ui/tests/docs_examples.rs`, so an agent that copies an
  example copies one that builds.
- **Page reload.** `gogo_ui::Reloader` (`crates/gogo-ui/src/reload.rs`,
  cargo feature `hot-reload`) shows a saved `.gogo` file in the running
  app. It refuses a save that adds or removes a node
  (FanRust's `docs/todo-remaining.md` RM8).
- **Touches and keys as values.** `Input::touch`, `Input::key` and
  `Input::wheel` (`crates/gogo-ui/src/input.rs`) take one touch or key and
  return the events it caused. No window and no operating system is
  involved; gogo's own tests drive pages this way (21 test files).
- **A small host.** `examples/window` opens a window and shows a page.
- **A GPU device with no window.** FanRust's
  `crates/imaging_wgpu/src/headless.rs`. It is in FanRust, not in the gogo
  repo, and for that reason the 11 GPU tests of gogo's renderer still live
  beside it (GM31).
- **Editor support.** `vscode/`, published as `nikolamsft.gogo-language`.
- **Rules for agent work.** This project's `CLAUDE.md`, its skills
  (`.claude/skills`) and the tracker shape of this file. They live in the
  Fantasia repository and are written for it.
- **Not there:** a call that prints a page's nodes with their rectangles
  as text. A search of the gogo crates for `dump`, `print_tree`,
  `tree_text`, `describe_tree` and `outline` found none.

## What the user wants the three parts to be

The user's words of 2026-11-12, for the README and for marketing. They are
goals: nothing of TurboDot or Storvald is built.

- **Storvald**: "Capable AI-first creative environment for unslop games and
  apps". In the user's chart: "AI-first IDE + agent environment".
- **TurboDot**: "App/game engine powered by 80s simplicity. Build for
  agents." and "the engine AI can prove that its changes work. headless,
  deterministic, high-performance, AI-native development." In the chart:
  "App / game engine", "Runtime, execution, rendering".
- **Gogo UI**: "declarative UI with immediate mode performance". In the
  chart: "Declarative UI", "Layouts, controls, interactions".
- **What the engine gives an agent** (a text the user pasted the same day;
  it ends: "being headless doesn't automatically deliver these capabilities.
  They're the APIs, invariants, and tooling that could make TurboDot
  genuinely AI-first"):
  - a structured project model: an agent changes typed entities, resources
    and properties, and does not click through inspector panels;
  - deterministic execution: a game state is replayed and a bug shows again;
  - headless testing: thousands of simulation frames with no editor open;
  - inspectable execution: why a collision happened, which system used the
    frame time, why an entity changed state;
  - performance budgets: a regression is caught before it shows in play.
- Which rows below serve these: AD7 and AD9 (no window), AD11 (a clock the
  script sets, so a run can be played again), AD10 (a page's nodes as text).
  The project model, the answers to "why" and the budgets have no row yet:
  they wait for the engine's design (AD14). The rows stay PROPOSED until the
  user agrees to each.

## Phase 1 — where it lives and whose it is

| id | scope | status |
|---|---|---|
| AD1 | All of it lives in `nokola/gogo` for now | <span style='color:deepskyblue'>DECIDED</span> 2026-11-12 (the user); the same day: the repository is renamed `nokola/storvald`, done |
| AD2 | The engine is not tied to Artswept for now | <span style='color:deepskyblue'>DECIDED</span> 2026-11-12 (the user) |
| AD3 | The dev environment is public for now | <span style='color:deepskyblue'>DECIDED</span> 2026-11-12 (the user); one caveat in the entry |
| AD4 | USER CALL: is the dev environment a product to sell | <span style='color:orange'>LATER</span> |
| AD5 | USER CALL: the name of the command-line tool | <span style='color:deepskyblue'>DECIDED</span> 2026-11-12 (the user): `storvald` |
| AD15 | The dev environment is named Storvald | <span style='color:deepskyblue'>DECIDED</span> 2026-11-12 (the user) |
| AD16 | USER: hold the name Storvald (crate, domains, GitHub account) | <span style='color:lightgreen'>DONE</span> 2026-11-12: the GitHub name, storvald.com, storvald.dev, and `storvald` 0.5.0 on crates.io |
| AD20 | USER: hold the name TurboDot on crates.io | <span style='color:lightgreen'>DONE</span> 2026-11-12: `turbodot` 0.5.0 on crates.io |

- <span style='color:deepskyblue'>DECIDED</span> AD1: the gogo crates, the
  engine and the dev environment live in one repository, `nokola/gogo`, for
  now. No rename, no second repository, no GitHub organization.
  - DECIDED 2026-11-12 (user: "maybe we put all in nokola/gogo for now and
    separate them later, to avoid complexity"). FanRust's three pin lines
    keep their address. GM27 (move to a GitHub organization) stays LATER.
  - What it buys: a change that crosses the language, the engine and a tool
    is one commit and one test run, and an agent works in one folder.
  - What keeps a later split cheap: the engine and the tools name the gogo
    crates, never the other way round. Cargo refuses a circle of crates, so
    this holds by itself from the first engine crate on.
  - PROPOSED: each new part is its own crate in the workspace, with
    `publish = false` (as `examples/window` has) until the user wants it on
    crates.io.
  - CHANGED 2026-11-12, later the same day (user: "let's rename nokola/gogo
    to nokola/storvald"): the one repository is renamed `nokola/storvald`,
    still under the user's own account. Claude had advised to wait and to do
    the rename and a move to the `storvald` organization in one step. The
    folder of the clone on this PC stays `D:\repos\GitHub\gogo`.
  - In progress: the three files of the clone that name the address are
    edited and not committed (`Cargo.toml`, `vscode/README.md`,
    `vscode/package.json`; cargo reads the manifest, the extension's
    manifest parses, no test reads the address). Remaining: the rename on
    GitHub and the clone's commit (the user's); then in FanRust the three
    pin lines, the key of the override file and of its example (the key is
    the address, so it changes with the pin lines), `CLAUDE.md`, the two doc
    pointers, and the pin check.
  - DONE 2026-11-12 (DECISIONS.md D125): renamed on GitHub by the user, and
    the clone's three files committed and pushed as gogo `fc9a440`.
    FanRust's three pin lines name the new address and that commit; the key
    of the override file and of its example, `CLAUDE.md` and the two doc
    pointers name the new address. The pin check passed both ways
    (`build\gogo_pin_move16.log`): with the local clone through the new key,
    then with the override set aside and the three crates fetched from
    `nokola/storvald` at `fc9a440` (`cargo check --workspace --all-targets`
    exit 0); `Cargo.lock` is unchanged. GitHub's run 43281537584 for
    `fc9a440` is green on all four jobs (packaged crates, Linux, macOS,
    Windows). The old address forwards (checked: it answers 301 with the new
    one).
  - The folder followed, 2026-11-12 (user: "I updated local path to storvald
    as well"): the clone on this PC is `D:\repos\GitHub\storvald`. Until the
    three paths of FanRust's override file named it, a FanRust build would
    not have found the gogo crates. Changed: the override file and its
    example, `CLAUDE.md`, the two doc pointers, D125, this file and the two
    other live trackers; `cargo metadata` lists the three crates at the new
    folder.
- <span style='color:deepskyblue'>DECIDED</span> AD2: the engine is not tied
  to Artswept for now. The repository stays under the user's own account.
  - DECIDED 2026-11-12 (user: "engine is not tied to artswept for now").
  - For a later day, checked 2026-11-09: the GitHub name `turbodot` belongs
    to a user account unused since 2016; `turbodot-engine`, `turbodot-rs`
    and `turbodot-dev` were free. Nothing is registered.
- <span style='color:deepskyblue'>DECIDED</span> AD3: the dev environment is
  developed in the public repository for now.
  - DECIDED 2026-11-12 (user: "dev environment private or public -
    undecided yet, may or may not be product to sell. probably public for
    now").
  - Caveat (Claude, told to the user 2026-11-12): code pushed to the public
    repository cannot be made private again, and the workspace's license
    (`MIT OR Apache-2.0`) lets anyone keep using what was pushed. A later
    decision to sell (AD4) can close only the code written after it. A part
    that might be sold can wait in a private repository until AD4 is
    answered.
- <span style='color:orange'>LATER</span> AD4: USER CALL: is the dev
  environment a product to sell. Open by the user's choice ("may or may not
  be product to sell"). The answer decides whether the steering app (AD6)
  stays in the public repository.
- <span style='color:deepskyblue'>DECIDED</span> AD5: USER CALL: the name of the
  command-line tool. The commands of phase 3 need one program name. `giddy`
  was the name while the engine was called Scamp, and went with that name.
  Not `turbo` (the command of Vercel's Turborepo) and not `dot` (the command
  of Graphviz): both are on many developers' PCs already.
  - DECIDED 2026-11-12 (user: "just storvald for cli is good"). The command
    is `storvald`, the dev environment's own name (AD15), so there is no
    second name to learn.
  - Set aside the same day: `stor`, the user's first idea. The PyPI package
    `stor` and a Go tool for Azure storage each install a command of exactly
    that name, every earlier use of the word means "storage", and spoken it
    is "store", which a tool that builds apps for app stores would trip
    over. Also set aside, from a list by Claude that was not checked:
    `svald`, `vald`, `steer`, `jarl`, `tiller`, `tdot`, `turbodot`.
- <span style='color:deepskyblue'>DECIDED</span> AD15: the dev environment
  is named Storvald. The three names are then Storvald (the dev environment:
  the command, the steering app, the template), TurboDot (the engine) and
  gogo (the UI language).
  - DECIDED 2026-11-12 (user: "for the IDE, I'm thinking naming it Storvald,
    `stor` as cli command, then we have TurboDot engine and Gogo UI", and
    later "just storvald for cli is good"). Claude reads the second message
    as settling the name as well as the command; the user was told so.
  - Checked 2026-11-12: `storvald` is free on crates.io, npm and PyPI;
    GitHub has no account of that name; storvald.com, .net, .dev, .app, .io
    and .org are not registered; the US trademark search finds no mark with
    the word. The first web hit is Jarl Storvald, a frost giant ruler in the
    Dungeons & Dragons book "Storm King's Thunder": a character's name, no
    software. NOT checked: trademarks outside the US, the app stores, Steam.
  - Catches told to the user: by ear the name can be written Storvold,
    Storwald, Thorvald or Torvald, and it is one letter from Torvalds.
  - PROPOSED: Storvald is the one name a newcomer installs and hears first;
    TurboDot and gogo are named as its parts.
  - CONFIRMED 2026-11-12 (user: "sounds good, storvald stays"), after two
    more names were weighed the same day. Starvald: in lowercase it reads
    "starv-ald", the Star- names are many in games, and starvald.com and the
    GitHub name are taken. Vald: a registered US trademark in the software
    class (Vald Pty Ltd), and an open-source vector search engine of that
    name.
- <span style='color:lightgreen'>DONE</span> AD16: USER: hold the name Storvald:
  the crate name `storvald` on crates.io, a domain, the GitHub account name.
  All were free on 2026-11-12 (AD15), nothing is registered, and a free name
  can be taken by someone else on any day. A crate name is held by
  publishing a crate, and a published version can never be removed, only
  yanked; no crate is prepared.
  - Landed 2026-11-12 (user: "I own it now"): the GitHub name `storvald` is
    the user's. Remaining: the crate name on crates.io, a domain.
  - Landed 2026-11-12 (user: "I own storvald.com and storvald.dev now"): the
    two domains are the user's.
  - Landed 2026-11-12 in the clone, not committed (user: "let's do crates.io
    storvald"): `crates/storvald`, a crate of the workspace that builds the
    `storvald` command. This version runs nothing. A bare call, `--help` and
    `--version` say that Storvald is in development and succeed; any other
    call fails with code 2 and names what was asked, so a script or an agent
    that calls a command that does not exist yet does not take it for a
    pass. Its version is the workspace's (0.5.0), because
    `scripts/test-packages.sh` reads one version for every crate under
    `crates/`. Its page on crates.io is the repository's README, which opens
    with Storvald.
  - Seen failing: with a command that always succeeds,
    `a_command_that_does_not_exist_yet_fails` reports code 0 where 2 is
    asked. Tests: `crates/storvald/tests/command.rs`, two, both run the
    built program. Proven: the clone's whole suite 90 suites, 1266 passed, 0
    failed, 61 ignored (scratchpad `storvald_crate_clone_test1.log`; 88 and
    1264 before); the packaged crates 83 suites, 1233 passed, 0 failed, 53
    ignored, the package of `storvald` among the four
    (`storvald_crate_packages1.log`; 81 and 1231 before); `cargo publish
    --dry-run -p storvald`: 9 files, built from the package. The name was
    free on crates.io the same hour.
  - The rule of crates.io, told to the user: it forbids content that "exists
    only to reserve a name for a prolonged period of time" "without having
    any genuine functionality, purpose, or significant development activity
    on the corresponding repository"; the team decides case by case and in
    most cases asks the author first. A command that says only "in
    development" leans on the repository's activity; a first command that
    does work (AD8 is the nearest) takes the question away.
  - DONE 2026-11-12: the user committed and pushed the crate (gogo
    `572be1b`) and published it: crates.io lists `storvald` 0.5.0, published
    2026-11-12 20:10 UTC, with the repository's address. The name is held in
    all three places. FanRust's three pin lines name `572be1b`. The pin
    check passed both ways (`build\gogo_pin_move19.log`): with the local
    clone, then with the override set aside and the three crates fetched
    from GitHub (`cargo check --workspace --all-targets` exit 0);
    `Cargo.lock` is unchanged. GitHub's run 43297582531 for `572be1b` is
    green on all four jobs (packaged crates, Linux, macOS, Windows).
- <span style='color:lightgreen'>DONE</span> AD20: USER: hold the name TurboDot on
  crates.io (user, 2026-11-12: "lets also reserve turbodot"). The name was
  free that hour. The GitHub name `turbodot` belongs to an account unused
  since 2016 (AD2), and no TurboDot domain is held.
  - Landed 2026-11-12 in the clone, not committed: `crates/turbodot`, a
    library crate of the workspace with no API; its text says TurboDot is in
    development. Version 0.5.0, the workspace's. Its page on crates.io is
    the repository's README, which has a TurboDot part.
  - Proven: the clone's whole suite 92 suites, 1266 passed, 0 failed, 61
    ignored (scratchpad `turbodot_crate_clone_test1.log`; the two new suites
    hold no test); the packaged crates 85 suites, 1233 passed, 0 failed, 53
    ignored, five packages (`turbodot_crate_packages1.log`); `cargo publish
    --dry-run -p turbodot`: 8 files, built from the package.
  - Told to the user: a library with no API is nearer to what crates.io
    forbids than the `storvald` command is (the rule is quoted under AD16);
    the repository's activity is the argument for it, and the first real
    piece of the engine (AD7) takes the question away. Remaining: the gogo
    commit, `cargo publish -p turbodot` and the pin move (the user's).
  - DECIDED 2026-11-12 (user: "I don't care about the github name and
    site"): TurboDot gets no GitHub name and no site of its own. This row is
    the crate name alone.
  - DONE 2026-11-12: the user committed and pushed the crate (gogo
    `be8c100`) and published it: crates.io lists `turbodot` 0.5.0, published
    2026-11-12 20:33 UTC. GitHub's run 43301516069 for `be8c100` is green on
    all four jobs (packaged crates, Linux, macOS, Windows). FanRust's three
    pin lines name `be8c100`. The pin check passed both ways
    (`build\gogo_pin_move20.log`): with the local clone, then with the
    override set aside and the three crates fetched from GitHub (`cargo
    check --workspace --all-targets` exit 0); `Cargo.lock` is unchanged.

## Phase 2 — the steering app

| id | scope | status |
|---|---|---|
| AD6 | USER: the app that helps the user steer the agent | <span style='color:red'>TODO</span> the first screen landed 2026-11-22 (AD24); remaining: the rest of the user's notes |

- <span style='color:red'>TODO</span> AD6: USER: the app that helps the user
  steer the agent. User, 2026-11-12: "I'd like an app that helps steer along
  the AI, have ideas and will discuss them later", and of the whole
  environment: "specifically for games/apps that is built on turbodot and
  focused on AI workflows". Nothing is designed, and nothing is to be
  designed before the user brings the ideas.
  - It is an app built on the engine, so it needs AD14.
  - The ideas, 2026-11-21: the user's notes are in
    [TODO-main.md](TODO-main.md). User, the same day: "first screen would
    be bootstrap something system skills list on the left", and "Once we
    have the first Storvald, I start it, open this folder, then it drives
    the claude agent with its system skills and development moves there.
    lots of undecided stuff yet. just thinking".
  - Landed 2026-11-22, not committed: the first screen (AD24). Remaining:
    everything else in the notes. The design is the user's, and none of it
    is designed.

## Phase 3 — what an agent needs to check its own work

All rows PROPOSED. Each is for a page on its own unless it says otherwise;
a whole app with its own code needs the engine (AD14).

| id | scope | status |
|---|---|---|
| AD7 | A GPU device with no window, and its pixels read back, in the gogo repo | <span style='color:red'>TODO</span> `turbodot::Gpu` landed 2026-11-22; remaining: the GPU tests in FanRust (GM31) |
| AD8 | A command that compiles every page of a project and lists the errors | <span style='color:red'>TODO</span> PROPOSED |
| AD9 | A command that draws one page to a picture file, with no window | <span style='color:red'>TODO</span> landed 2026-11-22 as a call (`Gpu::draw`); remaining: the command |
| AD10 | A command that prints a page's nodes with their rectangles | <span style='color:red'>TODO</span> landed 2026-11-22 as a call (`Ui::outline`); remaining: the command |
| AD11 | Touches and keys from a script, on a clock the script sets | <span style='color:red'>TODO</span> PROPOSED |

- <span style='color:red'>TODO</span> AD7: a GPU device with no window, and a
  way to read its pixels back, in the gogo repo. AD9 needs it, and it is
  what GM31 waits for: with it the 11 GPU tests of gogo's renderer can move
  out of FanRust. FanRust's `headless.rs` is the model; it cannot move as it
  is, because it carries FanRust's canvas format check.
  Check: one GPU test of the renderer runs inside the gogo repo.
  - Landed 2026-11-22, not committed (user, 2026-11-21: "do 1, 3, 4"):
    `turbodot::Gpu` (`crates/turbodot/src/gpu.rs`), a device with no
    window, and `Gpu::draw`, which draws a page with gogo's own renderer
    and hands its pixels back as a `Picture`. It carries nothing of
    FanRust's `headless.rs`: no canvas format check and no device options.
  - The check as written was met before: `crates/gogo-wgpu/tests/draws_a_page.rs`
    holds three GPU tests of the renderer in this repository. Remaining:
    the GPU tests of the renderer that live in FanRust (GM31). Nobody
    looked at them on 2026-11-22.
- <span style='color:red'>TODO</span> AD8: a command that compiles every
  `.gogo` page of a project and lists the errors, one per line, with file,
  line and code. Today the errors come out of the project's `cargo build`
  (its build script compiles the pages), after the wait for the Rust
  compiler. A command of its own answers in the time gogo takes.
  Check: a page with a known error is listed with its code; a clean project
  lists nothing and exits 0.
- <span style='color:red'>TODO</span> AD9: a command that draws one page to a
  picture file with no window, at a size, a theme and a language given on
  the command line. It lets an agent look at what it made without touching
  the user's desktop: this project's rules forbid photographing the user's
  screen and sending mouse or key input to it. Needs AD7.
  Check: the picture of a known page matches a kept one.
  - Landed 2026-11-22 as a call, not a command: `Gpu::draw` and
    `Picture::save_png`. The test `the_screen_is_drawn_as_the_page_says`
    writes `target/pictures/skills_screen.png`. Remaining: the command,
    with a size, a theme and a language, and a kept picture to compare
    with.
- <span style='color:red'>TODO</span> AD10: a command that prints a page's
  nodes, each with its name, kind and rectangle after layout, as text. It
  lets an agent check a layout by numbers ("the button is 48 high and
  inside its row"), where a picture has to be judged by eye. No such call
  exists today.
  Check: the printed rectangle of a named node equals the one a test reads
  from the runtime.
  - Landed 2026-11-22 as a call, not a command: `Ui::outline`
    (`crates/turbodot/src/ui.rs`). The test
    `the_outline_gives_each_node_its_kind_its_name_and_its_rectangle`
    compares a printed rectangle with the one read from the same frame.
    Remaining: the command.
- <span style='color:red'>TODO</span> AD11: touches and keys from a script,
  on a clock the script sets: touch at 120,300 at 0.0 s, lift at 0.1 s,
  draw at 0.5 s. gogo already takes them as values (`Input::touch`,
  `Input::key`); missing are a file format, a program that plays it, and
  time that moves only when the script says so.
  Check: the same script played twice gives the same pictures.
  - A part is there since 2026-11-22: `Ui::touch` takes its time from the
    caller and `Ui::frame` takes the seconds since the last frame, so a
    test moves the clock itself. Remaining: the file format, the program
    that plays it, and the check.

## Phase 4 — what the user needs to review a change

All rows PROPOSED.

| id | scope | status |
|---|---|---|
| AD12 | Before and after pictures of every page a change touched | <span style='color:red'>TODO</span> PROPOSED |
| AD13 | A project template that carries the agent's instructions | <span style='color:red'>TODO</span> PROPOSED |

- <span style='color:red'>TODO</span> AD12: for one change, each page it
  touched, before and after, side by side. Needed because in this project's
  rounds the writing is fast, and the proving and the reviewing take the
  time. It may turn out to be a part of the steering app (AD6); the user
  decides that there. Needs AD9.
- <span style='color:red'>TODO</span> AD13: a starting project for a new app
  or game that comes with the instructions an agent needs: how to build,
  test and look at a page, and the rules for comments, docs and trackers.
  This project's `CLAUDE.md` and skills are the raw material. They name
  Fantasia throughout, so they are to be written again, not copied.
  - Raw material, 2026-11-22: this repository's `.claude` folder (AD28)
    holds the rules with Fantasia's names taken out. Remaining: the
    template.

## Phase 5 — the engine

| id | scope | status |
|---|---|---|
| AD14 | The engine comes out of FanRust's `shell` into the gogo repo | <span style='color:red'>TODO</span> the first piece landed 2026-11-22 (`crates/turbodot`); remaining in the entry |
| AD31 | FanRust's host code moves into TurboDot, piece by piece, and FanRust builds on TurboDot | <span style='color:red'>TODO</span> moved 2026-11-22 to FanRust's `docs/todo-storvald.md`, SV12 |
| AD32 | gogo allocates in three places while a page moves | <span style='color:red'>TODO</span> moved 2026-11-22 to FanRust's `docs/todo-storvald.md`, SV14 |
| AD33 | USER CALL: `Ui` and `pages::HostedPage` both run a page | <span style='color:red'>TODO</span> moved 2026-11-22 to FanRust's `docs/todo-storvald.md`, SV1, SV2 |
| AD34 | USER CALL: an app draws under its pages | <span style='color:red'>TODO</span> moved 2026-11-22 to FanRust's `docs/todo-storvald.md`, SV5 |
| AD35 | A TurboDot app's pages are compiled when the program is built | <span style='color:red'>TODO</span> moved 2026-11-22 to FanRust's `docs/todo-storvald.md`, SV8 |

- <span style='color:red'>TODO</span> AD14: the engine comes out of FanRust's
  `shell` crate into the gogo repo (AD1). This continues GM28, which said
  "when the first game needs it, not before". The dev environment is a
  second reason: the user described it as "built on turbodot", so the
  steering app (AD6) needs an engine that exists outside Fantasia.
  - Open: what comes first. PROPOSED: phase 3 first. AD7 to AD11 need the
    gogo crates and a GPU device, not the engine, and AD7 is the engine's
    first piece anyway. The window loop, the Android part and the input of
    `shell` follow when the steering app or a game needs them.
  - Nothing of Fantasia may come along (the gogo repo's rule). `shell` mixes
    the host work with Fantasia's services; the services stay in FanRust.
  - Landed 2026-11-22, not committed (user, 2026-11-21: "do 1, 3, 4"): the
    first piece, `crates/turbodot`. `Ui` runs one gogo page with no window.
    `Gpu` draws it to a picture (AD7). `run` shows it in a window with an
    `App`: what the app writes into the page, and what the page's events
    mean to it. The same app code runs in a window and in a test. On
    Windows the window starts on Direct3D 12 alone, then on every backend
    (the rule of FanRust's DECISIONS.md D126).
  - It came from `examples/window`, not from `shell`. `examples/window`
    stays as it is: gogo's example of a host written by hand.
  - Remaining: Android, more than one page, the clipboard, the project
    model, replayed runs, the answers to "why" and the budgets (the goals
    above), and whatever of `shell` a game needs.

- <span style='color:red'>TODO</span> AD31: FanRust's host code moves into TurboDot, piece by piece,
  and FanRust builds on TurboDot (user, 2026-11-22: "fix the concerns, then
  move all").
  - MOVED 2026-11-22 (user: "yes"): the plan, the list of pieces and what
    landed are in FanRust's `docs/todo-storvald.md`, row SV12. The text that
    stood here is in this file's history (commit `ebb85e8`).
- <span style='color:red'>TODO</span> AD32: gogo allocates in three places while a page moves.
  - MOVED 2026-11-22: FanRust's `docs/todo-storvald.md`, row SV14.
  - Landed 2026-11-22: two of the three are gone. Laying a box out no
    longer lists its children (`Lay::visible_children` is a walk), and
    the hit test reads the paint order the placed tree keeps
    (`PlacedTree::paint_order`, written once per `build`).
    Measured 2026-11-22 on the user's PC, a release build, a page of about 600 nodes: a mouse move 7.2 to 3.2 microseconds; a step of a pan (touch and frame) 365 to about 345, inside the run-to-run spread. A mouse move and a pan frame now allocate nothing (`crates/turbodot/tests/allocations.rs`).
    Remaining: a touch that causes an event returns a new list.
- <span style='color:red'>TODO</span> AD33: USER CALL: `Ui` and `pages::HostedPage` both run a
  page.
  - MOVED 2026-11-22: FanRust's `docs/todo-storvald.md`, rows SV1 and SV2. Decided
    there: TurboDot owns navigation, and a client writes pages.
  - Landed 2026-11-22 (rows SV10 and SV11 there): `turbodot::Page`,
    `CachedPage`, `Host`, `Nav`, `Program`, `start`. The `storvald`
    command runs on them, with a second page. `Ui`, `App`, `run` and
    `Gpu::draw` are gone (user, 2026-11-22: "remove if not called");
    `Gpu::draw_program` draws a program's pages to a picture.
  - 2026-11-22 (user's calls, recorded there under SV10):
    `pages::HostedPage` is now `pages::PageUi`; `start!` is gone, a
    client writes `main` and calls `turbodot::start`; `show` is
    optional.
- <span style='color:red'>TODO</span> AD34: USER CALL: an app draws under its pages.
  - MOVED 2026-11-22: FanRust's `docs/todo-storvald.md`, row SV5.
- <span style='color:red'>TODO</span> AD35: a TurboDot app's pages are compiled when the program
  is built (`gogo_ui::build_pages`).
  - MOVED 2026-11-22: FanRust's `docs/todo-storvald.md`, row SV8. Decided there:
    yes, by one line in `build.rs`.
  - Landed 2026-11-22: `turbodot::build::pages("ui")`, used by the
    `storvald` command and by `examples/notes`.

## Phase 6 — what a visitor reads

| id | scope | status |
|---|---|---|
| AD17 | The repository's README opens with a Storvald section and the chart | <span style='color:lightgreen'>DONE</span> 2026-11-12: gogo `415876a`, the section in the user's own words |
| AD18 | USER CALL: what the three crates show on crates.io | <span style='color:red'>TODO</span> |
| AD19 | Slips in the README's Storvald part: the name is written "Stormvald" twice | <span style='color:lightgreen'>DONE</span> 2026-11-12: the six fixes alone, gogo `62555da` |
| AD21 | One site for all three parts: storvald.dev, with storvald.com sending visitors on | <span style='color:deepskyblue'>DECIDED</span> 2026-11-12 (the user); which of the two is the main one is open |

- <span style='color:lightgreen'>DONE</span> AD17: the README of the repository
  opens with a Storvald section; the gogo section stays (user, 2026-11-12:
  "update the readme to keep the gogo section, but add a storvald one
  upfront", with a chart "to re-make").
  - Landed 2026-11-12 in the clone, not committed. In front of the README:
    the name and the user's line, the chart
    (`docs/img/storvald_overview.svg`, drawn again from the user's picture,
    with its own light and dark colors), a table of the three parts with
    what each is and where it stands today, and "What TurboDot is built for"
    (the five goals above, said as goals). The gogo section is byte for byte
    as it was.
  - Two things differ from what the user sent, both told to the user: the
    chart says `storvald .` and `storvald go` where the picture said `stor
    .` and `stor go` (AD5), and "Build for agents" is written "Built for
    agents". The table says "In development" for Storvald and TurboDot, and
    one sentence under it says nothing of them is released: a README that
    reads as if they existed would send a visitor looking for them.
  - Proven 2026-11-12: the clone's whole suite, which compiles the README's
    examples: 88 suites, 1264 passed, 0 failed, 61 ignored (scratchpad
    `readme_clone_test2.log`), the counts of gogo `151a230`. The chart was
    drawn to a picture by a browser with no window, in its light and its
    dark colors, and looked at. A first run had 26 failures, every one
    "cannot find the path": the test programs had been built before the
    clone's folder was renamed and held the old folder; the build output of
    the clone's four packages was removed and the run repeated.
  - DONE 2026-11-12: the user wrote the section again in their own words and
    pushed it with the chart as gogo `415876a` ("I updated the readme and
    pushed"). As published: the name and its line, the chart, a table of the
    three parts (no column for where each stands; the sentence under it says
    TurboDot and Storvald are in development), a part "Storvald", a part
    "TurboDot" with the five goals, then the gogo section. In the chart the
    user wrote "AI-FIRST IDE AND AGENT ENVIRONMENT" and made the small
    captions black; the commands stay `storvald .` and `storvald go`.
    FanRust's three pin lines name `415876a`. The pin check passed both ways
    (`build\gogo_pin_move17.log`): with the local clone in its renamed
    folder, then with the override set aside and the three crates fetched
    from GitHub (`cargo check --workspace --all-targets` exit 0);
    `Cargo.lock` is unchanged. GitHub's run 43288656748 for `415876a` is
    green on all four jobs (packaged crates, Linux, macOS, Windows), and it
    compiled the README as the user wrote it.
- <span style='color:red'>TODO</span> AD18: USER CALL: what the three crates
  show on crates.io. `gogo-ui`, `gogo-text` and `gogo-wgpu` each use the
  repository's README as their page (`readme = "../../README.md"`). From the
  next publish on, someone who opens the `gogo-ui` page to learn about a UI
  crate reads about Storvald first. The choices: leave it; or give the
  crates a README of their own that holds the gogo part, at the price of the
  same text in two files. Nothing changes on crates.io before the user
  publishes.
- <span style='color:lightgreen'>DONE</span> AD19: slips in the README's Storvald
  part, as published in gogo `415876a` (read 2026-11-12 and told to the
  user). The name is written "Stormvald" in two places (lines 19 and 23).
  "reduce de-duplication (less tokens)" says the opposite of what is meant,
  which is less duplication. "quiality". "Bug are easier to reproduce and
  deterministic". "Ready on crates.io: ... on crates.io" names crates.io
  twice. Two headings are `# Storvald` (lines 1 and 16). USER SEES: a
  visitor reads the product's name spelled two ways on its first page. The
  text is the user's own, so nothing is changed before the user says so.
  - Landed 2026-11-12 in the clone, not committed (user: "improve"). The
    part in front of the gogo section is written again, keeping what the
    user said: the six slips are gone; one `# Storvald` heading, with
    "Storvald is AI-first" and "TurboDot makes the right route the easy one"
    under it; the line that repeated the headline is dropped; the paragraph
    on traditional game engines moved to TurboDot, where it is the reason
    for "the right route"; the table's Storvald row says what the chart says
    and adds "where you steer an AI agent and check its work" (Claude's
    words, told to the user). The gogo section is byte for byte as it was
    (151 lines). `cargo test -p gogo-ui --test docs_examples`, the test that
    compiles the README's examples: 4 passed, 0 failed. Remaining: the gogo
    commit and the pin move (the user's).
  - CORRECTED 2026-11-12 (user: "I reverted the commit. when I said
    'improve' I meant fix only these", with the six slips): the rewrite
    above went too far. The user had committed and pushed it (gogo
    `fdc331c`) and reverted it in the clone (`2402638`, not pushed when this
    was written). Landed in its place, not committed: the six fixes and
    nothing else. "Stormvald" is "Storvald" in both places; "reduce
    duplication (less tokens)"; "creates quality code"; "Bugs are easier to
    reproduce and deterministic."; the second "on crates.io" is taken out of
    the table row; the second `# Storvald` heading line is removed, so its
    paragraphs follow the opening. `cargo test -p gogo-ui --test
    docs_examples`: 4 passed, 0 failed. Remaining: the gogo commit and the
    pin move (the user's).
  - DONE 2026-11-12: committed and pushed by the user, in three commits
    after the revert that end at gogo `62555da`; they also carry the user's
    own change to the opening line, which now reads "AI-first creative
    environment." The six fixes are in the published text. FanRust's three
    pin lines name `62555da`. The pin check passed both ways
    (`build\gogo_pin_move18.log`): with the local clone, then with the
    override set aside and the three crates fetched from GitHub (`cargo
    check --workspace --all-targets` exit 0); `Cargo.lock` is unchanged.
    GitHub's run 43290942150 for `62555da` is green on all four jobs
    (packaged crates, Linux, macOS, Windows).
- <span style='color:deepskyblue'>DECIDED</span> AD21: one site for all
  three parts (user, 2026-11-12: "storvald.dev will be the main site with
  storvald.com redirect to it or vice versa"). TurboDot and gogo get no site
  of their own. Open by the user's choice: which of the two addresses is the
  main one. No site exists yet; when one does, the crates' manifests can
  name it as their homepage.

## Phase 7 — the first Storvald

| id | scope | status |
|---|---|---|
| AD22 | Storvald is one program that carries its system skills | <span style='color:deepskyblue'>DECIDED</span> 2026-11-21 (the user) |
| AD23 | This repository is set up for agent work until Storvald drives the agent | <span style='color:deepskyblue'>DECIDED</span> 2026-11-21 (the user) |
| AD24 | The first screen: the system skills on the left, the picked one on the right | <span style='color:red'>TODO</span> landed 2026-11-22; remaining: the commit (the user's) |
| AD25 | `cr-code-review` as a system skill | <span style='color:orange'>LATER</span> |
| AD26 | The carried skills can be updated | <span style='color:red'>TODO</span> not designed |
| AD27 | Storvald opens a folder and drives the Claude agent with its system skills | <span style='color:red'>TODO</span> not designed |
| AD28 | Rules for agent work in this repository's `.claude` folder | <span style='color:red'>TODO</span> landed 2026-11-22; remaining: the commit (the user's) |
| AD29 | What the first screen does not do yet | <span style='color:red'>TODO</span> |
| AD30 | A double click on `storvald.exe` opens no console window | <span style='color:red'>TODO</span> landed 2026-11-22; remaining: the commit (the user's) |

- <span style='color:deepskyblue'>DECIDED</span> AD22: Storvald is one program, `storvald.exe`, that
  carries its system skills inside it. The skills can be updated later
  (AD26).
  - DECIDED 2026-11-21 (user: "I want a single storvald.exe taht comes with
    pre-bundled skills that are later update-able", and "Goal is for
    Storvald to be the dev environment as soon as possible, and it's
    powerful and efficient system skills to just work").
  - How it is built: `crates/storvald/src/skills.rs` reads each
    `crates/storvald/skills/<name>/SKILL.md` when the program is built.
    The page and the font are carried the same way, so the program runs
    from any folder with nothing beside it.
- <span style='color:deepskyblue'>DECIDED</span> AD23: this repository is set up for agent work for now.
  Once the first Storvald runs, the user starts it and opens this folder,
  it drives the Claude agent with its system skills, and the development
  moves there.
  - DECIDED 2026-11-21 (user: "let's temporarily make this repo a place the
    agent can work in. Once we have the first Storvald, I start it, open
    this folder, then it drives the claude agent with its system skills
    and development moves there").
- <span style='color:red'>TODO</span> AD24: the first screen. A bare `storvald` opens a window:
  the system skills in a list on the left, the picked skill's name,
  description and text on the right.
  - Landed 2026-11-22, not committed (user, 2026-11-21: "do 1, 3, 4"). The
    page is `crates/storvald/ui/skills.gogo`, the code
    `crates/storvald/src/screen.rs`, on TurboDot (AD14). The skills are
    `doc`, `explain-comments` and `make-todo`, taken from the Fantasia
    repository. Three places changed in them: the comment example is Rust
    now, the part about the Godot addon's docs is replaced by a short
    general one, and "shipped" refs are called provenance refs.
  - `--help` and `--version` say what this version is. Any other word
    fails with code 2, as before. A debug build follows the page's file:
    a save shows in the open window.
  - Seen failing, each by breaking the code and putting it back:
    `the_agents_copies_match_the_bundled_skills` (one more byte in a
    copy), `a_click_on_a_row_shows_that_skill` (a pick that does not
    write the skill), `the_wheel_scrolls_the_text_under_the_mouse` (a
    wheel that moves nothing).
  - Proven 2026-11-22 on the user's PC, Vulkan: the whole suite
    97 suites, 1288 passed, 0 failed, 63 ignored (`target/logs/workspace_test.log`;
    92 suites, 1266 passed, 0 failed, 61 ignored at `be8c100`). The
    packaged crates: 88 suites, 1248 passed, 0 failed, 54 ignored
    (`target/logs/packages_test.log`). The GPU tests of `turbodot` and
    `storvald`, asked for by hand: 2 passed. The built program, started
    once: its window came up, drew the screen, showed a saved change of
    the page's file, and closed with code 0.
  - Found by use, 2026-11-22 (user: "clicking on explain-comments
    crashes"). USER SEES: a click on the second row closed the window.
    gogo's text engine measured a text of several lines wrongly
    (`TextEngine::narrowest_wrap`, `crates/gogo-text/src/lib.rs`): it read
    each letter's byte positions against the whole text, and they count
    from the start of the letter's own line. In that skill's text one
    position landed inside a "—", which stops the program. In a text with
    no such character the same mistake gave a wrong smallest width, and
    nothing failed. Fixed in `gogo-text`; the counts above are from the
    run after the fix.
  - Checks for it, each seen failing first:
    `a_second_line_folds_to_its_own_longest_word` (a word 67 wide measured
    as 12) and `every_skill_can_be_picked` (stopped with the message the
    click gave). The first tests of the screen clicked only the last row.
  - Remaining: the commit (the user's).
- <span style='color:orange'>LATER</span> AD25: `cr-code-review` as a system skill (user,
  2026-11-21: "can skip for now"). 32 of its 183 lines name Fantasia's
  port: it is to be written again, not copied.
- <span style='color:red'>TODO</span> AD26: the carried skills can be updated. Not designed.
  The user's notes ([TODO-main.md](TODO-main.md), "System skills") ask
  for: a visible note of what changed, updates that take effect at the
  start of a session, one step back, and system skills that stay
  read-only with the user's own layer on top.
- <span style='color:red'>TODO</span> AD27: Storvald opens a folder and drives the Claude agent
  with its system skills (AD23). Not designed. Until then an agent reads
  the copies under `.claude/skills`, and a test keeps them the same as the
  carried ones.
- <span style='color:red'>TODO</span> AD28: rules for agent work in this repository's `.claude`
  folder (user, 2026-11-22: "check all the claude and other docs in
  FanRust for style, language, expression etc and move them to this
  project's claude folder", and "I want to make sure we start with concise
  explanations, ability to read/write/maintain code well, ergonomics,
  active voice").
  - Landed 2026-11-22, not committed: `.claude/CLAUDE.md` (the four things
    every change is measured against, and gogo's goals as the Fantasia
    repository's `CLAUDE.md` has them), `.claude/rules/language.md`,
    `code.md`, `ergonomics.md` and `working.md`, and the skills `doc`,
    `explain-comments`, `make-todo` and `wait-what`.
  - Read for it: the Fantasia repository's `CLAUDE.md` and its eight
    skills, FanRust's `docs/DECISIONS.md` (D2, D11, D71, D89),
    `docs/GOALS.md` and `docs/todo-DONE-rust.md`, and the rules Claude
    keeps for that project. Left there: `build-store`, `publish-store`
    and `xaml-review` (Fantasia's own work), `cr-code-review` (AD25), the
    settings file that forbids git writes, and its hook.
  - Remaining: the commit (the user's).
- <span style='color:red'>TODO</span> AD29: what the first screen does not do yet. Each is a gap
  a user of the window meets.
  - A skill's text shows as written, with its `#` and `**`: nothing draws
    Markdown.
  - Text cannot be selected or copied.
  - The mouse wheel moves a `Scroll` and a `List` through TurboDot
    (`Ui::wheel`), because gogo pans them by touch only. gogo's own wheel
    round has not happened.
  - A double click on `storvald.exe` opens a console window beside the
    app's window.
    - Fixed 2026-11-22 for Windows 11 24H2 and later (AD30).
  - Signs Roboto lacks ("→") come from a font of the operating system.
    Only Windows was tried; on a computer without such a font they show as
    boxes.
  - The page is compiled when the program starts, not when it is built
    (`gogo_ui::build_pages`), so a mistake in it shows at the first start,
    and in the tests.
  - Started by a double click, the program has nowhere to print. A page
    that cannot be reloaded, and a crash, say their reason to nobody: no
    log file, no message in the window (found with AD30).
- <span style='color:red'>TODO</span> AD30: a double click on `storvald.exe` opens no console
  window. USER SEES, before: a black console window opened beside the
  app's window and stayed as long as the app did.
  - Why it opened: `storvald.exe` is a console program, so that a terminal
    waits for it and reads its exit code. Windows opens a console for a
    console program that is started with none.
  - DECIDED 2026-11-22 (user: "just do one .exe. win11 24h2 is good"). The
    program carries a manifest with `consoleAllocationPolicy` set to
    `detached` (`crates/storvald/storvald.manifest`, put in by
    `crates/storvald/build.rs`). Windows 11 24H2 (build 26100) and later
    read it. An older Windows opens the console as before, and so does a
    Windows build with the GNU linker.
  - Weighed and set aside: a window program (a terminal would not wait for
    `storvald` any more); closing the console after the start (it shows
    for a moment); a second small program beside the first, as Godot has
    (two files).
  - Landed 2026-11-22, not committed. Check, seen failing first:
    `the_windows_program_asks_for_no_console_of_its_own` reads the built
    program for the line. By hand on the user's PC (build 26300), the
    program started through Explorer, as a double click starts it: before,
    `conhost.exe` ran as its child; after, it has no child process, and
    its window opens alone. From a terminal, `storvald --version` prints
    and returns 0, and `storvald go` returns 2.
  - Proven 2026-11-22: the whole suite 97 suites, 1289 passed, 0 failed,
    63 ignored; the packaged crates 88 suites, 1249 passed, 0 failed, 54
    ignored. The new test ran in both.
  - Remaining: the commit (the user's).
