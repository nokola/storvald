# Code that is easy to read, write and maintain

The bar gogo is held to holds for the Rust code: easy to read six months
later, easy to write, easy to find. Types carry the meaning, and the
compiler catches the mix-up.

## Names

- Name a thing the way you would say it to a friend:
  `get_next_line_params_slot`, not `rent_line_params_slot`.
- A name says what the thing does. Do not shorten it:
  `recalculate_view_size_and_scale_if_needed`, not `recalc`. A long clear
  name beats a short unclear one.
- A bool's name leads with a verb: `is_in_gesture`, not `in_gesture`.
  `was_`, `has_`, `can_`, `must_`, `did_`, `do_`, `are_` are fine. Fields,
  locals, parameters. Neighbours without a verb are no reason to drop it.
- A conversion names its target first: `x = linear_from_srgb(y)`, never
  `srgb_to_linear`. The reader sees the variable next to what it now holds.
- A const is SNAKE_CASE and keeps its unit: `THUMBNAIL_WIDTH_EPX`,
  `IDLE_TIME_MS`. A rename never loses meaning.
- No comment that repeats the name beside it.
- Before you name a new type, look at how its nearest relatives are named
  and follow them.

## Types

- A newtype crosses an API as the newtype. Only the crate that owns the
  domain unwraps it (`.RGBA`, `.0`). A caller that needs the raw value
  asks the owner for an operation.
- Two values of one primitive type that mean different things get a
  newtype each: epx and device pixels, straight and premultiplied alpha,
  an index and a count. No bare-number and no string contracts where a
  type fits.
- Ids are types: `NodeId`, `StrId`, `KeyId`. Code never hands a string
  where a node belongs.
- A texture, a color and a pixel buffer are named by what they hold:
  `Tex_<Role>_<Alpha>_<Space>_<channels x type>_<channel map>`
  (`Tex_Tile_Premul_Srgb_4xU8_RGBA`, `Color_Straight_Srgb_4xU8_RGBA`,
  `Buf_Straight_Srgb_4xU8_RGBA`).
- A local says its type: `let count: usize = rows.len();`. Leave it out
  where the right side already names it (`Ui::new(..)`, a struct literal).

## Shape

- One code path. No second renderer, no branch per vendor, unless a
  crash or a wrong result needs it.
- A crate knows only what it needs. `gogo-ui` has no dependency, no
  window and no GPU. The gogo crates never name the engine or the tools.
  `turbodot` owns the window and the GPU. An app's own code knows
  neither: it runs the same in a window and in a test.
- One way to do a thing. A second way costs every reader, and it sends
  an AI down the wrong one.
- Make a mistake impossible to write before you document it.
- A call that did nothing must not pass: an unknown command fails with a
  code and names what was asked.
- An error teaches: a code, the line, and how to fix it.
- A part nobody uses costs nothing until it is used.

## No double work

- When the final value is known up front, build it directly. No
  create-then-fill, no build-then-rebuild.
- A hot path allocates nothing per frame. Never allocate inside a loop
  over pixels.

## Comments

- Every comment you write or edit goes through the `explain-comments`
  skill: what, why ("Needed because ..."), where the guarantee stops,
  warnings, what it returns.
- No tracker ids or tags in code. A full reference is fine:
  `(docs/todo-ai-dev.md/AD7)`.
- A fix's comment says what a user would have seen without it.
- No stories of how the code used to be.

## Tests

- A goal without a check does not go red. A rule worth keeping gets a
  test or a lint.
- See a new test fail before you trust it: break the code, watch that
  test go red, put the code back.
- Test logic with no window: a touch as numbers, events as values, a
  layout as rectangles, pixels from a picture.
- A test that needs a GPU is `#[ignore]` with the reason. It never
  returns early and counts as passed.
- Every example in the docs is compiled by a test.

## Dependencies and files

- A new dependency: look up its latest stable version and pin that one.
- Text files are LF, on every OS.
