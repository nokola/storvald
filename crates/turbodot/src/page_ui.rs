//! What a page's code writes into its page and reads from it.

use std::fmt::Write as _;

use gogo_ui::runtime::{Item, ItemKey, Resolved, Runtime};
use gogo_ui::{Event, Input, KeptFrame, Kind, NO_NODE, NO_STR, NodeId, Page, Rect};

use crate::pages::HostedPage;

/// How far one notch of a mouse wheel moves a scrolling box, in epx.
pub(crate) const WHEEL_NOTCH_EPX: f32 = 48.0;

/// The page of `kept`'s frame as text: one line per node that shows, with
/// its kind, its name and its rectangle, children indented under their
/// parent.
///
/// Returns an empty text before the first frame.
pub(crate) fn outline(page: &Page, kept: &KeptFrame) -> String {
    let mut out: String = String::new();
    if !kept.is_laid_out() {
        return out;
    }
    let look: &Resolved = kept.look();
    let count: usize = page.nodes.len();
    let mut children: Vec<Vec<NodeId>> = vec![Vec::new(); count];
    let mut roots: Vec<NodeId> = Vec::new();
    for i in 0..count {
        let node: NodeId = NodeId::from_index(i);
        if look.gone(page, node) {
            continue;
        }
        match look.placed_parent(page, node) {
            parent if parent == NO_NODE => roots.push(node),
            parent => children[parent.index()].push(node),
        }
    }
    // Each (node, depth) still to print; the last one is printed next.
    let mut todo: Vec<(NodeId, usize)> = roots.into_iter().rev().map(|n| (n, 0)).collect();
    while let Some((node, depth)) = todo.pop() {
        let n = &page.nodes[node.index()];
        let r: Rect = kept.rects()[node.index()];
        let name: &str = if n.name == NO_STR { "-" } else { page.str(n.name) };
        let _ = writeln!(out, "{:indent$}{} {} x={} y={} w={} h={}", "", n.kind.word(), name, r.x.round(), r.y.round(), r.w.round(), r.h.round(), indent = depth * 2);
        todo.extend(children[node.index()].iter().rev().map(|c| (*c, depth + 1)));
    }
    out
}

/// The mouse wheel turned by `notches` at (`x`, `y`); a positive count is
/// a turn away from the user.
///
/// Needed because gogo pans a `Scroll` and a `List` by touch only, and on
/// a PC a list that does not follow the wheel reads as broken.
///
/// A `Slider` or `Toggle` under the mouse takes the wheel first, as gogo's
/// `Input::wheel` has it. Otherwise the nearest `Scroll` or `List` around
/// the node under the mouse moves up or down. The far end is held by gogo
/// on the next frame.
///
/// Returns the events of a control that took the wheel.
pub(crate) fn wheel(page: &Page, runtime: &mut Runtime, input: &mut Input, kept: &KeptFrame, x: f32, y: f32, notches: f32) -> Vec<Event> {
    if !kept.is_laid_out() || notches == 0.0 {
        return Vec::new();
    }
    let events: Vec<Event> = input.wheel(page, runtime, kept.frame(), x, y, notches > 0.0);
    if !events.is_empty() {
        return events;
    }
    let look: &Resolved = kept.look();
    let mut node: NodeId = match gogo_ui::hit(page, kept.frame(), x, y, true) {
        Some(node) => node,
        None => return events,
    };
    while node != NO_NODE {
        if matches!(page.nodes[node.index()].kind, Kind::Scroll | Kind::List) {
            let [left, top]: [f32; 2] = runtime.scroll_of(node);
            runtime.set_scroll(node, [left, (top - notches * WHEEL_NOTCH_EPX).max(0.0)]);
            break;
        }
        node = look.placed_parent(page, node);
    }
    events
}

impl HostedPage {
    /// The node the page's file names `name`.
    ///
    /// WARNING: panics when the page has no such node. A page built with
    /// the program has its nodes as `ui::<file>::NAMES`, which cannot name
    /// a missing one.
    pub fn node(&self, name: &str) -> NodeId {
        self.page.find(name).unwrap_or_else(|| panic!("the page has no node named `{name}`"))
    }

    /// Writes the text `node` shows. The same text written again changes
    /// nothing.
    pub fn set_text(&mut self, node: NodeId, text: &str) {
        self.runtime.set_text(&mut self.page, node, text);
    }

    /// The text `node` shows: the last `set_text`, else the `text=` its
    /// file says.
    pub fn text_of(&self, node: NodeId) -> Option<&str> {
        self.runtime.text_of(&self.page, node)
    }

    /// Shows or hides `node`. A hidden node takes no space and no touch.
    pub fn set_visible(&mut self, node: NodeId, is_visible: bool) {
        self.runtime.set_visible(&mut self.page, node, is_visible);
    }

    /// Writes the value of a Slider or of a control that has one.
    pub fn set_value(&mut self, control: NodeId, value: f32) {
        self.runtime.set_value(&mut self.page, control, value);
    }

    /// Hands a `repeat` its rows, one per item, in the order of `items`. A
    /// row whose key stays keeps its node.
    ///
    /// Returns gogo's refusal, with its code, when an item does not fit
    /// the rows of the file. The rows on screen then stay as they were.
    pub fn set_items(&mut self, repeat: NodeId, items: &[Item]) -> Result<(), String> {
        self.runtime.set_items(&mut self.page, repeat, items)
    }

    /// The rows of a `repeat`, in the order they show.
    pub fn rows(&self, repeat: NodeId) -> Vec<NodeId> {
        self.runtime.items(&self.page, repeat)
    }

    /// The key of the row `node` is in, `None` for a node outside a row.
    ///
    /// Needed to tell which item an event belongs to: the event names a
    /// node, and the key names the item.
    pub fn row_key(&self, node: NodeId) -> Option<&ItemKey> {
        self.runtime.item_key(&self.page, node)
    }

    /// Writes which rows of a List are selected.
    ///
    /// Returns whether the selection changed, or gogo's refusal: a List
    /// with `select=off` has no selection, and `select=one` takes one key.
    pub fn set_selected(&mut self, list: NodeId, keys: &[ItemKey]) -> Result<bool, String> {
        self.runtime.set_selected(&self.page, list, keys)
    }

    /// The keys of the rows a List has selected.
    pub fn selected_keys(&self, list: NodeId) -> Vec<ItemKey> {
        self.runtime.selected_keys(&self.page, list)
    }

    /// Writes how far the content of a Scroll or a List has moved: [left,
    /// up] in epx. gogo holds the far end on the next frame.
    pub fn set_scroll(&mut self, scroll: NodeId, offset: [f32; 2]) {
        self.runtime.set_scroll(scroll, offset);
    }

    /// How far the content of a Scroll or a List has moved: [left, up] in
    /// epx.
    pub fn scroll_of(&self, scroll: NodeId) -> [f32; 2] {
        self.runtime.scroll_of(scroll)
    }

    /// Where `node` sat in the last frame.
    pub fn rect(&self, node: NodeId) -> Rect {
        self.kept.rects()[node.index()]
    }

    /// The page of the last frame as text: one line per node that shows,
    /// with its kind, its name and its rectangle, children indented under
    /// their parent.
    ///
    /// Needed so a layout is checked by numbers ("the list is 260 wide and
    /// left of the text") where a picture has to be judged by eye.
    ///
    /// Returns an empty text before the first frame. A line reads
    /// `Label title x=16 y=12 w=228 h=28`.
    pub fn outline(&self) -> String {
        outline(&self.page, &self.kept)
    }

    /// The mouse wheel turned by `notches` at (`x`, `y`); a positive count
    /// is a turn away from the user. A control under the mouse takes it
    /// first, else the nearest `Scroll` or `List` moves.
    ///
    /// Returns the events of a control that took the wheel.
    pub fn wheel(&mut self, x: f32, y: f32, notches: f32) -> Vec<Event> {
        wheel(&self.page, &mut self.runtime, &mut self.input, &self.kept, x, y, notches)
    }
}
