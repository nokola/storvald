Gives the GPU draw calls of a `Surface draw=gpu` a depth buffer: what
the code draws nearer hides what it draws farther, whatever the order of
its calls. For a 3D world.

```
Layers width=fill, height=fill {
    Surface world draw=gpu, depth=true, width=fill, height=fill
    Label score text="0", halign=right, valign=top
}
```

- Only with `draw=gpu` (G7016): gogo's own drawing is in paint order and
  never tests or writes depth. The label above is drawn over the world,
  however near the code drew.
- One Surface that says `depth=true`, in any page, gives every frame of
  the program a depth buffer. A program with none never makes one and
  pays nothing. So the word is written on the Surface's own line, never
  in a `when` (G7018).
- The depth inside the Surface is 1.0, the far end, when its code is
  called. Two such Surfaces in one frame do not see each other's depth.
- In a program that has a depth buffer, every pipeline the code draws
  with in a `draw=gpu` Surface names the depth buffer's format, also in
  a Surface without `depth=true`. The code is handed the format.
