---
name: make-todo
description: Create or extend a Markdown todo tracker (todo-<topic>.md) — an audit findings list or a milestone map — in a fixed house shape: colored TODO/DONE/DECIDED/LATER/CUT statuses, one overview table, priority sections, one entry per finding with ground-truth cites + what the user sees + the check that proves the fix. Use whenever the user asks for a todo, tracker, audit list, or "what's left" document, or when a review round produces findings that need a home.
---

# Make a todo tracker

Two shapes; pick by content:

- **Findings tracker**: a numbered list of defects/gaps from a review or
  audit, each with a priority and a check that goes red without the fix.
- **Milestone map**: what is left, per milestone/phase, with the project's
  vocabulary spelled out once so a reader six months out needs no decoder.

Both share the status convention and the resolution rules below.

## 1. File and header

- Name `todo-<topic>.md` at the project's docs root (next to the other
  `todo-*.md` files if any). When a tracker closes, rename it to
  `todo-DONE-<topic>.md` — never delete; other documents link into it.
- First line: `# <title> — <one-line what> (<YYYY-MM-DD>)`; append
  `refreshed <date>` on later passes.
- Then, in this order: **why the file exists** (one paragraph naming the
  rule it serves); **how it was produced** (what was reviewed, and what was
  NOT run — e.g. "device checks not run, marked DEVICE"); **sources of
  truth** it summarizes, as links; the **status legend** (verbatim below).

## 2. Status legend — copy verbatim; no emoji, no icons

```
Statuses: <span style='color:red'>TODO</span> /
<span style='color:lightgreen'>DONE</span> /
<span style='color:deepskyblue'>DECIDED</span> /
<span style='color:orange'>LATER</span> (deliberate future work) /
<span style='color:gray'>CUT</span> (decided against).
Flip TODO→DONE keeping the original text; the resolution goes in an indented
sub-line.
```

Priority words (findings trackers): `P1` user-visible today, fix + check ·
`P2` narrower reach or perf/memory, fix or decide · `P3` declared deviation
lacking a proving check, check or consciously accept · `P4` minor, recorded ·
`DOC` stale docs. Add an environment tag when a check needs hardware or a
specific setup (e.g. `DEVICE`). The colored `<span>` word IS the color code —
never colored circles or other emoji.

## 3. Overview table (findings tracker)

ONE table for all priorities (a split per priority was tried and rejected),
priority as a column, in this column order:

```
| Title | Pri | Status | Class | If left as-is (what the user hits) |
|---|---|---|---|---|
| XX7: Rate-app opens an empty listing | P1 | <span style='color:red'>TODO</span> | SUSPECT · DEVICE | Settings → Rate lands on "item not found" in the store |
```

- The title starts with a short stable id — a tracker prefix (two letters
  from the topic) + number, then a colon: `XX7: Title`. No separate id
  column. Ids never renumber; new ones append, or go on top when the user
  asks for that.
- `Class` = how the gap was found. Suggested vocabulary: SILENT-DROP (no
  comment anywhere) · DECLARED-NO-CHECK (a rationale exists, nothing proves
  it) · SUSPECT (call kept, callee reimplemented differently) · STALE-DOC ·
  DESIGN (needs the user's call).
- The last column is what a USER SEES, not the mechanism.

Milestone maps use a 3-column table per phase: `| id | scope | status (+ date, deviations) |`.

## 4. Sections and entries

Findings tracker: `## P1 — …`, `## P2 — …`, `## P3 — …`, `## P4 — …`,
`## STALE-DOC — …`, then `## Verified clean` (what was reviewed and found
correct — so nobody re-reviews it).

Entry shape (one bullet, wrapped at ~76 columns):

```
- P1 · <span style='color:red'>TODO</span> · **XX7: Rate-app opens an empty listing.**
  Reference `File.cs:123-130` does X; implementation `Main.cs:456` does Y
  (cite BOTH sides with line numbers — the fix pass re-reads them).
  USER SEES: <what appears on screen / what the user experiences>.
  Check: <the test or probe that goes red without the fix>.
  <CLASS>, confidence <high|medium|low>.
```

- Bullet order is priority · status · `id: title` (same id form as the table).
- Lead with the ground truth (spec, reference implementation, previous
  version), then the deviation, then the observable, then the check. An
  entry without a USER SEES line is not finished.
- Plain everyday words; name things as the user would; no invented
  metaphors; no tracker ids or tags inside source code.
- Milestone-map bullets: `- <status> <scope>` with an indented
  `- DONE <date> (<ref>): <one-line resolution + deviations>` sub-line.

## 5. Resolving an entry

- Flip the status word ONLY; the original finding text stays byte-identical
  (the finding is the record of what was wrong).
- Add an indented sub-line: `SHIPPED (<date>): …` for fixes (what landed,
  the check's name, what is still owed), `DECIDED <date> (<who>): …` for
  calls, `<ENV> (<where>, <date>): …` for hands-on results. Evidence (log
  lines, numbers) goes there too.
- Milestone-table rows are the opposite: status + a concise deviation list,
  no narrative.
- CUT needs a reason in the sub-line. A dropped behavior that the ground
  truth had still needs a proving check, not just a rationale.
- Severity corrections from the field (e.g. "not rarer — every session")
  go in a dated sub-line, and the table cell is updated to match.

## 6. Producing the content

- Findings come from read-only review that compares the ground truth
  line-by-line with the implementation — producers AND consumers; grep every
  reference before writing an entry. Record what was not reviewed.
- Surface design forks to the user when discovered; don't bury them as
  entries.
- Before saving, check every table row has the same column count as its
  header.
