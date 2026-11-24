Bitmaps: the picture scales to fit the box (Uniform), centered. It gives
the box no minimum size unless the file asks: `width=hug` / `height=hug`
is the picture's own size, `aspect=natural` follows its shape, and a bare
Image (no size word) in a `Column` takes the Column's width and the
height the picture's shape gives there. `source=` names the picture
file, scheme first: `res://path` is an asset embedded at build time. A
picture the program's code makes goes in a `Surface draw=picture`, not
in an Image — see the `Surface` card.

```
Image logo source="res://icon.svg", width=44, height=44
```

`loading=lazy` asks the host program to decode the picture off the frame,
starting at its first draw; the node shows empty until the pixels land.
gogo decodes nothing itself: the word is kept on the node for the host to
read. For image-heavy catalogs; default is `eager`.
