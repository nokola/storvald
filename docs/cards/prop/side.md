The side of its parent's rectangle a `Popup` opens on: `top`, `bottom`,
`left` or `right`. Required: there is no default (G7020).

```
View page {
    Button more {
        Label text="More"
        Popup hint side=right {
            Label text="More choices"
        }
    }
}
```

- `side=top`: the Popup's bottom edge on the parent's top edge.
  `side=right`: its left edge on the parent's right edge. And so on.
- A Popup that would reach past the page is moved back inside it. It
  then covers a part of its parent; it is not flipped to the other
  side.
- A `when` may write it: `when screen.landscape hint.side = left`.
