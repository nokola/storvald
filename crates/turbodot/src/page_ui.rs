//! What a page's code writes into its page and reads from it.

use std::fmt::Write as _;

use gogo_ui::runtime::{Item, ItemKey, Resolved};
use gogo_ui::{CommandSurface, EventList, Kind, Mods, NO_NODE, NO_STR, NodeId, NodeName, PictureSurface, Rect};

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

    /// Shows a picture made in code in `surface`, a `Surface
    /// draw=picture` of page's file. `rgba`: `width` x `height` pixels,
    /// four bytes each (R, G, B, A; straight alpha, sRGB), row by row
    /// from the top.
    ///
    /// Needed for what no file holds: a thumbnail of user's work, a
    /// preview. Callable anywhere: `loaded`, `show`, a handler. It needs
    /// no GPU, so a page that calls it runs in a test.
    ///
    /// The Surface keeps the size its file gives it: the picture is
    /// fitted into it by the Surface's `fit=`. Pixels reach the screen
    /// at the next frame drawn on a GPU.
    ///
    /// Page takes `rgba` over, with no copy, and keeps it while it
    /// lives: a renderer made new gets the pixels again. Picture that
    /// changes every frame: draw it into a texture on the GPU instead
    /// (`set_picture_texture`), which keeps no pixels.
    ///
    /// WARNING: panics when `rgba` is not `width * height * 4` bytes long.
    ///
    /// Returns buffer of the picture `surface` showed before, empty for
    /// its first picture. Page that fills it for its next picture
    /// allocates nothing.
    pub fn set_picture(&mut self, surface: PictureSurface, width: u32, height: u32, rgba: Vec<u8>) -> Vec<u8> {
        let node: NodeId = surface.node();
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
        picture.is_size_new = true;
        picture.is_uploaded = false;
        std::mem::replace(&mut picture.rgba, rgba)
    }

    /// Shows a texture of page's own in `surface`, a `Surface
    /// draw=picture` of page's file: the renderer samples it where the
    /// Surface is, in the pass that draws every page. No copy, no pass
    /// of its own.
    ///
    /// Needed for a picture page draws on the GPU: a painting, a game's
    /// world drawn into a texture. Called from `Page::gpu_prepare`, once
    /// or every frame: the same texture again costs one compare, and
    /// another texture (page draws into two in turn) takes its place.
    ///
    /// Page keeps a handle to the texture while it lives, so a renderer
    /// made new gets it again.
    #[cfg(feature = "gpu")]
    pub fn set_picture_texture(&mut self, surface: PictureSurface, texture: gogo_wgpu::Tex_Image_Straight_Srgb_4xU8_RGBA<'_>) {
        let node: NodeId = surface.node();
        match self.picture_textures.iter_mut().find(|picture| picture.node == node) {
            Some(picture) => {
                if picture.texture != *texture.RGBA {
                    picture.texture = texture.RGBA.clone();
                    picture.is_new = true;
                }
            }
            None => self.picture_textures.push(crate::pages::PictureTexture { node, key: None, texture: texture.RGBA.clone(), is_new: true }),
        }
    }

    /// Format of the color target that the pass which draws the pages
    /// writes into, as `Page::gpu_prepare` finds it.
    ///
    /// Needed by a page with a `Surface draw=gpu`: the pipeline of its
    /// own draw calls is made for this format, in `gpu_prepare`, before
    /// `draw_surface_gpu` uses it. A window and a picture of the program
    /// (`Gpu::draw_program`) may have two formats: a page compares, and
    /// makes its pipeline again for another one.
    ///
    /// Returns `None` before page's first GPU frame.
    #[cfg(feature = "gpu")]
    pub fn target_format(&self) -> Option<wgpu::TextureFormat> {
        self.target_format
    }

    /// Format of the depth buffer of the pass which draws the pages, as
    /// `Page::gpu_prepare` finds it. `Some` in a program with a `Surface
    /// draw=gpu, depth=true` in any of its pages.
    ///
    /// Needed by a page with a `Surface draw=gpu`, with `depth=true` or
    /// without: with `Some`, the pipeline of its own draw calls names
    /// this format in its `depth_stencil`. The GPU refuses a pipeline
    /// with no depth format in a pass that has a depth buffer.
    ///
    /// Returns `None` when the pass has no depth buffer, and before
    /// page's first GPU frame.
    #[cfg(feature = "gpu")]
    pub fn depth_format(&self) -> Option<wgpu::TextureFormat> {
        self.depth_format
    }

    /// Says what `surface`, a `Surface draw=commands` of page's file,
    /// shows is out of date: program calls page's `draw_surface` for it
    /// before the next frame is drawn.
    ///
    /// Needed because the Surface's draw commands are kept from frame
    /// to frame: a score that rose, a board after a move.
    pub fn surface_changed(&mut self, surface: CommandSurface) {
        self.runtime.surface_changed(&mut self.page, surface);
    }

    /// Writes `capture_mouse=` on `surface`, a Surface of page's file:
    /// true when play starts, false at a pause.
    ///
    /// While it is true, page is the top page and the window has the
    /// keyboard, the cursor is hidden and held in place, and everything
    /// the mouse does goes to `surface`: a move with no button down to
    /// its `on:hover=`, a move with one down and each button to its
    /// `on:pointer=`, the wheel to its `on:wheel=`. `dx` and `dy` of a
    /// sample are then the mouse's own counts.
    ///
    /// WARNING: with no cursor the user cannot press a button of the
    /// page. Give them a key that ends it: Esc is the back request
    /// (`Page::back`).
    #[inline]
    pub fn set_capture_mouse(&mut self, surface: impl NodeName, is_captured: bool) {
        self.set_capture_mouse_by_id(surface.node(), is_captured)
    }

    /// What `set_capture_mouse` does, by the id of the node.
    fn set_capture_mouse_by_id(&mut self, surface: NodeId, is_captured: bool) {
        self.runtime.set_capture_mouse(surface, is_captured);
    }

    /// Shows or hides `node`. Hidden node takes no space and no touch.
    #[inline]
    pub fn set_visible(&mut self, node: impl NodeName, is_visible: bool) {
        self.set_visible_by_id(node.node(), is_visible)
    }

    /// What `set_visible` does, by the id of the node.
    fn set_visible_by_id(&mut self, node: NodeId, is_visible: bool) {
        self.runtime.set_visible(&mut self.page, node, is_visible);
    }

    /// Checks or unchecks a Check, a Toggle or a Radio, as a tap would
    /// leave it. Raises no event.
    ///
    /// Needed by a page that closes a menu after a pick: the menu is a
    /// `Popup` the file shows while its Check is `selected`.
    pub fn set_checked(&mut self, control: NodeId, is_checked: bool) {
        self.runtime.set_state(control, gogo_ui::runtime::SELECTED, is_checked);
    }

    /// True while a Check, a Toggle or a Radio is checked.
    pub fn is_checked(&self, control: NodeId) -> bool {
        self.runtime.has(control, gogo_ui::runtime::SELECTED)
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
    #[inline]
    pub fn row_key(&self, node: impl NodeName) -> Option<&ItemKey> {
        self.row_key_by_id(node.node())
    }

    /// What `row_key` does, by the id of the node.
    fn row_key_by_id(&self, node: NodeId) -> Option<&ItemKey> {
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
    #[inline]
    pub fn rect(&self, node: impl NodeName) -> Rect {
        self.rect_by_id(node.node())
    }

    /// What `rect` does, by the id of the node.
    fn rect_by_id(&self, node: NodeId) -> Rect {
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
    /// Returns events of a control that took the wheel: none when a
    /// `Scroll` or a `List` moved, or when mouse is over a node that
    /// takes input and has nothing to turn. `None`: page has nothing
    /// for the wheel there, and nothing moved.
    pub fn wheel(&mut self, x: f32, y: f32, notches: f32, mods: Mods) -> Option<EventList> {
        if !self.kept.is_laid_out() || notches == 0.0 {
            return None;
        }
        let events: EventList = self.input.wheel(&self.page, &mut self.runtime, self.kept.frame(), x, y, notches, mods);
        if !events.is_empty() {
            return Some(events);
        }
        let look: &Resolved = self.kept.look();
        let mut node: NodeId = gogo_ui::hit(&self.page, self.kept.frame(), x, y, true)?;
        while node != NO_NODE {
            if matches!(self.page.nodes[node.index()].kind, Kind::Scroll | Kind::List) {
                let [left, top]: [f32; 2] = self.runtime.scroll_of(node);
                self.runtime.set_scroll(node, [left, (top - notches * WHEEL_NOTCH_EPX).max(0.0)]);
                return Some(events);
            }
            node = look.placed_parent(&self.page, node);
        }
        // A press here would be this page's: so is the turn.
        self.hit(x, y).map(|_| events)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn page_with_a_picture_surface() -> (PageUi, PictureSurface) {
        let compiled = gogo_ui::compile("View root { Surface made draw=picture, width=40, height=40 }", &|_| None);
        assert!(!gogo_ui::has_errors(&compiled.diags), "{:?}", compiled.diags);
        let ui: PageUi = PageUi::new(compiled.page);
        let surface: PictureSurface = PictureSurface::find(&ui.page, "made").expect("a Surface draw=picture");
        (ui, surface)
    }

    /// Page takes picture's bytes over with no copy, and hands back
    /// buffer of the picture before: page that fills it for its next
    /// picture allocates nothing.
    #[test]
    fn a_page_takes_a_pictures_bytes_over_and_hands_back_the_buffer_before() {
        let (mut ui, node) = page_with_a_picture_surface();
        let first: Vec<u8> = vec![1, 2, 3, 4, 5, 6, 7, 8];
        let first_memory: *const u8 = first.as_ptr();
        assert!(ui.set_picture(node, 2, 1, first).is_empty(), "node's first picture replaces none");
        assert_eq!(ui.code_pictures[0].rgba.as_ptr(), first_memory, "page holds caller's own buffer: no copy");
        ui.code_pictures[0].is_size_new = false;
        ui.code_pictures[0].is_uploaded = true;
        let second: Vec<u8> = vec![9, 9, 9, 9, 8, 8, 8, 8];
        let second_memory: *const u8 = second.as_ptr();
        let handed_back: Vec<u8> = ui.set_picture(node, 1, 2, second);
        assert_eq!(handed_back.as_ptr(), first_memory, "first picture's buffer comes back");
        assert_eq!(ui.code_pictures.len(), 1, "one picture per node");
        let picture: &CodePicture = &ui.code_pictures[0];
        assert_eq!(picture.rgba.as_ptr(), second_memory, "page holds the second buffer: no copy");
        assert_eq!((picture.width, picture.height, &picture.rgba[..]), (1, 2, &[9, 9, 9, 9, 8, 8, 8, 8][..]));
        assert!(picture.is_size_new && !picture.is_uploaded, "to place and to upload again");
    }

    /// Bytes that do not make `width` x `height` pixels stop program with
    /// the counts, where the call is: a picture drawn skewed says nothing.
    #[test]
    #[should_panic(expected = "set_picture: 2 x 2 pixels are 16 bytes")]
    fn a_picture_with_the_wrong_count_of_bytes_is_refused() {
        let (mut ui, node) = page_with_a_picture_surface();
        ui.set_picture(node, 2, 2, vec![0; 12]);
    }
}
