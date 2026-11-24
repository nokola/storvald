Where a `Popup` sits along the side it opens on: `start`, `center` or
`end`. Unwritten: `center`.

```
View page {
    Button more {
        Label text="More"
        Popup hint side=top, align=end {
            Label text="More choices"
        }
    }
}
```

- Along a `top` or `bottom` side: `start` puts the left edges in line,
  `end` the right edges.
- Along a `left` or `right` side: `start` puts the top edges in line,
  `end` the bottom edges.
- `center` puts the two middles in line.
