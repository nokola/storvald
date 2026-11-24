What a control opens beside itself: a menu, a strip of choices, a hint.
Written inside the node it belongs to, drawn above the whole page.

```
View page {
    Check more width=60, height=50 {
        Label text="More"
        Popup menu side=top, align=end, visibility=gone {
            Column background=#2C2F38, padding=6, spacing=4 {
                Button about text="About"
                Button help text="Help"
            }
        }
        when selected menu.visibility = visible
    }
}
```

- **`side=`** is required: `top`, `bottom`, `left` or `right` of the
  parent's rectangle. **`align=`** is where along that side: `start`,
  `center` (when unwritten) or `end`.
- It is as big as what it holds, and is moved back inside the page when
  it would reach past an edge.
- It shows and hides with `visibility=`. Here the Check shows it while
  it is checked: a tap opens the menu, the next tap closes it.
- A finger on it goes to what it holds; a finger beside it goes to the
  page, and the Popup stays open.
- Back closes the topmost Popup before it leaves the page.
- It is no part of its parent's look or size.

A dialog is a page, not a Popup: any page can open it, and it blocks
what is under it.
