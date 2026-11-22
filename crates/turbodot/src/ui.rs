//! One gogo page with everything it needs to run, and no window.

use std::cell::RefCell;
use std::path::{Path, PathBuf};

use gogo_text::{EpxMeasure, TextEngine};
use gogo_ui::reload::{Reloader, Sources};
use gogo_ui::runtime::{Item, ItemKey};
use gogo_ui::{Compiled, Draw, Event, Input, KeptFrame, Key, MOUSE_ID, Mods, NodeId, Orientation, Page, Phase, Rect, Runtime, ScreenClass, Touch, compile, compile_file, has_errors};

/// Where the text of a page comes from.
pub enum PageSource<'a> {
    /// The text of a page, compiled when the program starts.
    ///
    /// Needed for a program that ships as one file: it carries its pages
    /// inside it (`include_str!`) and reads nothing from disk.
    Text(&'a str),
    /// A `.gogo` file in a folder, compiled when the program starts.
    ///
    /// Needed while a page is being written: with the cargo feature
    /// `hot-reload` a save of the file shows in the running program.
    /// Without the feature the file is read once.
    File { dir: PathBuf, name: &'a str },
}

/// One gogo page with its state, its text engine and its last frame.
///
/// Needed so an app, a test and an AI agent drive a page the same way: a
/// touch goes in as numbers, events come out as values, and no window or
/// GPU takes part. `run` shows a `Ui` in a window, and `Gpu::draw` draws
/// one to a picture.
///
/// All positions and sizes are in epx, the unit a `.gogo` file is written
/// in. Holds ONE page. Several pages in one window are a
/// `pages::PageStack`, which `run` does not show yet.
pub struct Ui {
    pub page: Page,
    /// Everything that changes while the page lives: states, scroll
    /// positions, running animations, what the app wrote.
    pub rt: Runtime,
    pub input: Input,
    /// Where every node sat in the last frame, and how it looked.
    pub kept: KeptFrame,
    /// The last frame's draw commands, in paint order.
    pub draws: Vec<Draw>,
    pub(crate) text: RefCell<TextEngine>,
    /// Keeps `page` in step with its file. `None` for a page from text.
    reload: Option<Reloader>,
    /// Device pixels per epx.
    epx: f32,
    /// The width and height of the last frame.
    size: (f32, f32),
    is_entered: bool,
}

impl Ui {
    /// Compiles a page and makes it ready for its first frame.
    ///
    /// `fonts` are (key, file bytes) pairs. The first one draws every text
    /// that names no font. gogo ships no font, so a `Ui` needs at least one.
    ///
    /// Returns the compiler's errors, one per line, when the page does not
    /// compile or its file cannot be read.
    pub fn new(source: PageSource<'_>, fonts: &[(&str, Vec<u8>)]) -> Result<Ui, String> {
        let (compiled, followed): (Compiled, Option<(Reloader, &str)>) = match source {
            PageSource::Text(text) => (compile(text, &|_| None), None),
            PageSource::File { dir, name } => (compile_file(&dir.join(name))?, Some((Reloader::files(Sources::dir(dir)), name))),
        };
        if has_errors(&compiled.diags) {
            let lines: Vec<String> = compiled.diags.iter().map(|d| d.to_string()).collect();
            return Err(lines.join("\n"));
        }
        let mut page: Page = compiled.page;
        let rt: Runtime = Runtime::new(&page);
        // This program compiled the page itself and finds its nodes by
        // name, so the reloader takes the page over and may change
        // anything in it.
        let reload: Option<Reloader> = followed.map(|(mut reload, name)| {
            reload.follow(&mut page, name);
            reload
        });
        Ok(Ui { page, rt, input: Input::new(), kept: KeptFrame::default(), draws: Vec::new(), text: RefCell::new(TextEngine::new(fonts)), reload, epx: 1.0, size: (0.0, 0.0), is_entered: false })
    }

    /// The node a `.gogo` file names `name`.
    ///
    /// Needed because an app that asks for a node it wrote into its own
    /// page has no answer to a missing one: a typo in either file should
    /// stop the program at the line that asked, with both names in view.
    ///
    /// WARNING: panics when the page has no such node. `page.find` returns
    /// an `Option` for a node that may be missing.
    pub fn node(&self, name: &str) -> NodeId {
        self.page.find(name).unwrap_or_else(|| panic!("the page has no node named `{name}`"))
    }

    /// Writes the text `node` shows. The same text written again changes
    /// nothing.
    pub fn set_text(&mut self, node: NodeId, text: &str) {
        self.rt.set_text(&mut self.page, node, text);
    }

    /// The text `node` shows: the last `set_text`, else the `text=` its
    /// file says.
    pub fn text_of(&self, node: NodeId) -> Option<&str> {
        self.rt.text_of(&self.page, node)
    }

    /// Shows or hides `node`. A hidden node takes no space and no touch.
    pub fn set_visible(&mut self, node: NodeId, is_visible: bool) {
        self.rt.set_visible(&mut self.page, node, is_visible);
    }

    /// Writes the value of a Slider or of a control that has one.
    pub fn set_value(&mut self, control: NodeId, value: f32) {
        self.rt.set_value(&mut self.page, control, value);
    }

    /// Hands a `repeat` its rows, one per item, in the order of `items`. A
    /// row whose key stays keeps its node.
    ///
    /// Returns gogo's refusal, with its code, when an item does not fit
    /// the rows of the file. The rows on screen then stay as they were.
    pub fn set_items(&mut self, repeat: NodeId, items: &[Item]) -> Result<(), String> {
        self.rt.set_items(&mut self.page, repeat, items)
    }

    /// The rows of a `repeat`, in the order they show.
    pub fn rows(&self, repeat: NodeId) -> Vec<NodeId> {
        self.rt.items(&self.page, repeat)
    }

    /// The key of the row `node` is in, `None` for a node outside a row.
    ///
    /// Needed to tell which item an event belongs to: the event names a
    /// node, and the key names the item.
    pub fn row_key(&self, node: NodeId) -> Option<&ItemKey> {
        self.rt.item_key(&self.page, node)
    }

    /// Writes which rows of a List are selected.
    ///
    /// Returns whether the selection changed, or gogo's refusal: a List
    /// with `select=off` has no selection, and `select=one` takes one key.
    pub fn set_selected(&mut self, list: NodeId, keys: &[ItemKey]) -> Result<bool, String> {
        self.rt.set_selected(&self.page, list, keys)
    }

    /// The keys of the rows a List has selected.
    pub fn selected_keys(&self, list: NodeId) -> Vec<ItemKey> {
        self.rt.selected_keys(&self.page, list)
    }

    /// Writes how far the content of a Scroll or a List has moved: [left,
    /// up] in epx. gogo holds the far end on the next frame.
    pub fn set_scroll(&mut self, scroll: NodeId, offset: [f32; 2]) {
        self.rt.set_scroll(scroll, offset);
    }

    /// How far the content of a Scroll or a List has moved: [left, up] in
    /// epx.
    pub fn scroll_of(&self, scroll: NodeId) -> [f32; 2] {
        self.rt.scroll_of(scroll)
    }

    /// Adds a font file that draws the characters no font of the page has.
    ///
    /// Needed because a character the page's fonts lack draws as an empty
    /// box: Roboto has no arrows, so "a → b" shows a box between the two
    /// letters until a font with arrows is added here.
    ///
    /// Returns false when the file is missing, cannot be read, or holds no
    /// font.
    pub fn add_fallback_font_file(&mut self, path: &Path) -> bool {
        let is_added: bool = self.text.borrow_mut().add_fallback_font_file(path);
        if is_added {
            self.rt.dirty = true;
        }
        is_added
    }

    /// Sets how many device pixels one epx is, for the text.
    ///
    /// Needed because text is shaped in device pixels: on a screen at 150 %
    /// a letter has more pixels, and measured at 100 % it would come out
    /// soft and a little off its place.
    pub fn set_epx(&mut self, epx: f32) {
        if epx != self.epx {
            self.epx = epx;
            self.rt.dirty = true;
        }
    }

    /// Device pixels per epx, as last set.
    pub fn epx(&self) -> f32 {
        self.epx
    }

    /// The width and height of the last frame, (0, 0) before the first.
    pub fn size(&self) -> (f32, f32) {
        self.size
    }

    /// One frame, `dt` seconds after the last: moves the animations, lays
    /// the page out for `width` x `height` and fills `draws`.
    ///
    /// Returns true while something still moves. A host may stop asking
    /// for frames when it returns false.
    pub fn frame(&mut self, width: f32, height: f32, dt: f32) -> bool {
        if (width, height) != self.size {
            self.size = (width, height);
            // Tells the page which `when screen.*` words hold.
            self.rt.set_screen(&self.page, Orientation::from_view(width, height), ScreenClass::from_view(width, height));
        }
        if !self.is_entered {
            self.is_entered = true;
            // Plays each visible node's `on:show` once.
            self.rt.play_entrances(&self.page);
        }
        let mut is_moving: bool = self.rt.tick(&mut self.page, dt);
        if self.kept.is_laid_out() {
            is_moving |= self.input.tick(&self.page, &mut self.rt, self.kept.frame(), dt);
        }
        let measure = EpxMeasure { engine: &self.text, epx: self.epx, font: None, images: &[] };
        self.draws.clear();
        self.kept.frame_retained(&mut self.rt, &self.page, Rect::new(0.0, 0.0, width, height), &measure, &mut self.draws, None);
        is_moving | self.rt.is_moving()
    }

    /// Hands one touch or mouse press to the page. `t` is the time in
    /// seconds, from any start the caller likes.
    ///
    /// Returns the events the touch caused. Before the first frame there
    /// is nothing to touch, and it returns none.
    pub fn touch(&mut self, phase: Phase, x: f32, y: f32, t: f64) -> Vec<Event> {
        if !self.kept.is_laid_out() {
            return Vec::new();
        }
        let measure = EpxMeasure { engine: &self.text, epx: self.epx, font: None, images: &[] };
        self.input.touch(&self.page, &mut self.rt, self.kept.frame(), &measure, Touch { phase, x, y, t, id: MOUSE_ID })
    }

    /// A press and a lift at the middle of `node`, 0.05 s apart.
    ///
    /// Needed so a test says "tap the second row" and not where the row
    /// happens to sit.
    ///
    /// Returns the events of both.
    pub fn tap(&mut self, node: NodeId, t: f64) -> Vec<Event> {
        let r: Rect = self.rect(node);
        let (x, y) = (r.x + r.w / 2.0, r.y + r.h / 2.0);
        let mut events: Vec<Event> = self.touch(Phase::Down, x, y, t);
        events.extend(self.touch(Phase::Up, x, y, t + 0.05));
        events
    }

    /// The mouse moved with no button held: the node under it shows its
    /// `when hovered` look.
    pub fn hover(&mut self, x: f32, y: f32) {
        if self.kept.is_laid_out() {
            self.input.hover(&self.page, &mut self.rt, self.kept.frame(), x, y);
        }
    }

    /// The mouse left the page: nothing is hovered.
    pub fn unhover(&mut self) {
        self.input.unhover(&self.page, &mut self.rt);
    }

    /// One key press, for the control that has the keyboard.
    ///
    /// Returns the events the key caused.
    pub fn key(&mut self, key: Key, mods: Mods) -> Vec<Event> {
        if !self.kept.is_laid_out() {
            return Vec::new();
        }
        self.input.key(&mut self.page, &mut self.rt, self.kept.frame(), key, mods)
    }

    /// The mouse wheel turned by `notches` at (`x`, `y`); a positive count
    /// is a turn away from the user.
    ///
    /// Needed because gogo pans a `Scroll` and a `List` by touch only, and
    /// on a PC a list that does not follow the wheel reads as broken.
    ///
    /// A `Slider` or `Toggle` under the mouse takes the wheel first, as
    /// gogo's `Input::wheel` has it. Otherwise the nearest `Scroll` or
    /// `List` around the node under the mouse moves up or down. The far
    /// end is held by gogo on the next frame.
    ///
    /// Returns the events of a control that took the wheel.
    pub fn wheel(&mut self, x: f32, y: f32, notches: f32) -> Vec<Event> {
        crate::page_ui::wheel(&self.page, &mut self.rt, &mut self.input, &self.kept, x, y, notches)
    }
    /// Asks to be told when the page's file was saved. `wake` runs on
    /// another thread.
    ///
    /// Needed because a window that draws nothing while its page is still
    /// would not notice a save: `wake` brings its loop back, and the loop
    /// then calls `reload_if_saved`. Does nothing for a page from text.
    pub fn on_file_saved(&mut self, wake: impl Fn() + Send + 'static) {
        if let Some(reload) = self.reload.as_mut() {
            reload.on_change(wake);
        }
    }

    /// Brings the page in step with its file when the file was saved.
    ///
    /// After `Ok(true)` the app writes again what its code writes: a page
    /// whose node list changed starts fresh.
    ///
    /// Returns `Ok(true)` when a new page is in, `Ok(false)` when nothing
    /// was saved, and the compiler's words when the save cannot be shown.
    /// The page on screen then stays.
    pub fn reload_if_saved(&mut self) -> Result<bool, String> {
        let Some(reload) = self.reload.as_mut() else { return Ok(false) };
        match reload.refresh(&mut self.page, &mut self.rt) {
            Ok(true) => {
                self.input = Input::new();
                self.kept = KeptFrame::default();
                Ok(true)
            }
            Ok(false) => Ok(false),
            Err(why) => Err(why.to_string()),
        }
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
        crate::page_ui::outline(&self.page, &self.kept)
    }
}
