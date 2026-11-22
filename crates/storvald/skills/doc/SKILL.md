---
name: doc
description: Write or improve Gogo (or any technical) documentation in the "Rust book" teaching style — motivate first, show working code, define every term before use, ban unexplained jargon. Use this skill whenever the user asks to write docs, explain a feature, clarify an explanation, "make this less jargony", rewrite AI-generated explanations for humans, or review documentation for clarity. Also use it when producing any tutorial, chapter, README section, or feature announcement, even if the user doesn't say "documentation".
---

# Writing documentation that teaches

The goal of a doc page is not to describe a feature. It is to take a reader
who has never seen the feature and leave them able to *use* it, quickly, and
feeling good about it. Every rule below was earned by iterating on real docs;
each fixes a specific way explanations fail.

## The shape of a chapter

Motivate → show → build → generalize → recap.

1. **Concrete anatomy first.** Before any concept, name the physical parts
   the reader can see. ("A slider is made of pieces: the track, the progress,
   the thumb.") Use a bullet list. Never use a term like "the progress"
   before defining it.
2. **A problem story before the feature.** Show what goes wrong *without*
   the feature, as an observable event with a character in it: "Someone ships
   a theme with `margin_left=40`. Every slider's thumb is now glued 40 pixels
   from the left. You drag it, it snaps back." The reader must *feel* the
   problem before you name the solution. Abstract phrasings ("this would
   break encapsulation") teach nothing.
3. **The feature as the answer.** Introduce the keyword, show the minimal
   declaration, and state plainly what it does for the reader. "That one word
   does two things for you: …"
4. **Build one.** A complete, working, minimal example the reader can type
   in — a progress bar in a handful of lines beats three paragraphs of
   description. Then extend it: style it, then deliberately break it and show
   the error message. Seeing the compiler catch the mistake is the "aha".
5. **Generalize.** Now the full option list, the design rationale, the edge
   cases. These land only after the reader has something working.
6. **Recap.** Four or five bullets restating what they learned, ending with
   what they can now build. Send them off happy: "You now know enough to
   build X, Y, or Z."

## Language rules

These are the failure modes to hunt for on every pass:

- **No unexplained "the X".** "The boundary", "the control", "the problem" —
  if a "the" points at something not yet introduced, either introduce it or
  rewrite. Section titles especially: "A problem without `automatic`" not
  "The problem".
- **Titles state the benefit or the claim.** A title should be scannable as
  a fact: "`automatic=` marks the property Gogo updates", "`automatic=` works
  in control declarations only", "Restyle any part of a control without
  breaking it". Not "Overview", not "Declaring it", not clever titles the
  reader can't parse ("Why six, and not any property with a formula" —
  unclear until read; "Why you can't add your own kinds" — clear instantly).
- **Name the actor.** Never "I", "it", or "the control" doing something when
  the reader can't tell who that is. Say "Gogo will automatically update…"
  or "the compiler rejects it". When ownership matters (styles vs. runtime),
  spell out which side every time.
- **Replace jargon verbs with plain ones.** "Pin", "mark", "drive", "enforce
  the boundary" → "glue to 40 pixels", "add", "update from the value",
  "doesn't compile". Test: would a friend explaining this over coffee use
  that word?
- **Consequences as behavior, not adjectives.** Not "styling it would break
  it" but "you drag it with your mouse, it snaps back to 40". Not "the
  slider stops working" but "you can't see the value and you can't change
  it".
- **Backticks for keywords, bold for concepts.** `slot` the keyword vs. a
  slot the part. `automatic=` always with the `=` if that's how it's
  written. Bold a term once at definition, then plain.
- **Show real error messages** in the project's actual diagnostic format
  (`error[G3002]: …` with its note/help lines), as fenced blocks. An error
  message is the best documentation of a rule.
- **One idea per sentence in explanatory prose.** Long compound sentences
  are fine in stories, deadly in rules.
- **Cut analogies that don't carry weight.** "It's the Rust trade" means
  nothing to someone who doesn't already agree. If the reasoning stands on
  its own, the name-drop is noise — delete it.
- **Cut repetition.** If a fact appears in a bullet list, don't restate it
  in a closing paragraph ("What exists today: …" after each item already
  said so).

## For lists of options/variants

Each item gets: the plain-words meaning, a concrete example ("That's a
progress bar's fill"), and — when items claim or own something — a labeled
sub-line stating exactly what (`*Takes over:* width on horizontal, height on
vertical`). Parallel structure across all items; if one has an example, all
do.

## Honesty markers

If any detail in the doc is inferred rather than confirmed from the spec
(invented syntax, guessed property names), flag it to the user in your reply
— never silently, and never inside the doc itself.

## Process

When rewriting an existing explanation, work in passes and expect several
rounds:
1. Restructure to the chapter shape above.
2. Hunt language-rule violations (read every "the", every verb, every
   title).
3. Add the build-one example and recap if missing.
4. On each user correction, apply the fix *and* scan the whole doc for other
   instances of the same failure — the user flagged one, there are usually
   more.

## Ground every example in real sources

- Take syntax, names and error wording from the project's test fixtures,
  its production files and the compiler's own error strings — never trust
  a draft's syntax, color names, slot names, or error wording without
  checking.
- Where the project compiles or runs the examples in its docs, an example
  is finished when that check passes on it.
- Docs referent rule: never bare "the box"/"the frame" — name whose
  rectangle it is, with a tiny inline example.
