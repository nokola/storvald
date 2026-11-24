//! Two changes the retained layout (`LayoutCache`) answers without
//! placing a box: a Scroll or List whose offset moved, and a text
//! rewritten to one of the same size. Every step's rects must equal a
//! plain `layout` bit for bit, so the savings cannot change what shows.
//!
//! Needed because both happen at every frame of a common gesture. A pan
//! of a 100-row list placed 407 boxes and made 422 allocations a step;
//! a number rewritten on a fixed-width label beside a dragged slider
//! placed 109 of its page's 115 boxes.
use gogo_ui::layout::LayoutStats;
use gogo_ui::runtime::{Item, ItemValue, Resolved, Runtime, resolve_animated};
use gogo_ui::{FixedAdvance, NodeId, Page, PropKey, Rect, Value, compile, has_errors, layout};

const VIEWPORT: Rect = Rect { x: 0.0, y: 0.0, w: 400.0, h: 800.0 };

/// A Scroll that pans both ways and holds a Scroll that pans sideways, a
/// box shown in part, a box that is gone; a List of rows under it; and a
/// line of labels beside them, outside any Scroll.
const PAGE: &str = "View page {
    Column body spacing=4, width=fill, height=fill {
        Label head text=\"Head\"
        Scroll outer allow=both, width=fill, height=120 {
            Column inner spacing=3 {
                Label first text=\"first row\"
                Panel part reveal_y=0.5 { Label inPart text=\"half shown\" }
                Scroll strip allow=horizontal, width=100, height=30 {
                    Row words { Label one text=\"one long strip of words\"; Label two text=\"two\" }
                }
                Rectangle hidden visibility=gone, width=10, height=10
                Label last text=\"last row\"
            }
        }
        List list height=140, spacing=2 {
            Label fixed text=\"above the rows\"
            empty { Label none text=\"no rows\" }
            repeat rows by $item.id {
                ListItem { Row spacing=4 { Label title text=$item.title; Label count text=\"7\", width=40 } }
            }
        }
        Row line spacing=6 {
            Label name text=\"Size\", width=80
            Label value text=\"41\", width=40
            Label loose text=\"41\"
            Label folded text=\"41\", width=40, wrap=true
        }
    }
}
";

fn record(id: i64, title: &str) -> Item {
    Item::Record(vec![("id".to_string(), ItemValue::Int(id)), ("title".to_string(), ItemValue::Str(title.to_string()))])
}

struct Host {
    page: Page,
    rt: Runtime,
}

impl Host {
    /// The page with `row_count` rows in its list, after its first frame.
    fn new(row_count: i64) -> Host {
        let c = compile(PAGE, &|_| None);
        assert!(!has_errors(&c.diags), "{:#?}", c.diags);
        let rt = Runtime::new(&c.page);
        let mut host = Host { page: c.page, rt };
        host.set_rows(row_count);
        let first: LayoutStats = host.frame("the first frame");
        assert!(first.full, "the first frame lays the whole page out");
        host
    }

    fn set_rows(&mut self, row_count: i64) {
        let repeat: NodeId = self.id("rows");
        let items: Vec<Item> = (1..=row_count).map(|i| record(i, &format!("row {i}"))).collect();
        self.rt.set_items(&mut self.page, repeat, &items).unwrap();
    }

    fn id(&self, name: &str) -> NodeId {
        self.page.find(name).unwrap_or_else(|| panic!("no node `{name}`"))
    }

    /// Resolve and lay out retained, check the rects against a plain
    /// `layout`, and hand back what the retained road did.
    fn frame(&mut self, what: &str) -> LayoutStats {
        let res: Resolved = resolve_animated(&self.page, &mut self.rt);
        let kept: Vec<Rect> = self.rt.layout_retained(&self.page, VIEWPORT, &FixedAdvance).to_vec();
        let fresh: Vec<Rect> = layout(&self.page, &res, VIEWPORT, &FixedAdvance);
        if let Some(i) = (0..fresh.len()).find(|i| kept[*i] != fresh[*i]) {
            panic!("{what}: node {i} (`{}`) is at {:?}, a full layout puts it at {:?}", self.page.str(self.page.nodes[i].name), kept[i], fresh[i]);
        }
        self.rt.debug_layout_stats()
    }

    fn rect(&mut self, name: &str) -> Rect {
        let node: NodeId = self.id(name);
        self.rt.layout_retained(&self.page, VIEWPORT, &FixedAdvance)[node.index()]
    }

    fn rect_of(&mut self, node: NodeId) -> Rect {
        self.rt.layout_retained(&self.page, VIEWPORT, &FixedAdvance)[node.index()]
    }

    /// The label of a written width in the list's first row: the second
    /// label of the Row the row's ListItem holds.
    fn count_label_of_first_row(&self) -> NodeId {
        let first_row: NodeId = self.rt.items(&self.page, self.id("rows"))[0];
        let line: NodeId = self.page.children(first_row).next().expect("a row holds its Row");
        self.page.children(line).nth(1).expect("the Row holds two labels")
    }
}

#[test]
fn a_pan_of_a_list_places_no_box_and_moves_its_rows() {
    let mut host = Host::new(30);
    let list: NodeId = host.id("list");
    let repeat: NodeId = host.id("rows");
    let third_row: NodeId = host.rt.items(&host.page, repeat)[2];
    let at_rest: Rect = host.rect_of(third_row);
    let fixed_at_rest: Rect = host.rect("fixed");
    // A sideways offset on a List that pans down moves nothing sideways.
    for (x, y) in [(0.0, 12.0), (0.0, 24.5), (5.0, 37.25), (0.0, 0.1 + 0.2), (0.0, 0.0)] {
        host.rt.set_scroll(list, [x, y]);
        let st: LayoutStats = host.frame("a pan of the list");
        assert!(!st.full, "a pan laid the whole page out");
        assert_eq!((st.placed, st.measured), (0, 0), "a pan placed {} boxes and measured {}", st.placed, st.measured);
        assert!(st.moved > 90, "a pan moved {} boxes; the list holds 30 rows of four boxes", st.moved);
        let row: Rect = host.rect_of(third_row);
        assert_eq!((row.x, row.y, row.w, row.h), (at_rest.x, at_rest.y - y, at_rest.w, at_rest.h), "the third row at offset {y}");
        assert_eq!(host.rect("fixed").y, fixed_at_rest.y - y, "what a List holds above its rows pans with them");
    }
}

#[test]
fn a_pan_of_a_scroll_inside_a_scroll_places_no_box() {
    let mut host = Host::new(4);
    let (outer, strip): (NodeId, NodeId) = (host.id("outer"), host.id("strip"));
    let two_at_rest: Rect = host.rect("two");
    let steps: [([f32; 2], [f32; 2]); 5] = [([0.0, 9.0], [0.0, 0.0]), ([3.5, 9.0], [0.0, 0.0]), ([3.5, 9.0], [11.25, 0.0]), ([0.7, 20.3], [31.1, 4.0]), ([0.0, 0.0], [0.0, 0.0])];
    for (outer_offset, strip_offset) in steps {
        host.rt.set_scroll(outer, outer_offset);
        host.rt.set_scroll(strip, strip_offset);
        let st: LayoutStats = host.frame("a pan of a Scroll and of the Scroll it holds");
        assert!(!st.full && st.placed == 0 && st.measured == 0, "a pan: {st:?}");
        assert!(st.moved > 0, "a pan moved nothing: {st:?}");
        // The strip pans sideways only: its offset down is not a pan.
        let two: Rect = host.rect("two");
        assert_eq!(two.y, two_at_rest.y - outer_offset[1], "a label in the strip, down");
        assert!((two.x - (two_at_rest.x - outer_offset[0] - strip_offset[0])).abs() < 0.001, "a label in the strip, sideways: {} from {}", two.x, two_at_rest.x);
    }
}

#[test]
fn a_box_nobody_placed_keeps_the_zero_rect_under_a_pan() {
    let mut host = Host::new(6);
    let repeat: NodeId = host.id("rows");
    let sixth_row: NodeId = host.rt.items(&host.page, repeat)[5];
    // Four rows leave: their boxes stay in the page, shown nowhere.
    host.set_rows(2);
    host.frame("four rows left");
    let (outer, list): (NodeId, NodeId) = (host.id("outer"), host.id("list"));
    host.rt.set_scroll(outer, [0.0, 15.0]);
    host.rt.set_scroll(list, [0.0, 6.0]);
    let st: LayoutStats = host.frame("a pan over a box that is gone and a row that left");
    assert_eq!(st.placed, 0, "a pan placed {} boxes", st.placed);
    assert_eq!(host.rect("hidden"), Rect::ZERO, "a box that is gone, under a Scroll at offset 15");
    assert_eq!(host.rect_of(sixth_row), Rect::ZERO, "a row that left, under a List at offset 6");
    // A box that had a rect loses it while the Scroll above it stands
    // scrolled, and gets it back.
    let first: NodeId = host.id("first");
    host.rt.set_visible(&mut host.page, first, false);
    let st: LayoutStats = host.frame("a label under a scrolled Scroll is gone");
    assert!(!st.full, "a label gone laid the whole page out");
    assert_eq!(host.rect("first"), Rect::ZERO, "a label that is gone, under a Scroll at offset 15");
    host.rt.set_visible(&mut host.page, first, true);
    host.frame("the label is back");
    assert!(host.rect("first").h > 0.0, "the label is back");
}

#[test]
fn a_pan_and_a_change_in_one_frame_agree_with_a_full_layout() {
    let mut host = Host::new(8);
    let (outer, strip, list): (NodeId, NodeId, NodeId) = (host.id("outer"), host.id("strip"), host.id("list"));
    let (first, one, value): (NodeId, NodeId, NodeId) = (host.id("first"), host.id("one"), host.id("value"));
    // A label under the outer Scroll grows while both Scrolls pan.
    host.rt.set_scroll(outer, [2.0, 14.5]);
    host.rt.set_scroll(strip, [8.0, 0.0]);
    host.rt.set_text(&mut host.page, first, "first row, now a good deal longer");
    let st: LayoutStats = host.frame("a longer label and two pans");
    assert!(!st.full && st.placed > 0, "a longer label under a Scroll is placed again: {st:?}");
    // A label in the strip grows while only the Scroll above the strip pans.
    host.rt.set_scroll(outer, [2.0, 30.0]);
    host.rt.set_text(&mut host.page, one, "one");
    host.frame("a shorter label in the strip and a pan above it");
    // A pan beside a change that has nothing to do with it.
    host.rt.set_scroll(list, [0.0, 21.0]);
    host.rt.set_prop(value, PropKey::Width, Value::VNum(64.0));
    host.frame("a pan of the list and a wider label in the line");
    // Rows come and go under a List that stands scrolled.
    host.set_rows(3);
    host.frame("rows left a scrolled list");
    host.set_rows(12);
    host.rt.set_scroll(list, [0.0, 33.0]);
    host.frame("rows came and the list panned");
}

#[test]
fn a_new_text_of_the_same_size_places_no_box() {
    let mut host = Host::new(4);
    let (value, loose, count): (NodeId, NodeId, NodeId) = (host.id("value"), host.id("loose"), host.count_label_of_first_row());
    // The first write gives the label a string of its own (a changed
    // word); the writes after it rewrite that string in place.
    for label in [value, loose, count] {
        host.rt.set_text(&mut host.page, label, "40");
    }
    host.frame("the first write of each label");
    for (label, what) in [(value, "a label of a written width"), (loose, "a label as wide as its text"), (count, "a label in a list's row")] {
        host.rt.set_text(&mut host.page, label, "42");
        let st: LayoutStats = host.frame(what);
        assert!(!st.full, "{what}: a new text laid the whole page out");
        assert_eq!(st.placed, 0, "{what}: a new text of the same size placed {} boxes", st.placed);
        assert!(st.measured >= 1 && st.measured <= 3, "{what}: the label alone is measured again, at each width it was asked before; measured {}", st.measured);
    }
}

#[test]
fn a_new_text_of_another_size_is_placed_again() {
    let mut host = Host::new(4);
    let loose: NodeId = host.id("loose");
    host.rt.set_text(&mut host.page, loose, "40");
    host.frame("the first write");
    let narrow: Rect = host.rect("loose");
    host.rt.set_text(&mut host.page, loose, "40 000 000");
    let st: LayoutStats = host.frame("a longer text on a label as wide as its text");
    assert!(st.placed > 0, "a longer text placed nothing: {st:?}");
    assert!(host.rect("loose").w > narrow.w, "the label did not grow with its text: {:?} from {narrow:?}", host.rect("loose"));
    // More lines on a label of a written width that folds its text: as
    // wide, taller.
    let folded: NodeId = host.id("folded");
    host.rt.set_text(&mut host.page, folded, "40");
    host.frame("the first write of the label that folds");
    let one_line: Rect = host.rect("folded");
    host.rt.set_text(&mut host.page, folded, "40 and a good many more words");
    let st: LayoutStats = host.frame("more lines on a label of a written width");
    assert!(st.placed > 0, "more lines placed nothing: {st:?}");
    let grown: Rect = host.rect("folded");
    assert!(grown.w == one_line.w && grown.h > one_line.h, "the label did not grow by its new lines: {grown:?} from {one_line:?}");
}

#[test]
fn a_text_change_after_a_change_under_a_fixed_box_is_still_seen() {
    // A button with a text of its own holds a box of a written size.
    // A change inside that box stops at the box; the button's text
    // changing next must still move what stands beside the button.
    const NESTED: &str = "View page {
    Row line spacing=6 {
        Button press text=\"go\" { Panel box width=30, height=10 { Label deep text=\"x\" } }
        Label beside text=\"beside\"
    }
}
";
    let c = compile(NESTED, &|_| None);
    assert!(!has_errors(&c.diags), "{:#?}", c.diags);
    let rt = Runtime::new(&c.page);
    let mut host = Host { page: c.page, rt };
    host.frame("the first frame");
    let (press, deep): (NodeId, NodeId) = (host.id("press"), host.id("deep"));
    host.rt.set_text(&mut host.page, press, "go");
    host.rt.set_text(&mut host.page, deep, "y");
    host.frame("the first write of each");
    host.rt.set_text(&mut host.page, deep, "yes, longer");
    let st: LayoutStats = host.frame("a longer label inside the box of a written size");
    assert!(!st.full, "{st:?}");
    let beside: Rect = host.rect("beside");
    host.rt.set_text(&mut host.page, press, "go, a much longer word");
    host.frame("a longer text on the button");
    assert!(host.rect("beside").x > beside.x, "the label beside the button did not move when the button grew: {:?} from {beside:?}", host.rect("beside"));
}
