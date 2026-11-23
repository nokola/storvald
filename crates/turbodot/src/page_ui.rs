//! What a page's code writes into its page and reads from it.

use std::fmt::Write as _;

use gogo_ui::runtime::{Item, ItemKey, Resolved};
use gogo_ui::{EventList, Kind, NO_NODE, NO_STR, NodeId, Rect};

use crate::pages::{CodePicture, PageUi};

/// Distance one notch of a mouse wheel moves a scrolling box, in epx.
const WHEEL_NOTCH_EPX: f32 = 48.0;

impl PageUi {
    /// Node the page's file names `name`.
    ///
    /// WARNING: panics when page has no such node. Page built with the
    /// program has its nodes as `ui::<file>::NAMES`, which cannot name a
    /// missing one.
    pub fn node(&self, name: &str) -> NodeId {
        self.page.find(name).unwrap_or_else(|| panic!("the page has no node named `{name}`"))
    }

    /// Writes text `node` shows. Same text written again changes nothing.
    pub fn set_text(&mut self, node: NodeId, text: &str) {
        self.runtime.set_text(&mut self.page, node, text);
    }

    /// Text `node` shows: last `set_text`, else `text=` from its file.
    pub fn text_of(&self, node: NodeId) -> Option<&str> {
        self.runtime.text_of(&self.page, node)
    }

    /// Shows a picture made in code in Image `node`. `rgba`: `width` x
    /// `height` pixels, four bytes each (R, G, B, A; straight alpha,
    /// sRGB), row by row from the top.
    ///
    /// Needed for what no file holds: a thumbnail of user's work, a
    /// preview. Callable anywhere: `loaded`, `show`, a handler. It needs
    /// no GPU, so a page that calls it runs in a test.
    ///
    /// Layout has the size from program's next frame. Pixels reach the
    /// screen at the next frame drawn on a GPU; with no GPU only the
    /// size shows.
    ///
    /// Page keeps a copy of the bytes for this node: next picture of the
    /// same size for the same node allocates nothing. Picture that
    /// changes every frame: draw it on the GPU instead
    /// (`Page::gpu_draw_screen`), which copies nothing.
    ///
    /// WARNING: panics when `rgba` is not `width * height * 4` bytes long.
    pub fn set_picture(&mut self, node: NodeId, width: u32, height: u32, rgba: &[u8]) {
        let byte_count: usize = width as usize * height as usize * 4;
        assert_eq!(rgba.len(), byte_count, "set_picture: {width} x {height} pixels are {byte_count} bytes (4 a pixel, R G B A), and `rgba` holds {}", rgba.len());
        let at: usize = match self.code_pictures.iter().position(|picture| picture.node == node) {
            Some(at) => at,
            None => {
                self.code_pictures.push(CodePicture { node, key: None, width: 0, height: 0, rgba: Vec::new(), is_size_new: true, is_uploaded: false });
                self.code_pictures.len() - 1
            }
        };
        let picture: &mut CodePicture = &mut self.code_pictures[at];
        picture.width = width;
        picture.height = height;
        picture.rgba.clear();
        picture.rgba.extend_from_slice(rgba);
        picture.is_size_new = true;
        picture.is_uploaded = false;
    }

    /// Shows or hides `node`. Hidden node takes no space and no touch.
    pub fn set_visible(&mut self, node: NodeId, is_visible: bool) {
        self.runtime.set_visible(&mut self.page, node, is_visible);
    }

    /// Writes value of a Slider or of a control that has one.
    pub fn set_value(&mut self, control: NodeId, value: f32) {
        self.runtime.set_value(&mut self.page, control, value);
    }

    /// Hands a `repeat` its rows, one per item, in order of `items`. Row
    /// whose key stays keeps its node.
    ///
    /// Returns gogo's refusal, with its code, when an item does not fit
    /// the file's rows. Rows on screen then stay as they were.
    pub fn set_items(&mut self, repeat: NodeId, items: &[Item]) -> Result<(), String> {
        self.runtime.set_items(&mut self.page, repeat, items)
    }

    /// Rows of a `repeat`, in the order they show.
    pub fn rows(&self, repeat: NodeId) -> Vec<NodeId> {
        self.runtime.items(&self.page, repeat)
    }

    /// Key of the row `node` is in. `None`: node outside a row.
    ///
    /// Needed to tell which item an event belongs to: event names a node,
    /// key names the item.
    pub fn row_key(&self, node: NodeId) -> Option<&ItemKey> {
        self.runtime.item_key(&self.page, node)
    }

    /// Writes which rows of a List are selected.
    ///
    /// Returns true when selection changed, or gogo's refusal: List with
    /// `select=off` has no selection, `select=one` takes one key.
    pub fn set_selected(&mut self, list: NodeId, keys: &[ItemKey]) -> Result<bool, String> {
        self.runtime.set_selected(&self.page, list, keys)
    }

    /// Keys of the rows a List has selected.
    pub fn selected_keys(&self, list: NodeId) -> Vec<ItemKey> {
        self.runtime.selected_keys(&self.page, list)
    }

    /// Writes how far content of a Scroll or List has moved: [left, up] in
    /// epx. gogo holds the far end on next frame.
    pub fn set_scroll(&mut self, scroll: NodeId, offset: [f32; 2]) {
        self.runtime.set_scroll(scroll, offset);
    }

    /// How far content of a Scroll or List has moved: [left, up] in epx.
    pub fn scroll_of(&self, scroll: NodeId) -> [f32; 2] {
        self.runtime.scroll_of(scroll)
    }

    /// Where `node` sat in last frame.
    pub fn rect(&self, node: NodeId) -> Rect {
        self.kept.rects()[node.index()]
    }

    /// Page of last frame as text: one line per node that shows, with
    /// kind, name and rectangle, children indented under their parent.
    ///
    /// Needed so a layout is checked by numbers ("list is 260 wide, left
    /// of the text") where a picture has to be judged by eye.
    ///
    /// Returns empty text before first frame. A line reads
    /// `Label title x=16 y=12 w=228 h=28`.
    pub fn outline(&self) -> String {
        let mut out: String = String::new();
        if !self.kept.is_laid_out() {
            return out;
        }
        let look: &Resolved = self.kept.look();
        let count: usize = self.page.nodes.len();
        let mut children: Vec<Vec<NodeId>> = vec![Vec::new(); count];
        let mut roots: Vec<NodeId> = Vec::new();
        for i in 0..count {
            let node: NodeId = NodeId::from_index(i);
            if look.gone(&self.page, node) {
                continue;
            }
            match look.placed_parent(&self.page, node) {
                parent if parent == NO_NODE => roots.push(node),
                parent => children[parent.index()].push(node),
            }
        }
        // Each (node, depth) still to print; last one is printed next.
        let mut todo: Vec<(NodeId, usize)> = roots.into_iter().rev().map(|n| (n, 0)).collect();
        while let Some((node, depth)) = todo.pop() {
            let n = &self.page.nodes[node.index()];
            let r: Rect = self.rect(node);
            let name: &str = if n.name == NO_STR { "-" } else { self.page.str(n.name) };
            let _ = writeln!(out, "{:indent$}{} {} x={} y={} w={} h={}", "", n.kind.word(), name, r.x.round(), r.y.round(), r.w.round(), r.h.round(), indent = depth * 2);
            todo.extend(children[node.index()].iter().rev().map(|c| (*c, depth + 1)));
        }
        out
    }

    /// Mouse wheel turned by `notches` at (`x`, `y`). Positive count: turn
    /// away from user.
    ///
    /// Needed because gogo pans a `Scroll` and a `List` by touch only, and
    /// on a PC a list that does not follow the wheel reads as broken.
    ///
    /// `Slider` or `Toggle` under mouse takes the wheel first, as gogo's
    /// `Input::wheel` has it. Otherwise nearest `Scroll` or `List` around
    /// the node under mouse moves up or down. gogo holds the far end on
    /// next frame.
    ///
    /// Returns events of a control that took the wheel.
    pub fn wheel(&mut self, x: f32, y: f32, notches: f32) -> EventList {
        if !self.kept.is_laid_out() || notches == 0.0 {
            return EventList::new();
        }
        let events: EventList = self.input.wheel(&self.page, &mut self.runtime, self.kept.frame(), x, y, notches > 0.0);
        if !events.is_empty() {
            return events;
        }
        let look: &Resolved = self.kept.look();
        let mut node: NodeId = match gogo_ui::hit(&self.page, self.kept.frame(), x, y, true) {
            Some(node) => node,
            None => return events,
        };
        while node != NO_NODE {
            if matches!(self.page.nodes[node.index()].kind, Kind::Scroll | Kind::List) {
                let [left, top]: [f32; 2] = self.runtime.scroll_of(node);
                self.runtime.set_scroll(node, [left, (top - notches * WHEEL_NOTCH_EPX).max(0.0)]);
                break;
            }
            node = look.placed_parent(&self.page, node);
        }
        events
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn page_with_an_image() -> (PageUi, NodeId) {
        let compiled = gogo_ui::compile("View root { Image made size=hug }", &|_| None);
        assert!(!gogo_ui::has_errors(&compiled.diags), "{:?}", compiled.diags);
        let ui: PageUi = PageUi::new(compiled.page);
        let node: NodeId = ui.node("made");
        (ui, node)
    }

    /// Second picture of same size for same node goes into memory page
    /// already holds: no allocation. Other node gets its own.
    #[test]
    fn a_second_picture_for_a_node_takes_no_new_memory() {
        let (mut ui, node) = page_with_an_image();
        ui.set_picture(node, 2, 1, &[1, 2, 3, 4, 5, 6, 7, 8]);
        let memory: *const u8 = ui.code_pictures[0].rgba.as_ptr();
        ui.code_pictures[0].is_size_new = false;
        ui.code_pictures[0].is_uploaded = true;
        ui.set_picture(node, 1, 2, &[9, 9, 9, 9, 8, 8, 8, 8]);
        assert_eq!(ui.code_pictures.len(), 1, "one picture per node");
        let picture: &CodePicture = &ui.code_pictures[0];
        assert_eq!(picture.rgba.as_ptr(), memory, "same memory");
        assert_eq!((picture.width, picture.height, &picture.rgba[..]), (1, 2, &[9, 9, 9, 9, 8, 8, 8, 8][..]));
        assert!(picture.is_size_new && !picture.is_uploaded, "to place and to upload again");
    }

    /// Bytes that do not make `width` x `height` pixels stop program with
    /// the counts, where the call is: a picture drawn skewed says nothing.
    #[test]
    #[should_panic(expected = "set_picture: 2 x 2 pixels are 16 bytes")]
    fn a_picture_with_the_wrong_count_of_bytes_is_refused() {
        let (mut ui, node) = page_with_an_image();
        ui.set_picture(node, 2, 2, &[0; 12]);
    }
}
