An Image's picture, named by a path string that always carries a scheme:

- `res://path` — an asset under the build's resource root, checked and
  embedded at build time. A bare `icon.png` is refused (G0071; the check
  knows `.png`, `.jpg`, `.jpeg` and `.svg` names): a bare
  word could be a label or a path, the scheme says which.
- An atlas slice: `animals[12]`, `toolbar.play`.

Any other name is an error (G2023): an Image shows a file. A picture the
program's code makes (a live brush preview, the tiles of a canvas) goes
in a `Surface draw=picture`, which takes no `source=`: the code hands it
the picture.

An empty string means no picture (no load attempt — safe widget
`$icon=""` defaults). The box sizes with the normal vocabulary; the
picture fits `contain`, centered, unless `fit=` says otherwise.

```
Image logo source="res://icon.svg", width=48, height=48
```
