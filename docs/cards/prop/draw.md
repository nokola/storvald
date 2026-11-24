How the program's code fills a `Surface`. Required on every Surface;
there is no default, so nobody gets one kind of drawing by forgetting to
ask for another (G7015).

- `draw=picture` — the code hands over one picture. The Surface draws it
  as an Image draws a file (`fit=`, `window=`, `sampling=`).
- `draw=commands` — the code writes gogo draw commands, which gogo keeps
  from frame to frame.
- `draw=gpu` — the code makes GPU draw calls of its own, inside the pass
  that draws the pages, in a viewport on the Surface.

```
Surface preview 240x80, draw=picture, fit=distort
Surface chart draw=commands, width=200, height=120
```

Written on the Surface's own line only: a `when` cannot change it,
because the Surface's name in the code has a type by it.
