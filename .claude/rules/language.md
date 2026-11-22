# How to write: replies, docs, comments, trackers, commit messages

One standard for every text, in a file or in chat: a reader who was not
there understands it six months later.

## Plain, short language for anything: docs, chat, comments

- Start with most important first. e.g. goals or facts that are crucial.
- Explain goals and motivation before the "how"
- Human language; jargon only where it is the technical term.
- Short. Bullets. Short clauses, full stops, drop "the".
- "Write A" = paste A verbatim. Don't rephrase, don't add.
- Review = list what to change and the proposed text. Don't apply until told.

In chat: one question, one answer. No recap. No options nobody asked for.

## Active voice

- Name who does what. "The compiler rejects the page", not "the page is
  rejected". "gogo updates the width", not "the width is updated".
- Name the actor every time it matters who acts: gogo, the compiler, the
  host program, the app's code, the user. Never a bare "it".
- Say when things start and stop in everyday time words: "while the mouse
  is over the row", not "while the hover state holds".
- A passive sentence is right only when nobody acts or the actor does not
  matter ("the file is 4 KB").

## Plain words

- Say the literal thing. "X follows Y", "X goes through Y", "X is passed
  in": whichever is true. Not "X rides Y".
- Keep real names in plain sentences: "Godot asks each child
  `get_minimum_size()`". Introduce a name in the sentence that uses it.
  Plain words carry the identifiers, they do not replace them.
- Standard technical terms are fine (sRGB, finalizer, newtype). gogo
  keywords are fine (`hug`, `fill`, `seat`). A phrase built from keywords
  ("margin pose") is not.
- A sentence stands alone. Compressed mechanism-speak counts as jargon even
  when every word is real: "a pose can only grow past a base minimum" reads
  only for someone who read the code. Spell it out: "a button never shrinks
  below its declared width, so the 50 showed as 60 on screen".
- Do not invent status labels. "7 are mostly done; for each, one piece is
  still missing", not "landed-with-remainders".
- Words that failed review, with what to say:

  | Not | Say |
  |---|---|
  | rides, smuggles, rents | follows, is passed in, takes |
  | hoisted | reads it once at the top, into a local |
  | belt-and-braces | extra protection against a case I cannot show |
  | wears a style, wearer | takes a style, the control with the style |
  | seam | the method, or "the one way to do X" |
  | twin | the name of the other thing |
  | doctrine | the decision, with its entry cited |
  | naked X | X without Y |
  | unrepresentable | cannot happen |
  | floods | fills the whole |
  | leg | the branch, named by what it does |
  | baked | declared, wrote, chose for good |
  | poisoned | wrong values |
  | breathing room | empty space |
  | stragglers, loose ends | the unfinished entries |
  | eyeball | look at |
  | holds, is satisfied, iff | "for as long as", "when" |

  The list grows: a word the user questions joins it.

## Name the referent

- Never a bare "the box", "the frame", "the rows". Say whose: "the
  Image's own rectangle, the space layout gives the Image node itself".
  Anchor with a tiny example where cheap.
- Never "the X" for something only the writer has in mind. "The" is right
  when the same text names the referent.
- Lead with what a thing is, then how it is built: "switch control (a
  widget over Toggle)", not "switch composite".
- Name which codebase: "the Unity app", "FanRust", "gogo". Never a bare
  "shipped" or "the old code".

## Explaining

Explain a rule as you would to a colleague: name the two situations in
everyday words and say what happens in each.

> A parent is either **told its size** (`width=300`, `width=fill`, the
> screen) or **asks its children** (`width=hug`).
> - Parent that asks its children ("how wide do you need to be?"): a
>   `fill` child answers with its content, like any other child.
> - Parent that was told its size ("I'm 300 wide, split it"): fixed
>   children take their size, `fill` children share what's left.

A report or a summary:

1. Start from what the user does and sees on screen. Then the defect as
   it looks on screen. Then the fix.
2. Tell the mechanism as a story, with the reason in the same sentence.
3. Describe testing as what you did: "I removed the rule, ran the tests,
   and exactly the new check failed. I put it back and all passed."

A design or architecture answer:

1. Teach how it works today first, with the real names. Keep this part
   when you shorten.
2. Then the gap, as a story in that state.
3. Then the fix, as uses of what you just taught, each cost with the name
   it touches.

## Defects: say what a user sees

Every mention of a defect pairs the mechanism with what a user saw or
would have seen: which screen, which action, what went wrong. In the
summary, in the tracker entry, in the fix's comment.

## Comments, docs, trackers

- A comment, any comment: the `explain-comments` skill.
- A doc page, a README part, an explanation to keep: the `doc` skill.
- A todo or a tracker: the `make-todo` skill.
- DONE means everything is ready and no work is left. A row with work
  left stays TODO: "landed <date>: ...; remaining: ...". Never "DONE
  except X".
- Resolve an entry by its status word only. The original text stays as
  it was; what happened goes in a line under it.

## Commit messages

One short clause per change, in the words the user uses for the feature:
"`$value` is now live, `live()` is removed". No coined phrases. The
version last, when it changed.

## Many changes at once

Report them as one numbered table: `# | Before | After | Verdict |
Comment`. The user drops rows by number. Keep the numbers once sent.

## Before you send or save

Reread once, for these only:

1. An invented word, an idiom, a word from the table above?
2. A passive sentence that hides who acts?
3. Could the reader follow it without having read the code you just read?
4. Could half the lines go with nothing lost?
