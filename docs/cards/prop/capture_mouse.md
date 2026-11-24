Gives a `Surface` the mouse with no place: the cursor is hidden and
held, and the mouse's moves do not stop at the edge of the screen. A game
turns its camera that way, a 3D editor turns a model.

```
Surface world draw=gpu, depth=true, width=fill, height=fill,
    interactive=true, on:pointer=turn, on:hover=turn, on:wheel=zoom,
    capture_mouse=true
```

- True or false like any other such word: the file's line, a `when` or
  the code writes it. Code writes true when play starts and false at a
  pause.
- In effect only while the word is true, the Surface's page is the top
  page, and the window has the keyboard. A page that opens over it (a
  pause menu) has the cursor with no code; when that page leaves, the
  Surface has the mouse again.
- While in effect, everything the mouse does goes to this Surface: a
  move with no button down to `on:hover=`, a move with one down and each
  button to `on:pointer=`, the wheel to `on:wheel=`.
- A sample's `dx` and `dy` are then the mouse's own counts, before the
  system speeds a cursor up: multiply them by your own sensitivity. Every
  move the mouse reports is one event.
- With no cursor the user can press no button. Give them a key that ends
  it (Esc is the back request).
- One Surface of a page at a time: the first in the file whose word is
  true. On a device with no mouse the word does nothing.
