A box that the program's code draws: a painting, a chart, a game's world.
The file says where the box is, what is over it, and how the code fills
it. `draw=` is required, and there is no default:

- `draw=picture` — the code hands over one picture
  (`Runtime::set_picture`). The Surface draws it as an Image draws a
  file: `fit=`, `window=` and `sampling=` mean the same.
- `draw=commands` — the code writes gogo draw commands (rectangles,
  circles, arcs, text) in the Surface's own space, where (0, 0) is its
  top left corner. gogo keeps them and asks again only when the Surface's
  size changed or the code said its content changed.
  `clip_contents=true` cuts them at the Surface's edge.
- `draw=gpu` — the code makes GPU draw calls of its own (3D, a shader of
  its own), inside the pass that draws the pages, at the Surface's place
  among the nodes. gogo sets the viewport to the Surface's rectangle and
  the scissor to the part that shows, so the drawing moves with a page
  that slides and is cut where the Surface is cut. It cannot be turned:
  `rotate=` on it or on a node above it is an error (G7017).
  `depth=true` gives the calls a depth buffer, for 3D: what is nearer
  hides what is farther.

```
Layers width=fill, height=fill {
    Surface painting draw=picture, fit=distort, width=fill, height=fill
    Surface chart draw=commands, width=200, height=120, halign=right, valign=top
    Panel toolbar valign=bottom, height=56 { }
}
```

A Surface has no size of its own (its line writes one, or `fill`) and
holds no child: what goes over it is a node after it in the same
`Layers`, as the toolbar above. gogo draws nothing of it, so it has no
`background=`.

With `interactive=true` and `on:pointer=<name>` it takes every finger
that lands on it, each with a press of its own, so the code tells one
finger (a stroke) from two (a zoom). Without them a finger goes through
to what is under it.

Its name in the code has a type by its `draw=` (`PictureSurface`,
`CommandSurface`, `GpuSurface`): code that fills it another way than
the file says does not compile.
