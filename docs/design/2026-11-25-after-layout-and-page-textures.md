# A call after layout, and a page's nodes drawn into a texture (2026-11-25)

Status: built. Ruled by the user 2026-11-25 ("after_layout sounds good",
"go with Q then", "draw only in gpu_prepare", "I like the before_draw and
other suggestions b and c", "go").

## Why

A collage screen fits its picture into the room its file leaves it:
under a strip of buttons, inside the safe insets, beside a back button
in landscape. The code reads that room's rectangle, fits the picture
into it, and writes where the picture, a dotted outline and a small bar
sit. Three of the collage's frames are lines and words written in a
`.gogo` file of their own, drawn into a texture at the exact size of the
picture, and again at the size of the saved file.

In the first host of gogo, the host did both. It laid its pages out,
called its own code "after layout", laid out again what that code wrote,
and drew a file into a texture with its own renderer. A page of a
TurboDot program could do neither:

- `Page::frame` runs before layout. A rectangle read there is of the
  frame before, so the picture sat in the old place for one frame after
  a phone was turned.
- A page has no renderer, no fonts and no language to draw nodes with.

So such a screen could not be a page, and each program would write that
host code again.

## `Page::after_layout`

```rust
fn after_layout(&mut self, host: &mut Host<'_, A>) { }
```

Called after the page was laid out: every node has its rectangle of
this frame (`host.ui.rect(node)`).

- **Only in a frame that laid the page out.** A page is laid out again
  when a property, a text or a state of it changed, or the window's size
  or scale. A still page is not called, and a program with no layout in
  a frame pays nothing for the call.
- **What the page writes there is laid out once more before the draw.**
  That second layout makes no second call.
- **A page placed in a node of this page** (`Nav::place`) is laid out
  again in the node's new rectangle, before its own `after_layout`.
- **While the page is drawn.** A page laid out while a fullscreen page
  covers it, or while it is cached, gets the call in the first frame it
  is drawn again.
- **A page it asks something of** (`Nav::tell`, `Nav::go`) and that is
  laid out for it is called in the same frame, four rounds at most. Two
  pages that tell each other something at every call would never end a
  frame; the one still owed its call gets it next frame.

The cost of misuse: a page that writes there at every call is laid out
twice in every frame that lays it out.

The name says when. `laid_out` was looked at: it does not say what was
laid out. `frame_laid_out` reads as "the second half of every frame",
and the call is not made every frame.

## `Page::before_draw` and `PageGpu`

```rust
fn before_draw(&mut self, host: &mut Host<'_, A>, gpu: &mut PageGpu<'_>) { }
```

This is `gpu_prepare` under another name, with another third parameter.

- **The name.** "Prepare" is what graphics libraries call the step
  before a pass. It says nothing to someone who looks for "where do I
  draw my page into a texture". `before_draw` says when, and with
  `frame` and `after_layout` the calls of one frame read in order.
- **`PageGpu`** has `device` and `queue`, as `Gpu` had. It also holds
  the renderer the screen is drawn with, the fonts and the program's
  language, which a page never names. A function that draws nodes takes
  a `PageGpu`, and a `PageGpu` exists only in `before_draw`: the types
  lead there.

What a page writes in `before_draw` is laid out again before the draw,
as before. `after_layout` hears of that layout at the next frame.

## Nodes into a texture

```rust
fn before_draw(&mut self, host: &mut Host<'_, App>, gpu: &mut PageGpu<'_>) {
    let label: Rect = host.ui.rect(NAMES.label);
    let tile = host.ui.draw_to_texture(gpu, NAMES.label, TexturePlacement {
        px_per_epx: 3.0,
        origin_epx: (label.x, label.y),
        size_px: ((label.w * 3.0).round() as u32, (label.h * 3.0).round() as u32),
        turn: None,
    });
}
```

`PageUi::draw_to_texture` draws a node of the page and everything inside
it into a new texture, as the page's last layout has them. The walk, the
text shaping and the shader are the screen's: what the user sees is what
the texture gets. The texture's scale is the caller's
(`px_per_epx`): a text burned into a painting is drawn at the painting's
scale.

- **Only in `before_draw`.** It is the one call with a GPU, it runs
  after layout, and a program run without a GPU (a test) never makes it.
  A page that wants a texture at a tap sets a flag in `handle` and draws
  in the `before_draw` of the same frame.
- **The result is typed**: `Tex_Tile_Premul_Srgb_4xU8_RGBA`, colors
  multiplied by alpha, transparent where nothing was drawn.
- **Each call makes a texture, a pass and a submit.** For a change (a
  new size, a tap on "keep"), not for every frame.

The function a host with pages of its own calls, with its own renderer,
is `PageUi::draw_to_texture_with`. It had the short name before.

## A page kept off screen

```rust
fn loaded(&mut self, host: &mut Host<'_, App>) {
    self.art = Some(host.nav.keep_off_screen(FrameArtPage, 1024.0, 960.0));
}

fn before_draw(&mut self, host: &mut Host<'_, App>, gpu: &mut PageGpu<'_>) {
    let Some(tile) = self.art.and_then(|art| gpu.draw_to_texture(art, ART.versus, place)) else { return };
}
```

`Nav::keep_off_screen(page, width, height)` keeps a page that no screen
shows. It is a page like any other: its own file, its own struct,
`show`, `frame`, `after_layout`, the program's language, hot reload. It
is the sibling of `Nav::place`, which puts a page into a node.

- **Laid out for its own size, at one pixel per epx.** It has no screen,
  so the window's size, scale and safe insets do not reach it. A draw
  names the scale of its texture.
- **It lives with the page that keeps it**, and has its frames while
  that page is drawn.
- **`keep_off_screen` returns a name for it** (`OffScreenPage`). Who has
  the name draws the page, or one node of it:
  `gpu.draw_to_texture(name, node, place)`, and reads where its nodes
  sit: `gpu.off_screen_ui(name)`.
- **Another page writes into it** with `Nav::tell`. What was told in
  this frame is laid out before the draw.
- **Made when the call that kept it returns, laid out at the next
  frame.** Drawn before that, it draws nothing (`None`).
- **Not in this build:** its own `before_draw` is not called, a page
  placed in it is not drawn with it, and no finger reaches it.

A `Surface draw=commands` of a kept page gets its commands, and a
picture from a file that only a kept page shows is uploaded.

## What a frame calls, in order

```
frame          each drawn page, before layout
               (layout)
after_layout   each page laid out in this frame, then layout of what it wrote
before_draw    each drawn page, with the GPU
               (the pass that draws the pages)
```

## gogo in a 3D world: three roads

A game wants gogo in its world: a HUD in VR, a screen on a wall, damage
numbers over enemies. No single mechanism does all of it well.

1. **UI drawn into a texture**, shown on a quad by the game. Right for a
   few panels: a VR HUD, a screen on a wall. A page kept off screen is
   its base. Wrong for fifty damage numbers: a texture each.
2. **Nodes over the world, moved by code.** A page over the world's
   `Surface`; each frame the game writes a label's `translate.x/.y` from
   the enemy's place on the screen. Sharp text, the file's own anims,
   nothing new in the engine. No use in VR, which has no screen.
3. **The renderer draws inside the world's pass**, with the world's
   camera. Sharp at any distance and right in VR. A large change to the
   renderer; not built.

Not built for road 1 yet, and not shut out by this shape: a draw into a
texture the page keeps (none made per frame), a flag "changed since the
last draw", fingers from a ray, and a quad buffer of its own for a
panel drawn every frame (a draw into a texture uses the screen's quad
buffer, so the screen's next frame writes its quads again).

## Looked at and not done

- **A call that draws a file, with no page behind it**
  (`draw_file_to_texture(&PAGE, node, size, place)`). One line to use,
  and a dead end: no anims, no texts from code, no fingers. A VR HUD
  would need a second mechanism, and this one would then be a second
  way to do the same thing.
- **Said in the file**: a box whose children are drawn into a texture
  (`draw=texture`). Easiest to read. But a collage needs two sizes at
  once, chosen by code, and the hidden nodes would be laid out with the
  page at every layout. It can come later as a shorter spelling.
- **Drawing to a texture in any call.** Every call would carry the GPU,
  and page code would stop running in a test without one.
- **Only the page that keeps a page may draw it.** The name is enough:
  who was handed it may draw.

## Added the same day: a picture taken off its Surface (RULED)

`host.ui.clear_picture(NAMES.photo)` takes the picture off a `Surface
draw=picture`: pixels (`set_picture`) or a texture
(`set_picture_texture`).

- **Why.** A Surface held its picture until its page was freed. A page
  that slides out for 0.2 s kept its photos that long, and a page opened
  in that time added its own on top: tens of megabytes more for a moment
  on a phone. The first page to draw its own pictures (a collage of
  photos) met it.
- **What it does.** The Surface shows nothing from the next frame on
  and keeps the size its file gives it. The page lets go at the call: it
  frees the pixels' buffer and drops its handle to the texture. The
  renderer lets go at the next frame drawn on a GPU. A texture is freed
  once the page's own code holds no handle to it either.
- **One call for both kinds of picture.** A page does not have to
  remember which of the two set calls it used.
- **A picture set after** shows as a first one does, under the
  Surface's own key, also when no GPU frame came between the two calls.
- **Cost.** A frame in which no page took a picture off: two compares a
  live page in the GPU step. The work itself is out of line.

Tests, in the notes example and in `page_ui.rs`:
`a_picture_taken_off_its_surface_is_not_drawn`,
`a_picture_taken_off_its_surface_leaves_the_gpu` (needs a GPU),
`a_cleared_picture_is_gone_from_its_page`.

## Added the same day: `IMAGES` and `FONTS` are statics (RULED)

The build writes the two tables that point at file bytes as `pub
static`, not `pub const`.

- **Why.** A const is copied into every crate that names it, and with it
  what it points at: here the bytes of every picture, or every font. A
  program that named `IMAGES` from four of its crates held each of its
  255 pictures four times, 13 MB too many in each of its two libraries.
  Nothing said so; the program's size did.
- **What changes for a program.** Reading the tables is as before
  (`IMAGES.iter()`, `IMAGES[key.index()]`, `fonts: FONTS` in a
  `static`). A `const` of the program's own that is made from one of the
  tables no longer builds: make it a `static`.
- `KEYS`, `SLICES` and the pages' own tables stay as they are: they
  point at no file.

Test: `a_build_writes_down_what_a_reload_needs` (`crates/gogo-ui/tests/reload.rs`)
reads the two lines of the generated file.

## What breaks for a program that builds these crates

| Was | Is |
|---|---|
| `Page::gpu_prepare(&mut self, host, gpu: &Gpu)` | `Page::before_draw(&mut self, host, gpu: &mut PageGpu<'_>)` |
| `&gpu.device` in that call | `gpu.device` (it is a reference) |
| `PageUi::draw_to_texture(renderer, device, queue, assets, node, place)` | `PageUi::draw_to_texture_with(...)`, same parameters |
| `turbodot::prelude::Gpu` | `turbodot::prelude::PageGpu`; `turbodot::Gpu` stays |

## Tests

In the notes example (`examples/notes/src/main.rs`):

- `a_page_is_told_after_its_layout_and_what_it_writes_then_shows_in_that_frame`
- `a_placed_page_follows_what_its_page_writes_after_layout`
- `a_page_laid_out_under_another_is_told_when_it_shows_again`
- `a_page_told_after_another_s_layout_is_laid_out_and_told_in_that_frame`
- `a_page_kept_off_screen_is_laid_out_for_its_own_size_and_lives_with_its_page`
- `a_page_draws_its_own_node_and_a_node_of_a_page_it_keeps_into_textures`
  (needs a GPU)
