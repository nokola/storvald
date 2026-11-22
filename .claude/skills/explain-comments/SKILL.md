---
name: explain-comments
description: Write or review code comments - six-slot shape (what, why, gotchas, mechanism, returns, warnings), plain words, fully-qualified decision refs. Run whenever a turn writes or edits ANY comment, including refactor collateral.
---

# Explain comments

## The shape — slots in this order, skip a slot only when it's empty

1. **What it does and/or where used.** One sentence.
2. **Why it's needed.** A statement answering "if this code were deleted,
   what breaks?" Default opener "Needed because/so/for…" — drop the word
   only if the sentence still answers that exact question.
3. **Gotchas.** Where the guarantee stops, if anywhere.
4. **Mechanism.** Only what the code can't show; skip if the code shows it.
5. **Warnings.** Unexpected things to watch for, e.g.
   `WARNING: Call only on effect apply, to avoid allocation.`
6. **What it returns.** E.g. "Returns item info if item found, or `None`
   if not."

Add empty line between slots. In Rust a comment is `///` lines with no tags, and what it returns is the last paragraph, opened with "Returns". In a language whose doc comments have tags (C#), keep the summary tags on their own line and use the returns tag if the method returns a value.
Telegraphic: spend words on facts, not grammar. Drop "a", "the", "that is", "which", "there is", "in order to". Subject first, the thing the comment sits on: "Page cached on back", in page's file. Exact technical term over paraphrase: "cached", not "set aside". Keep a word when dropping it leaves two readings.
Where an article stays, use "a" instead of "the" e.g. "a filter" rather than "the filter" unless the comment is specific to a single filter.
Never write "the X" for something only the writer has in mind — a comment
explains on its own: "an apply runs on the GPU", not "the live apply";
"the same order EffectWorker.StartCycleBody uses", not "the worker's exact
order". If "the" is right, the same comment (or the code next to it) must
name the referent (user rule 2026-08-09).
Short comments lead with the most important fact — a validity constraint
may come before the what: "Valid only during tracking: whether history
existed when the effect was entered — …".
Expected invariants survive a rewrite at full strength ("reached ONLY
through a live history item") — never soften or drop them.

Worked example (slots marked; 3 and 4 empty here, so skipped):

```
/// Converts stored value to the `Box<dyn Any>` a filter takes.        [1]
///
/// Needed because filters take property values as `Box<dyn Any>`
/// (`Filter::set_property`), so value must be boxed somewhere —
/// this is that one conversion.                                       [2]
///
/// WARNING: Allocates on every call — call only on effect apply,
/// never per input event.                                             [5]
///
/// Returns boxed value; panics on uninitialized default.              [6]
```

## The generating rule: say it, write it, then cut the grammar words

Answer a coworker asking "what does this X do?" out loud, then write the
words down. "Oh it gets distinct sectors that are touched by dirty rects.
Used to make sure we diff each sector once when we draw multiple times
during paint" — drop the "oh", and that IS slots 1 and 2:

```
/// Gets distinct sectors that are touched by dirty rects.
///
/// Used to make sure we diff each sector once when we draw multiple
/// times during paint.
```

Then cut the grammar words ("that are", "to make sure"):

```
/// Gets distinct sectors touched by dirty rects.
///
/// Used so we diff each sector once when we draw multiple times
/// during paint.
```

Consequences:

- "we" is fine — spoken register beats document register.
- Slot 2 openers as spoken: "Used to make sure…" / "Needed because…".
- Name the user-level scenario ("when we draw multiple times during
  paint"), not the internal data shape ("stamp rects overlap along a
  stroke"), when both are true.
- A clause you would not say out loud gets cut ("the per-commit walk of
  undo capture").
- CAPS emphasis is good — capitalize the load-bearing word ("reached ONLY
  through a live history item", "Seals SWAP this list").
- Refs (provenance/decision) are not spoken: put them in parentheses wherever
  they read best — upfront, middle, or end.
- Slots 3-6 are written slots — a spoken answer doesn't cover gotchas,
  warnings, or return shapes; add them per the shape.

## The near-miss that keeps recurring

Slot 2 is the one that goes missing: a crisp what/cost/rule sentence
passes a lazy check while the why is absent or demoted to a trailing
clause. All of these failed review:

- "Allocates on every call" — cost, no why
- "The one place that maps a rect to sector indices" — what, why buried
  mid-sentence
- "Merging is only safe when one span contains the other" — rule, why
  demoted to "…so undo diffs never widen" at the end of the sentence
- "Spans come from GetSectorSpan" — mechanism first, no why

Slot 2 must exist as its own sentence. When rewriting a comment,
regenerate it from the shape and then compare — never patch the old text
(keeps you on ITS question) and never correct against only your last
mistake (keeps you one failure behind).

## Decision references

Fully-qualified decision refs are OK and useful:
`(todo-alloc-2026-08-02.md/B3)`. Bare tags (`B3`, `D143`) are not — the
next reader can't find them.

Provenance refs, for code that follows another codebase, are good — keep
them: `(the Unity app's semantics, BitmapSector.cs:348-375)`. Name which
codebase; file and lines say where.

## Cut on sight

- Old-behavior stories and PR-talk: "used to be…", before/after, why a
  change was correct. The next reader never saw the old code.
- Concept teaching (what boxing or reflection is) — state the consequence
  in plain words ("allocations").
- Extension guidance the code shape already shows ("the constructors below
  are the complete list…" next to a visible list).
- Prose enumerations that need maintenance: "etc." beats "complete list" —
  prose lists rot when the code grows.

## Word rules (standing)

- Plain everyday words; no invented metaphors — "rides", "smuggle", "rent",
  "lane". Standard terms (sRGB, LOH, boxing) are fine.
- Name the referent: never bare "the box" / "the rows" — say whose ("the
  effect page's property rows (sliders, checkboxes, pickers)").

## Checklist — run whenever a turn writes or edits ANY comment

Refactor collateral included: "this is a code task, not a comment task" is
exactly how comments dodge review. The check belongs next to running the
tests.

1. Slots present and in order; slot 2 (why) exists as its own sentence —
   a crisp what/cost/rule line with the why missing or trailing still
   fails.
2. Decision refs fully qualified (todo-alloc-2026-08-02.md/B3), never bare.
3. Old-behavior stories, invented metaphors, bare referents? Cut or fix.
4. Could half the lines go with nothing action-relevant lost? Cut them.
   (The founding example went 22 lines → 9 with nothing lost.)
5. Any "a" / "the" / "that is" that can go? Where an article stays: "a"
   instead of "the" unless the comment is specific to a single instance.
6. Read slots 1-2 aloud: is that how you'd answer a coworker asking "what
   does this do?" If not, say the answer and transcribe it.
7. Did the rewrite keep every invariant and constraint the old comment
   stated, at full strength (CAPS included)?
