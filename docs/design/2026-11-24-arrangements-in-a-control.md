# Arrangements inside a widget or control (2026-11-24)

Decision record: why a widget has arrangements of its own and why the
rules are the ones they are. The normative rules live in gogo-spec.md
("Arrangements inside a widget or control") and REFERENCE.md; this file
keeps the reasoning, so it is not worked out again from memory.

## A control with poses could not be used twice

An `arrangement` is a named pose of a whole page: the page is in one at a
time, and `when <name> { ... }` blocks say what each one changes. Take a
switch with three poses (off, option 1, option 2) and a page that needs
two of them side by side. A `widget` could not hold the poses:
`arrangement` was a word of the page, the page has one arrangement in
force, and both switches would flip together.

The first host of gogo had exactly this page. Each switch was a small
page of its own, hosted by code inside the bigger page: a host per switch
in the program, a second road for input and drawing, and nothing in the
bigger page's file showing where the switches sit.

## What was decided

`arrangement` lines beside the body of a `widget` or `control`. Each
instance is in one of them at a time, on its own.

```gogo
widget CompactSwitch {
    arrangement option1
    arrangement option2

    transition option1 -> option2 duration=0.5, easing=cubic_out
    transition option2 -> option1 duration=0.5, easing=cubic_out

    when option1 { tile2.margin_top = -60 }
    when option2 { tile1.margin_top = 60 }

    Layers box width=60, height=60 {
        Button tile2 60x60, overlay=true
        Button tile1 60x60, overlay=true
    }
}

View page {
    CompactSwitch restoreSwitch
    CompactSwitch adjustSwitch
}
```

```rust
rt.go_in(&NAMES.restore_switch, CompactSwitchArrangement::Option1);
```

## The five questions, and the answers (the user, 2026-11-24)

1. **The word.** `arrangement` inside a widget, the same word as on a
   page. One word for one idea; where it is written says whose it is. A
   second word (`pose`) would have been a second thing to learn for the
   same behavior.
2. **The call from code.** `rt.go_in(&NAMES.instance, X)` beside the
   page's `rt.go(X)`. The runtime holds every other state of the page;
   a call on the names (`NAMES.instance.go(&mut rt, X)`) would have been
   the only state change not written `rt.<verb>`.
3. **In a `repeat` row.** Each row is in an arrangement of its own, with
   no rule of its own: a row is an instance like any other.
4. **What glides.** On a timed flip the boxes of the instance glide, and
   a box outside it that the flip moves jumps. A widget times its own
   body; it does not reach out and animate its neighbours. A page that
   wants the neighbours to slide times them itself.
5. **May the page read an instance's arrangement in its own file?** Not
   in this version (`when restoreSwitch.option1 { ... }`). Code can
   (`rt.arrangement_in`). It is left out until a page needs it: it would
   be a second kind of condition on the page, with its own questions
   (which instance, what in a row).

## Rules that follow

- **A name the widget declares is the widget's own in its body.** A
  word in a `when`, `transition` or `go(...)` of a widget's body that
  the widget declares is that widget's arrangement, whatever the page
  declares: the nearest declaration wins, as a name does in Rust.
- **A name the widget does not declare is still the page's.** The first
  draft refused that. It was taken back when the build showed a widget
  that follows the page's arrangement on purpose: a chip that shows in
  the page's names-only view (COOKBOOK, "A names-only view of the same
  mosaic"). Such a widget does not build on a page that lacks the word,
  and the error names it.
- **The page's lines cannot name a widget's arrangement.** That is a
  build error which says whose the word is.
- **The instance's own line is the page's.** `CompactSwitch s
  on:press=go(expanded)`, the `when` blocks under an instance and what a
  page puts into a slot are compiled as lines of the page.
- **`base`** is an instance with no arrangement on.
- **A new row, and an instance at start, is in none.**

## What it costs

- A page with no such widget keeps no table and compares nothing: the
  lists are empty.
- A flip runs the `when` blocks of the nodes of that instance that watch
  its arrangement, and no other node's.
- A control that stands still in its page costs a frame what any node
  that does not change costs (the resolve names the nodes it touched,
  and the kept layout and draw list read those).
- The runtime keeps one entry per instance of such a widget.

## How it is built

- `Page::control_arrangement_names` holds one run of names per widget;
  `Page::control_arrangements` holds one entry per instance (its root
  and its widget's run).
- `State::ControlArrangement` and `State::ControlArrangementScreen`
  hold a place in the names; the `When`'s `of` is the instance's root.
- `Action::GoInControl` and `Action::ToggleInControl` hold a place too.
  A handler is shared by every clone of its node, so it holds no root:
  the instance is found from the node the event is on.
- A flip is a change on the instance's root, as a control's state is, so
  the rule search, the tweens and the `anim=` of a rule take the road a
  state flip takes. The box glide lists the nodes of the instance and no
  other.

## A widget's anims from code (decided 2026-11-24)

The switch above also fades a toast, and its code plays that anim. An
`anim` beside a widget's body is copied for every instance, under the
instance's name (`restoreSwitch/toastFade`), so code had only a string to
name a copy by: a misspelled one played nothing and said nothing.

A build writes `enum <Widget>Anim` per widget that declares anims, tied
to `<Widget>Names` as the arrangement enum is:

```rust
rt.play_in(&page, &NAMES.restore_switch, CompactSwitchAnim::ToastFade, Finish::Nothing);
```

- `Runtime::play_in`, `stop_in` and `is_playing_in` take the instance
  and the anim. `Page::anim_in` gives the name of the copy, which
  `take_finished` hands out.
- The enum holds the anim's place among the ones the widget declares.
  `Page::control_anim_names` holds, per instance, the names of its
  copies in that order; `Page::control_anims` holds one entry per
  instance (its root and where its names sit).
- A row a `repeat` clones had copies of its own already; it now gets an
  entry that names them.
- A hot reload keeps the build's places: an anim the file drops leaves a
  place with no name, and a play of it plays nothing.
- A page with no such widget keeps no table.

Why the names of the copies are not fields of `<Widget>Names`: the names
of a `repeat` row are the template's names moved by the row's distance
(`rebase`). A node id moves that way; the name of an anim does not.
