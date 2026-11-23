//! A program's pages with no window: back stack, page cache, pages on
//! their way in and out.

use std::any::TypeId;
use std::cell::RefCell;
use std::path::Path;
use std::sync::Arc;

use gogo_text::TextEngine;
use gogo_ui::reload::{BuildTables, Reloader, Sources};
use gogo_ui::{Draw, Event, EventList, Hook, ImageKey, Key, Locale, MOUSE_ID, Mods, NodeId, Orientation, Phase, PropKey, Rect, ScreenClass, Value};

use crate::page::{Back, Host, IntoPage, Nav, PageKind, PageObject, Request};
use crate::pages::{PageUi, PageId, PagePlatform, PagePose, PageStack, UiAssets};

/// Root of a page: first node of its file.
const ROOT: NodeId = NodeId::new(0);

/// What build wrote for a program: `ui::BUILT`, added to built pages by
/// `turbodot::build::pages`.
pub struct Built {
    /// (key, file bytes) of every font a page names. First one draws every
    /// text that names no font.
    pub fonts: &'static [(&'static str, &'static [u8])],
    /// gogo's tables of the build: pages, keys, pictures.
    pub tables: &'static BuildTables<'static>,
    /// Pictures every line marks `release=unused` (the build's
    /// `RELEASE_UNUSED`): they leave the GPU once no live page names
    /// them.
    pub release_unused: &'static [ImageKey],
    /// Folder the pages' files were in at build.
    pub folder: &'static str,
}

/// What a page on its way does.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Way {
    /// Opened page comes in: its root's `on:show` plays.
    ComesIn,
    /// Page under a leaving fullscreen page comes back: its root's
    /// `on:uncover` plays.
    ComesBack,
    /// Page leaves: its root's `on:hide` plays, then page is off screen.
    Leaves,
}

impl Way {
    /// Hook of page's root that plays on this way.
    fn hook(self) -> Hook {
        match self {
            Way::ComesIn => Hook::Show,
            Way::ComesBack => Hook::Uncover,
            Way::Leaves => Hook::Hide,
        }
    }
}

/// One page on its way in or out, for as long as its root's anim for
/// that way plays.
#[derive(Clone, Copy, Debug)]
struct PageOnItsWay {
    page: PageId,
    way: Way,
}

/// A program's pages, run with no window and no GPU.
///
/// Needed so same pages run in a window (`start`), in a test and under an
/// AI agent: touch goes in as numbers, what pages show is read back from
/// `ui`.
///
/// Holds program-wide state (`A`), back stack, cached pages, pages on
/// their way in or out. Positions and sizes in epx.
pub struct Program<A = ()> {
    app: A,
    pub(crate) assets: UiAssets,
    platform: PagePlatform,
    /// Every live page with its frame, in draw order.
    pub(crate) stack: PageStack,
    /// Every live page's struct, by its id in `stack`.
    objects: Vec<Option<Box<dyn PageObject<A>>>>,
    /// Pages user went through, oldest first. Last one is on screen.
    back_stack: Vec<PageId>,
    /// Cached pages: alive, not drawn, waiting for next open.
    cached: Vec<PageId>,
    /// True once page's `KIND` was checked against its picture, by page's
    /// id in `stack`.
    #[cfg_attr(not(debug_assertions), allow(dead_code))]
    is_kind_checked: Vec<bool>,
    /// True once page's nodes played their `on:show` for page's first
    /// show, by page's id in `stack`. False for page made ahead of its
    /// first open, until that open.
    has_played_entrances: Vec<bool>,
    /// Pages whose root plays its way in or out.
    on_their_way: Vec<PageOnItsWay>,
    nav: Nav<A>,
    reload: Reloader,
    /// True once host said the screen's facts
    /// (`set_screen_and_insets`): `frame` then leaves them alone.
    is_screen_from_host: bool,
    /// Device pixels per epx.
    epx: f32,
    /// Width and height of last frame, in epx.
    size: (f32, f32),
    /// What each page put under its nodes this frame (`gpu_draw_screen`),
    /// by page's id in `stack`.
    #[cfg_attr(not(feature = "gpu"), allow(dead_code))]
    layer_draws: Vec<Vec<Draw>>,
    /// Keys of each page's own pictures, by page's id in `stack`.
    #[cfg_attr(not(feature = "gpu"), allow(dead_code))]
    picture_keys: Vec<Vec<ImageKey>>,
    /// Pictures of freed pages: renderer lets go of their textures at
    /// next GPU frame.
    #[cfg_attr(not(feature = "gpu"), allow(dead_code))]
    pictures_to_release: Vec<ImageKey>,
    /// Page on screen that got its `arrived` call. `None` from an open
    /// or a back until no page is on its way.
    arrived: Option<PageId>,
    /// Layers above every page, in the order added (`Nav::add_layer`).
    layers: Vec<PageId>,
    /// By page's id in `stack`: page it is placed in and that page's
    /// node (`Nav::place`). `None`: page of the back stack, or cached.
    placed_in: Vec<Option<(PageId, NodeId)>>,
    /// Placed pages in the order placed: page placed inside a placed
    /// page comes after it.
    placed_order: Vec<PageId>,
    /// Keys of freed pages' code-made pictures: renderer lets go of
    /// their textures at next GPU frame.
    #[cfg_attr(not(feature = "gpu"), allow(dead_code))]
    code_pictures_to_release: Vec<ImageKey>,
    /// Decodes and uploads pictures the drawn pages name.
    #[cfg(feature = "gpu")]
    pictures: crate::pictures::PictureLoader,
    /// True from a page's free until next GPU frame, which then frees
    /// the `release=unused` pictures no live page names.
    #[cfg_attr(not(feature = "gpu"), allow(dead_code))]
    has_freed_a_page: bool,
    /// Draw commands of what is drawn off its laid-out place (a page
    /// placed in a node that moves, a page's own pictures under a root
    /// that moves), moved to where they are now. Filled and read within
    /// one `for_each_list` call.
    #[cfg_attr(not(feature = "gpu"), allow(dead_code))]
    posed: RefCell<Vec<Draw>>,
}

impl<A: 'static> Program<A> {
    /// Program that opens on `first`. `built`: `&ui::BUILT`. `app`:
    /// program-wide state, every page's `Host::app`.
    ///
    /// Returns why pages cannot show: no page names a font.
    pub fn new<P: IntoPage<A, Kind>, Kind>(built: &'static Built, app: A, first: P) -> Result<Program<A>, String> {
        Program::with_reloader(built, app, first, Reloader::off())
    }

    /// As `new`, and pages follow their `.gogo` files while program runs:
    /// a save shows in the open window.
    ///
    /// Needed while a page is being written. Follows only in debug build
    /// with cargo feature `hot-reload`, run where the folder it was built
    /// from still is. Every other build reads no file.
    ///
    /// A save can change a node's look and place. Node added or removed:
    /// build program again.
    pub fn following_page_files<P: IntoPage<A, Kind>, Kind>(built: &'static Built, app: A, first: P) -> Result<Program<A>, String> {
        let reloader: Reloader = if cfg!(debug_assertions) && Path::new(built.folder).is_dir() { Reloader::new(built.tables, Sources::dir(built.folder)) } else { Reloader::off() };
        Program::with_reloader(built, app, first, reloader)
    }

    /// As `following_page_files`, for files in `folder`, which need not
    /// be where build read them, in any build with cargo feature
    /// `hot-reload`.
    ///
    /// Needed on a phone: its pages' files are where a PC pushes each
    /// save, and its build is not a debug build.
    pub fn following_page_files_in<P: IntoPage<A, Kind>, Kind>(built: &'static Built, app: A, first: P, folder: &Path) -> Result<Program<A>, String> {
        Program::with_reloader(built, app, first, Reloader::new(built.tables, Sources::dir(folder)))
    }

    fn with_reloader<P: IntoPage<A, Kind>, Kind>(built: &'static Built, app: A, first: P, reload: Reloader) -> Result<Program<A>, String> {
        if built.fonts.is_empty() {
            return Err("no page names a font, so no text can be drawn: write `theme { font = \"res://assets/<file>.ttf\" }` in a page".to_string());
        }
        let fonts: Vec<(&str, Vec<u8>)> = built.fonts.iter().map(|(key, bytes)| (*key, bytes.to_vec())).collect();
        let assets: UiAssets = UiAssets::new(TextEngine::new(&fonts), crate::pictures::image_table(built.tables, built.release_unused));
        let platform: PagePlatform = PagePlatform::new([0.0; 4], Orientation::from_view(1.0, 1.0), ScreenClass::from_view(1.0, 1.0), None);
        let mut program: Program<A> = Program { app, assets, platform, stack: PageStack::new(), objects: Vec::new(), back_stack: Vec::new(), cached: Vec::new(), is_kind_checked: Vec::new(), has_played_entrances: Vec::new(), on_their_way: Vec::new(), nav: Nav::new(), reload, is_screen_from_host: false, epx: 1.0, size: (0.0, 0.0), layer_draws: Vec::new(), picture_keys: Vec::new(),
            pictures_to_release: Vec::new(),
            code_pictures_to_release: Vec::new(),
            arrived: None,
            layers: Vec::new(),
            placed_in: Vec::new(),
            placed_order: Vec::new(),
            #[cfg(feature = "gpu")]
            pictures: crate::pictures::PictureLoader::default(),
            has_freed_a_page: false,
            posed: RefCell::new(Vec::new()),
        };
        program.open(first.into_page_object().0);
        program.carry_out_requests();
        Ok(program)
    }

    /// Program-wide state.
    pub fn app(&self) -> &A {
        &self.app
    }

    /// Program-wide state, to change from outside a page.
    pub fn app_mut(&mut self) -> &mut A {
        &mut self.app
    }

    /// Program's text engine and picture table.
    ///
    /// Needed by a host that lays out pages of its own beside the
    /// program's (`turbodot::pages`): one renderer draws both, so both
    /// measure text with one engine and name pictures by one table.
    pub fn assets(&self) -> &UiAssets {
        &self.assets
    }

    /// As `assets`, to add a font or a picture's key from outside a
    /// page.
    pub fn assets_mut(&mut self) -> &mut UiAssets {
        &mut self.assets
    }

    /// Page on screen: its nodes, texts, rows, rectangles.
    pub fn ui(&self) -> &PageUi {
        self.stack.get(self.top())
    }

    /// Page on screen, to write into from outside a page.
    pub fn ui_mut(&mut self) -> &mut PageUi {
        let top: PageId = self.top();
        self.stack.get_mut(top)
    }

    /// Calls `call` now with the live page whose struct is a `P`, as
    /// `Nav::tell` from a page does. Allocates nothing.
    ///
    /// Needed for what comes from outside the pages: a host that got a
    /// file tells a layer to say so.
    ///
    /// Returns false when no such page is alive: `call` did not run.
    pub fn tell<P: 'static>(&mut self, call: impl FnOnce(&mut P, &mut Host<'_, A>)) -> bool {
        let told: Option<PageId> = self.stack.iter().map(|(id, _)| id).find(|id| self.page_type(*id) == Some(TypeId::of::<P>()));
        let Some(id) = told else { return false };
        self.call(id, |page, host| {
            if let Some(page) = page.page_mut().downcast_mut::<P>() {
                call(page, host);
            }
        });
        self.carry_out_requests();
        true
    }

    /// Adds `page` as a layer above every page, as `Nav::add_layer` from
    /// a page does.
    pub fn add_layer<P: IntoPage<A, Kind>, Kind>(&mut self, page: P) {
        self.add_layer_object(page.into_page_object().0);
        self.carry_out_requests();
    }

    /// Layer whose struct is a `P`: its nodes, texts, rectangles.
    pub fn layer_ui<P: 'static>(&self) -> Option<&PageUi> {
        self.layer_of::<P>().map(|id| self.stack.get(id))
    }

    /// Struct of the layer that is a `P`.
    pub fn layer<P: 'static>(&self) -> Option<&P> {
        self.objects[self.layer_of::<P>()?.index()].as_ref()?.page().downcast_ref::<P>()
    }

    fn layer_of<P: 'static>(&self) -> Option<PageId> {
        self.layers.iter().copied().find(|id| self.page_type(*id) == Some(TypeId::of::<P>()))
    }

    /// Page placed in `node` of page on screen (`Nav::place`): its
    /// nodes, texts, rectangles. `None`: nothing is placed there.
    pub fn placed_ui(&self, node: NodeId) -> Option<&PageUi> {
        self.placed_at(self.top(), node).map(|id| self.stack.get(id))
    }

    /// Struct of page placed in `node` of page on screen. `None`:
    /// nothing is placed there, or what is placed is not a `P`.
    pub fn placed<P: 'static>(&self, node: NodeId) -> Option<&P> {
        let id: PageId = self.placed_at(self.top(), node)?;
        self.objects[id.index()].as_ref()?.page().downcast_ref::<P>()
    }

    /// True when a live page whose struct is a `P` is drawn this frame:
    /// on screen, on its way, or under an overlay. False while a page above
    /// it covers the screen, and for a cached page.
    ///
    /// Needed by a check that a covered page costs no drawing.
    pub fn is_drawn<P: 'static>(&self) -> bool {
        self.stack.order.iter().any(|id| self.page_type(*id) == Some(TypeId::of::<P>()))
    }

    /// Count of live pages: on the back stack, placed in a page, cached,
    /// layers.
    ///
    /// Needed by a check that a page was freed, with what was placed in
    /// it.
    pub fn live_page_count(&self) -> usize {
        self.stack.iter().count()
    }

    /// Struct of page on screen, `None` when that page is not a `P`.
    ///
    /// Needed so a test reads what a page holds after a touch.
    pub fn page<P: 'static>(&self) -> Option<&P> {
        self.objects[self.top().index()].as_ref().and_then(|object| object.page().downcast_ref::<P>())
    }

    /// Count of pages on back stack. 1 = first page alone.
    pub fn depth(&self) -> usize {
        self.back_stack.len()
    }

    /// Count of cached pages waiting for next open.
    pub fn cached_page_count(&self) -> usize {
        self.cached.len()
    }

    /// Count of pages drawn: page on screen, plus a page on its way out
    /// or the page under one on its way. Page placed in another counts
    /// with it, not on its own. Layers do not count.
    pub fn drawn_page_count(&self) -> usize {
        self.stack.order.iter().filter(|id| self.placed_in[id.index()].is_none() && !self.layers.contains(id)).count()
    }

    /// True while a page is on its way in or out: its root plays its
    /// `on:show`, `on:uncover` or `on:hide`.
    pub fn is_page_arriving_or_leaving(&self) -> bool {
        !self.on_their_way.is_empty()
    }

    /// True while a fullscreen page is in place: nothing under the
    /// program's pages shows. False while that page is on its way in
    /// or out: what is under it shows through.
    ///
    /// Needed by host that draws under the program (a painting): it
    /// skips that drawing while this says true.
    ///
    /// First page counts by its `KIND` like any page: see-through
    /// first page says `PageKind::Overlay`.
    pub fn covers_screen(&self) -> bool {
        self.back_stack.iter().any(|id| self.page_covers_screen(*id))
    }

    /// Frees every cached page. Next open makes it again.
    ///
    /// Needed when OS says memory is short. Pages on back stack stay:
    /// user would return to an empty screen.
    pub fn free_cached_pages(&mut self) {
        for id in std::mem::take(&mut self.cached) {
            self.free(id);
        }
    }

    /// Adds font file for characters no page font has.
    ///
    /// Needed because such a character draws as empty box: Roboto has no
    /// arrows, so "a → b" shows a box between the letters until a font
    /// with arrows is added here.
    ///
    /// Returns false when file is missing, unreadable, or holds no font.
    pub fn add_fallback_font_file(&mut self, path: &Path) -> bool {
        let is_added: bool = self.assets.text.borrow_mut().add_fallback_font_file(path);
        if is_added {
            for (_, page) in self.stack.iter_mut() {
                page.runtime.dirty = true;
            }
        }
        is_added
    }

    /// Sets device pixels per epx.
    ///
    /// Needed because text is shaped in device pixels: at 150 % a letter
    /// has more pixels, and measured at 100 % it comes out soft and a
    /// little off its place.
    pub fn set_epx(&mut self, epx: f32) {
        self.epx = epx;
    }

    /// Device pixels per epx, as last set.
    pub fn epx(&self) -> f32 {
        self.epx
    }

    /// Width and height of last frame in epx. (0, 0) before first frame.
    pub fn size(&self) -> (f32, f32) {
        self.size
    }

    /// Calls `wake` when a page's file was saved. `wake` runs on another
    /// thread.
    ///
    /// Needed because a window that draws nothing while pages are still
    /// would miss a save: `wake` brings its loop back, next `frame` shows
    /// the save. Does nothing for a program that follows no file.
    pub fn on_page_file_saved(&mut self, wake: impl Fn() + Send + 'static) {
        self.reload.on_change(wake);
    }

    /// One frame, `dt` seconds after last: moves page animations, lays
    /// out every changed page for `width` x `height`, ends the way of
    /// each page whose root's anim is over.
    ///
    /// Returns true while something still moves. Host may stop asking for
    /// frames on false.
    pub fn frame(&mut self, width: f32, height: f32, dt: f32) -> bool {
        if (width, height) != self.size {
            self.size = (width, height);
            if !self.is_screen_from_host {
                self.platform.set_screen_and_insets([0.0; 4], Orientation::from_view(width, height), ScreenClass::from_view(width, height));
            }
        }
        for (id, page_ui) in self.stack.iter_mut() {
            match page_ui.refresh(&mut self.reload) {
                // Saved file gave page new strings: handler names sit at
                // other ids.
                Ok(true) => {
                    if let Some(object) = self.objects[id.index()].as_mut() {
                        object.read_handler_names(page_ui);
                    }
                }
                Ok(false) => {}
                Err(why) => eprintln!("the saved page cannot be shown, keeping the last one:\n{why}"),
            }
        }
        // Each drawn page's own frame call. What a page asks of `Nav`
        // there is carried out before pages are laid out.
        let mut is_page_asking_for_frames: bool = false;
        for i in 0..self.stack.order.len() {
            let id: PageId = self.stack.order[i];
            is_page_asking_for_frames |= self.call(id, |page, host| page.frame(host, dt));
        }
        self.carry_out_requests();
        self.place_code_pictures();
        let size_px: (u32, u32) = (((width * self.epx).round() as u32).max(1), ((height * self.epx).round() as u32).max(1));
        let mut is_moving: bool = self.frame_pages(dt, size_px);
        // Ways that ended in this frame: `left`, then `arrived` of page
        // on screen. What those calls write is laid out in this frame.
        if self.end_ways() {
            self.carry_out_requests();
            self.place_code_pictures();
            is_moving |= self.frame_pages(0.0, size_px);
        }
        #[cfg(debug_assertions)]
        self.check_page_kinds();
        is_moving | is_page_asking_for_frames
    }

    /// Screen's facts, from a host that knows them: what notch and
    /// on-screen keyboard cover at each edge (epx: left, top, right,
    /// bottom), orientation, size class. Pages read them as
    /// `padding_top=safe` and `when screen.landscape`.
    ///
    /// Needed on a phone: its size alone says nothing of a notch, and
    /// its size class counts the screen, not the window. With no call a
    /// program has no insets and reads orientation and class off its
    /// frame's size, which is right for a window on a PC.
    ///
    /// From first call on, `frame` leaves the three alone: host calls
    /// again when one changes. Every page has them from next frame.
    pub fn set_screen_and_insets(&mut self, safe: [f32; 4], orientation: Orientation, class: ScreenClass) {
        self.is_screen_from_host = true;
        self.platform.set_screen_and_insets(safe, orientation, class);
    }

    /// Language of every page. `locale`: translated texts (`text=/key`),
    /// `None` for texts as the files have them. `bcp47`: language's tag
    /// (`bg`, `ja`), which text is shaped and broken into lines by.
    ///
    /// Every page has it from next frame.
    pub fn set_locale(&mut self, locale: Option<Arc<Locale>>, bcp47: &str) {
        self.assets.set_locale(locale.clone(), bcp47);
        self.platform.locale = locale;
    }

    /// Gives each picture a page made in code (`PageUi::set_picture`) its
    /// key and hands layout its size. Its Image names the key from then
    /// on.
    ///
    /// Needed before the pages are laid out: page then sits at its new
    /// picture's size in the same frame, with no GPU.
    fn place_code_pictures(&mut self) {
        let Program { stack, assets, .. } = self;
        for (id, page_ui) in stack.iter_mut() {
            for i in 0..page_ui.code_pictures.len() {
                if !page_ui.code_pictures[i].is_size_new {
                    continue;
                }
                page_ui.code_pictures[i].is_size_new = false;
                let (node, width, height): (NodeId, u32, u32) = (page_ui.code_pictures[i].node, page_ui.code_pictures[i].width, page_ui.code_pictures[i].height);
                let key: ImageKey = match page_ui.code_pictures[i].key {
                    Some(key) => key,
                    None => {
                        // Named by page and node: next page in this place
                        // takes the key over, so none is made per open.
                        let key: ImageKey = assets.images.intern(&format!("turbodot://page/{}/node/{}", id.index(), node.index()));
                        page_ui.code_pictures[i].key = Some(key);
                        page_ui.runtime.set_prop(node, PropKey::Source, Value::VImage(key));
                        key
                    }
                };
                if assets.images.set_size(key, width, height) {
                    page_ui.runtime.dirty = true;
                }
            }
        }
    }

    /// Hands one touch of one finger to the page it belongs to. `finger`:
    /// id that stays the same from a finger's down to its up (`MOUSE_ID`
    /// for a mouse). `t`: time in seconds, from any start.
    ///
    /// Finger that comes down goes to topmost drawn page that takes it:
    /// one with a node under the finger that takes input, or one that
    /// takes every touch (`Page::TAKES_EVERY_TOUCH`, the default). Its
    /// moves and its lift go to that same page.
    ///
    /// Page on its way out takes no finger: user touched what shows
    /// under it.
    ///
    /// Returns false when no page took the touch: host hands it to what
    /// is under the pages. Before first frame there is nothing to touch.
    pub fn touch_finger(&mut self, phase: Phase, x: f32, y: f32, t: f64, finger: u64) -> bool {
        if !self.stack.get(self.top()).kept.is_laid_out() {
            return false;
        }
        // A press no page takes still goes to page whose field holds the
        // text caret, so the field ends its edit.
        let taker: Option<PageId> = if phase == Phase::Down { self.page_that_takes(x, y).or_else(|| self.caret_holder().map(|(id, _)| id)) } else { self.stack.touched_by(finger) };
        let Some(id) = taker else { return false };
        let events: EventList = self.stack.touch_to(id, phase, x, y, t, finger, self.epx, &self.assets);
        self.hand_events(id, &events);
        true
    }

    /// `touch_finger` for a mouse, or for a test with one finger.
    pub fn touch(&mut self, phase: Phase, x: f32, y: f32, t: f64) -> bool {
        self.touch_finger(phase, x, y, t, MOUSE_ID)
    }

    /// True for page on its way out, and for page placed in one.
    fn is_leaving(&self, id: PageId) -> bool {
        let mut page: PageId = id;
        loop {
            if self.on_their_way.iter().any(|moving| moving.page == page && moving.way == Way::Leaves) {
                return true;
            }
            match self.placed_in[page.index()] {
                Some((parent, _)) => page = parent,
                None => return false,
            }
        }
    }

    /// One frame of every live page. Page placed in another comes after
    /// it: it is laid out in its node's rectangle as that page has it
    /// this frame, with what is left of the safe insets there, and is
    /// drawn where an anim has that node now, as see-through as it.
    ///
    /// Returns true while something still moves.
    fn frame_pages(&mut self, dt: f32, size_px: (u32, u32)) -> bool {
        let Program { stack, assets, platform, placed_in, placed_order, epx, size, .. } = self;
        let mut is_moving: bool = false;
        for (id, page) in stack.iter_mut() {
            if placed_in[id.index()].is_none() {
                is_moving |= page.frame(dt, size_px, *epx, platform, assets);
            }
        }
        let window: Rect = Rect::new(0.0, 0.0, size.0, size.1);
        for i in 0..placed_order.len() {
            let id: PageId = placed_order[i];
            let Some((parent, node)) = placed_in[id.index()] else { continue };
            let parent_ui: &PageUi = stack.get(parent);
            let rect: Rect = if parent_ui.kept.is_laid_out() { parent_ui.rect(node) } else { Rect::new(0.0, 0.0, 0.0, 0.0) };
            let pose: Option<PagePose> = PagePose::both(parent_ui.pose, parent_ui.drawn_pose(node));
            let page: &mut PageUi = stack.get_mut(id);
            page.viewport = Some(rect);
            page.pose = pose;
            is_moving |= page.frame(dt, size_px, *epx, &platform.residual(window, rect), assets);
        }
        is_moving
    }

    /// Makes `page` a layer above every page.
    fn add_layer_object(&mut self, page: Box<dyn PageObject<A>>) {
        let id: PageId = self.make(page, true);
        // A layer never keeps a touch beside its nodes: one that did
        // would leave every page under it dead.
        self.stack.get_mut(id).is_modal = false;
        self.layers.push(id);
        self.order_drawn_pages();
        self.call(id, |page, host| page.show(host));
    }

    /// Places `page` inside `node` of page `parent`, in place of a page
    /// placed there before.
    fn place(&mut self, parent: PageId, node: NodeId, page: Box<dyn PageObject<A>>) {
        if !self.stack.contains(parent) {
            return; // asking page was freed before its request's turn
        }
        if let Some(old) = self.placed_at(parent, node) {
            self.free(old);
        }
        let id: PageId = self.make(page, true);
        self.placed_in[id.index()] = Some((parent, node));
        self.placed_order.push(id);
        if self.stack.is_ordered(parent) {
            self.order_drawn_pages();
        } else {
            self.stack.order_remove(id);
        }
        self.call(id, |page, host| page.show(host));
    }

    /// Page placed in `node` of page `parent`.
    fn placed_at(&self, parent: PageId, node: NodeId) -> Option<PageId> {
        self.placed_order.iter().copied().find(|id| self.placed_in[id.index()] == Some((parent, node)))
    }

    /// Puts `id` on top of draw order, then pages placed in it, in the
    /// order placed.
    fn order_top_with_placed(&mut self, id: PageId) {
        self.stack.order_top(id);
        for i in 0..self.placed_order.len() {
            let placed: PageId = self.placed_order[i];
            if self.placed_in[placed.index()].is_some_and(|(parent, _)| parent == id) {
                self.order_top_with_placed(placed);
            }
        }
    }

    /// Takes `id` out of draw order, with pages placed in it.
    fn order_remove_with_placed(&mut self, id: PageId) {
        self.stack.order_remove(id);
        for i in 0..self.placed_order.len() {
            let placed: PageId = self.placed_order[i];
            if self.placed_in[placed.index()].is_some_and(|(parent, _)| parent == id) {
                self.order_remove_with_placed(placed);
            }
        }
    }

    /// Calls `f` with page `id`, then with pages placed in it.
    fn call_with_placed(&mut self, id: PageId, f: fn(&mut dyn PageObject<A>, &mut Host<'_, A>)) {
        self.call(id, f);
        for i in 0..self.placed_order.len() {
            let placed: PageId = self.placed_order[i];
            if self.placed_in[placed.index()].is_some_and(|(parent, _)| parent == id) {
                self.call_with_placed(placed, f);
            }
        }
    }

    /// Topmost drawn page that takes a press at (`x`, `y`), in epx.
    fn page_that_takes(&self, x: f32, y: f32) -> Option<PageId> {
        self.stack.order.iter().rev().copied().find(|id| {
            if self.is_leaving(*id) {
                return false;
            }
            let page: &PageUi = self.stack.get(*id);
            page.covers(x, y) && (page.hit(x, y).is_some() || page.is_modal)
        })
    }

    /// Press and lift at middle of `node` of page on screen, 0.05 s apart.
    ///
    /// Needed so a test says "tap second row", not where the row happens
    /// to sit.
    pub fn tap(&mut self, node: NodeId, t: f64) {
        let r: Rect = self.ui().rect(node);
        let (x, y) = (r.x + r.w / 2.0, r.y + r.h / 2.0);
        self.touch(Phase::Down, x, y, t);
        self.touch(Phase::Up, x, y, t + 0.05);
    }

    /// Mouse moved with no button held: node under it shows its `when
    /// hovered` look.
    ///
    /// Returns false when no page is under the mouse (as a touch there
    /// would go on to the host): host with pages of its own under the
    /// program's hands them the move.
    pub fn hover(&mut self, x: f32, y: f32) -> bool {
        if !self.stack.get(self.top()).kept.is_laid_out() {
            return false;
        }
        let under_mouse: Option<PageId> = self.page_that_takes(x, y);
        self.stack.hover(under_mouse, x, y);
        under_mouse.is_some()
    }

    /// Mouse left the window: nothing is hovered.
    pub fn unhover(&mut self) {
        self.stack.unhover();
    }

    /// One key press, for control that has the keyboard.
    pub fn key(&mut self, key: Key, mods: Mods) {
        let top: PageId = self.top();
        if !self.stack.get(top).kept.is_laid_out() {
            return;
        }
        let events: Vec<(PageId, Event)> = self.stack.key(key, mods);
        for (id, event) in events {
            self.hand_events(id, &[event]);
        }
    }

    /// Mouse wheel turned by `notches` at (`x`, `y`). Positive count:
    /// turn away from user.
    ///
    /// Turn goes to topmost drawn page that has something for it under
    /// the mouse: a control, a `Scroll`, a `List`. Page that takes
    /// every touch (`Page::TAKES_EVERY_TOUCH`) keeps a turn it has
    /// nothing for: pages under a dialog do not scroll.
    ///
    /// Returns false when no page took the turn, and then none moved:
    /// host with something of its own under the program's pages hands
    /// that the turn.
    pub fn wheel(&mut self, x: f32, y: f32, notches: f32) -> bool {
        for i in (0..self.stack.order.len()).rev() {
            let id: PageId = self.stack.order[i];
            if self.is_leaving(id) || !self.stack.get(id).covers(x, y) {
                continue;
            }
            let page_ui: &mut PageUi = self.stack.get_mut(id);
            let is_modal: bool = page_ui.is_modal;
            match page_ui.wheel(x, y, notches) {
                Some(events) => {
                    self.hand_events(id, &events);
                    return true;
                }
                None if is_modal => return true,
                None => {}
            }
        }
        false
    }

    /// Opens `page` over page on screen, as `Nav::go` from a page does.
    ///
    /// Needed for what opens a page from outside a page: a host that got
    /// a file to show, a test.
    pub fn go<P: IntoPage<A, Kind>, Kind>(&mut self, page: P) {
        self.open(page.into_page_object().0);
        self.carry_out_requests();
    }

    /// Page and field that hold the text caret: where an on-screen
    /// keyboard types. `None`: no field of a drawn page is being edited.
    /// Field of a page that another page covers does not count: user
    /// cannot see what a key would change.
    ///
    /// Needed by a host that shows a keyboard: up when this names a
    /// field, down on `None`, and its text read again (`ime_text`) when
    /// this names another field.
    pub fn caret_holder(&self) -> Option<(PageId, NodeId)> {
        self.stack.order.iter().rev().find_map(|id| self.stack.get(*id).runtime.caret.map(|caret| (*id, caret.0)))
    }

    /// Page that raised gogo's edit bar (cut, copy, paste) over one of
    /// its fields, and the field. A finger held still on a field raises
    /// it.
    ///
    /// Needed by a host that draws the bar: it reads field's rectangle
    /// and selection from `page_ui`.
    pub fn edit_bar_raised(&self) -> Option<(PageId, NodeId)> {
        self.stack.edit_bar_raised()
    }

    /// Takes the edit bar down on whichever page raised it.
    pub fn dismiss_edit_bar(&mut self) {
        self.stack.dismiss_edit_bar();
    }

    /// Live page `id`: its nodes, texts, rectangles. `None`: page was
    /// freed since `id` was handed out.
    pub fn page_ui(&self, id: PageId) -> Option<&PageUi> {
        if self.stack.contains(id) { Some(self.stack.get(id)) } else { None }
    }

    /// True while a node of a drawn page has the keyboard.
    ///
    /// Needed by a host with pages of its own beside the program's: a
    /// copy or a paste goes to whichever side has the keyboard.
    pub fn has_focus(&self) -> bool {
        self.stack.focused().is_some()
    }

    /// Text selected in field that has the keyboard: what a copy puts on
    /// the clipboard. gogo reads and writes no clipboard itself.
    pub fn selected_text(&self) -> Option<String> {
        self.stack.selected_text()
    }

    /// Cut: deletes text selected in field that has the keyboard, and
    /// hands its page the change.
    ///
    /// Returns the deleted text, for the clipboard. `None`: nothing was
    /// selected, nothing changed.
    pub fn cut_selected_text(&mut self) -> Option<String> {
        let text: String = self.stack.selected_text()?;
        let id: PageId = self.stack.focused()?;
        let page: &mut PageUi = self.stack.get_mut(id);
        let events: EventList = page.input.delete_selection(&mut page.page, &mut page.runtime);
        self.hand_events(id, &events);
        Some(text)
    }

    /// Paste: writes `text` into field that has the keyboard, in place
    /// of its selection, and hands its page the change. Does nothing
    /// when no field has the keyboard.
    pub fn paste(&mut self, text: &str) {
        let Some(id) = self.stack.focused() else { return };
        let page: &mut PageUi = self.stack.get_mut(id);
        let events: EventList = page.input.paste(&mut page.page, &mut page.runtime, text);
        self.hand_events(id, &events);
    }

    /// Selects all text of field that has the keyboard.
    pub fn select_all(&mut self) {
        self.stack.select_all_focused();
    }

    /// Text of field that holds the caret, with its selection as two
    /// character counts (start, end; equal: caret alone).
    ///
    /// Needed by an on-screen keyboard that edits whole text (Android):
    /// host seeds it from here when a field takes the caret, so
    /// backspace and autocorrect act on field's real text.
    pub fn ime_text(&self) -> Option<(String, usize, usize)> {
        let (id, _) = self.caret_holder()?;
        let page: &PageUi = self.stack.get(id);
        page.input.ime_seed(&page.page, &page.runtime)
    }

    /// Whole text and selection an on-screen keyboard sends after each
    /// of its edits: written into field that holds the caret, and its
    /// page is handed the change. Does nothing when no field holds it.
    pub fn ime_replace(&mut self, text: &str, start: usize, end: usize) {
        let Some((id, _)) = self.caret_holder() else { return };
        let page: &mut PageUi = self.stack.get_mut(id);
        let events: EventList = page.input.ime_replace(&mut page.page, &mut page.runtime, text, start, end);
        self.hand_events(id, &events);
    }

    /// Pages' own GPU drawing for one frame: each drawn page's
    /// `gpu_draw_textures`, then its `gpu_draw_screen`. A window calls it
    /// after `frame` and before renderer prepares the pages.
    ///
    /// A page that is not drawn (under a fullscreen page, or cached) is
    /// not called: its GPU work would be for a picture nobody sees.
    ///
    /// Before that it uploads the pictures the drawn pages name. One
    /// marked `loading=lazy` goes to a decode thread and shows in a
    /// later frame (`is_loading_pictures`).
    #[cfg(feature = "gpu")]
    pub fn gpu_draw(&mut self, renderer: &mut gogo_wgpu::UiRenderer, gpu: &crate::gpu::Gpu) {
        self.gpu_draw_as(renderer, gpu, crate::pictures::LazyPictures::OnThread, None);
    }

    /// `gpu_draw` for a host that lays out pages of its own beside the
    /// program's (`turbodot::pages`), drawn by the same renderer.
    ///
    /// Needed because the program's picture loader is the one loader of
    /// that renderer: it uploads the pictures `host_pages` draw too, and
    /// does not free a `release=unused` picture a live page of the host
    /// names. With plain `gpu_draw` a host's page shows an empty place
    /// where its picture belongs.
    #[cfg(feature = "gpu")]
    pub fn gpu_draw_beside(&mut self, renderer: &mut gogo_wgpu::UiRenderer, gpu: &crate::gpu::Gpu, host_pages: &dyn crate::pages::HostPages) {
        self.gpu_draw_as(renderer, gpu, crate::pictures::LazyPictures::OnThread, Some(host_pages));
    }

    /// Says a page of the host's own was freed. Next GPU frame then
    /// frees the `release=unused` pictures no live page names, as it
    /// does after a page of the program was freed.
    pub fn host_freed_a_page(&mut self) {
        self.has_freed_a_page = true;
    }

    /// True while pictures marked `loading=lazy` are being decoded.
    ///
    /// Needed by a loop that draws no frame while nothing moves: it
    /// keeps drawing until they land, or they show only at next touch.
    #[cfg(feature = "gpu")]
    pub fn is_loading_pictures(&self) -> bool {
        self.pictures.is_decoding()
    }

    /// Marks every picture as not on the GPU: ones from files, ones
    /// pages made in code.
    ///
    /// Needed for a renderer that was made new: it has none of them, and
    /// page would show an empty place where each belongs.
    #[cfg(feature = "gpu")]
    pub(crate) fn forget_uploads(&mut self) {
        self.assets.images.forget_uploads();
        for (_, page_ui) in self.stack.iter_mut() {
            for picture in page_ui.code_pictures.iter_mut() {
                picture.is_uploaded = false;
            }
        }
    }

    /// `gpu_draw`, with what this frame does with a `loading=lazy`
    /// picture, and with a host's own pages when it has some.
    #[cfg(feature = "gpu")]
    pub(crate) fn gpu_draw_as(&mut self, renderer: &mut gogo_wgpu::UiRenderer, gpu: &crate::gpu::Gpu, lazy: crate::pictures::LazyPictures, host_pages: Option<&dyn crate::pages::HostPages>) {
        if self.has_freed_a_page {
            self.has_freed_a_page = false;
            let stack: &PageStack = &self.stack;
            self.pictures.release_unshown(&mut self.assets.images, renderer, &gpu.device, |live| {
                stack.mark_pictures(live);
                if let Some(host_pages) = host_pages {
                    host_pages.mark_pictures(live);
                }
            });
        }
        let stack: &PageStack = &self.stack;
        self.pictures.upload_for_draws(&mut self.assets.images, renderer, &gpu.device, &gpu.queue, lazy, |wanted| {
            stack.wanted_images(wanted);
            if let Some(host_pages) = host_pages {
                host_pages.wanted_images(wanted);
            }
        });
        for i in 0..self.code_pictures_to_release.len() {
            renderer.release_image(&gpu.device, self.code_pictures_to_release[i]);
        }
        self.code_pictures_to_release.clear();
        // Pixels pages made in code since last GPU frame.
        let Program { stack, assets, .. } = self;
        for (_, page_ui) in stack.iter_mut() {
            for picture in page_ui.code_pictures.iter_mut() {
                let Some(key) = picture.key else { continue };
                if !picture.is_uploaded {
                    assets.images.upload_image(renderer, &gpu.device, &gpu.queue, key, &picture.rgba, picture.width, picture.height);
                    picture.is_uploaded = true;
                }
            }
        }
        for i in 0..self.pictures_to_release.len() {
            let external: gogo_wgpu::ExternalImage = renderer.external_image(self.pictures_to_release[i]);
            renderer.release_external_image(&gpu.device, external);
        }
        self.pictures_to_release.clear();
        for i in 0..self.stack.order.len() {
            let id: PageId = self.stack.order[i];
            let Some(mut object) = self.objects[id.index()].take() else { continue };
            self.nav.caller = Some(id);
            let mut host: Host<'_, A> = Host { app: &mut self.app, ui: self.stack.get_mut(id), nav: &mut self.nav };
            object.gpu_draw_textures(&mut host, gpu);
            let draws: &mut Vec<Draw> = &mut self.layer_draws[id.index()];
            draws.clear();
            let mut layers: crate::page::Layers<'_> = crate::page::Layers { draws, renderer: &mut *renderer, device: &gpu.device, images: &mut self.assets.images, keys: &mut self.picture_keys[id.index()], used: 0, page_index: id.index() };
            object.gpu_draw_screen(&host, &mut layers);
            self.nav.caller = None;
            self.objects[id.index()] = Some(object);
        }
        self.carry_out_requests();
    }

    /// Back request: Android back key, Esc on PC, a swipe. Page on screen
    /// answers first (`Page::back`). If it leaves, page before it shows.
    ///
    /// Returns false when first page was asked to leave: nothing is before
    /// it. Phone then puts program in background, PC does nothing.
    pub fn back(&mut self) -> bool {
        let was_used: bool = self.back_once();
        self.carry_out_requests();
        was_used
    }

    /// Page on screen: last one of back stack.
    fn top(&self) -> PageId {
        *self.back_stack.last().expect("a program has a first page")
    }

    /// Calls a function of page `id` with what a page gets.
    fn call<R>(&mut self, id: PageId, f: impl FnOnce(&mut dyn PageObject<A>, &mut Host<'_, A>) -> R) -> R {
        let mut object: Box<dyn PageObject<A>> = self.objects[id.index()].take().expect("a live page has its struct");
        self.nav.caller = Some(id);
        let mut host: Host<'_, A> = Host { app: &mut self.app, ui: self.stack.get_mut(id), nav: &mut self.nav };
        let result: R = f(object.as_mut(), &mut host);
        self.nav.caller = None;
        self.objects[id.index()] = Some(object);
        result
    }

    /// Hands a page its events, then does what page asked for.
    fn hand_events(&mut self, id: PageId, events: &[Event]) {
        for event in events {
            self.call(id, |page, host| page.handle(host, event));
        }
        self.carry_out_requests();
    }

    /// Does what pages asked of `Nav`, in order asked. A page called on the
    /// way may ask for more. Then tells page on screen that it arrived,
    /// when it did; what that page asks in turn is carried out too.
    fn carry_out_requests(&mut self) {
        loop {
            self.carry_out_asked();
            if !self.tell_arrival() {
                return;
            }
        }
    }

    /// Calls `arrived` of page on screen (and of pages placed in it),
    /// once per arrival: no page is on its way, and that page has not
    /// had the call since it came on top.
    ///
    /// Returns true when it made the call.
    fn tell_arrival(&mut self) -> bool {
        let top: PageId = self.top();
        if !self.on_their_way.is_empty() || self.arrived == Some(top) {
            return false;
        }
        self.arrived = Some(top);
        self.call_with_placed(top, |page, host| page.arrived(host));
        true
    }

    fn carry_out_asked(&mut self) {
        while !self.nav.requests.is_empty() {
            for request in std::mem::take(&mut self.nav.requests) {
                match request {
                    Request::Go(page) => self.open(page),
                    Request::Replace(page) => self.replace(page),
                    Request::Back => {
                        self.back_once();
                    }
                    Request::Preload(page) => self.preload(page),
                    Request::Layer(page) => self.add_layer_object(page),
                    Request::Place { parent, node, page } => self.place(parent, node, page),
                    Request::Tell { page_type, call } => {
                        let told: Option<PageId> = self.stack.iter().map(|(id, _)| id).find(|id| self.objects[id.index()].as_ref().is_some_and(|object| object.page_type() == page_type));
                        if let Some(id) = told {
                            self.call(id, |page, host| call(page.page_mut(), host));
                        }
                    }
                }
            }
        }
    }

    /// Type of client's struct of page `id`.
    fn page_type(&self, id: PageId) -> Option<TypeId> {
        self.objects[id.index()].as_ref().map(|object| object.page_type())
    }

    /// Makes a page from its compiled file and calls its `loaded`. Page
    /// lands on top of draw order. `is_shown_now`: false for page made
    /// ahead of its first open.
    fn make(&mut self, mut page: Box<dyn PageObject<A>>, is_shown_now: bool) -> PageId {
        let mut page_ui: PageUi = PageUi::new(self.reload.open(page.built()));
        page.read_handler_names(&page_ui);
        // Plays each visible node's `on:show` once. Page nobody sees
        // yet plays them at its first open: they would play unseen.
        if is_shown_now {
            page_ui.runtime.play_entrances(&page_ui.page);
        }
        page_ui.is_modal = page.takes_every_touch();
        // Screen's facts and language, before page's `loaded`: what
        // page's code writes there is in that language.
        self.platform.apply(&page_ui.page, &mut page_ui.runtime);
        let id: PageId = self.stack.add(page_ui);
        if self.objects.len() <= id.index() {
            self.objects.resize_with(id.index() + 1, || None);
            self.is_kind_checked.resize(id.index() + 1, false);
            self.has_played_entrances.resize(id.index() + 1, false);
            self.layer_draws.resize_with(id.index() + 1, Vec::new);
            self.picture_keys.resize_with(id.index() + 1, Vec::new);
            self.placed_in.resize(id.index() + 1, None);
        }
        self.placed_in[id.index()] = None;
        self.objects[id.index()] = Some(page);
        self.is_kind_checked[id.index()] = false;
        self.has_played_entrances[id.index()] = is_shown_now;
        self.call(id, |page, host| page.loaded(host));
        id
    }

    /// Frees a page for good.
    fn free(&mut self, id: PageId) {
        // Pages placed in it go with it.
        while let Some(placed) = self.placed_order.iter().copied().find(|placed| self.placed_in[placed.index()].is_some_and(|(parent, _)| parent == id)) {
            self.free(placed);
        }
        self.placed_order.retain(|other| *other != id);
        self.placed_in[id.index()] = None;
        self.on_their_way.retain(|moving| moving.page != id);
        self.code_pictures_to_release.extend(self.stack.get(id).code_pictures.iter().filter_map(|picture| picture.key));
        self.stack.remove(id);
        self.objects[id.index()] = None;
        self.layer_draws[id.index()].clear();
        self.pictures_to_release.extend(self.picture_keys[id.index()].iter().copied());
        self.has_freed_a_page = true;
    }

    /// Page on screen leaves unasked, `fresh` opens in its place.
    fn replace(&mut self, fresh: Box<dyn PageObject<A>>) {
        let leaving: PageId = self.back_stack.pop().expect("a program has a first page");
        self.start_leave(leaving);
        self.open(fresh);
    }

    /// True while root of page `id` plays its anim for `way`.
    fn is_way_playing(&self, id: PageId, way: Way) -> bool {
        let page_ui: &PageUi = self.stack.get(id);
        page_ui.runtime.is_hook_playing(&page_ui.page, ROOT, way.hook())
    }

    /// Notes page `id` as on `way`, when its root plays its anim for
    /// it, in place of a way page was on. Page whose root plays none
    /// stays as noted.
    ///
    /// Returns true when page is on `way` now.
    fn note_way(&mut self, id: PageId, way: Way) -> bool {
        if !self.is_way_playing(id, way) {
            return false;
        }
        self.on_their_way.retain(|moving| moving.page != id);
        self.on_their_way.push(PageOnItsWay { page: id, way });
        true
    }

    /// Page `id` starts its way out: its root plays its `on:hide`. Page
    /// whose root has none is off screen at once (`end_leave`).
    ///
    /// Returns true while page is on its way out.
    fn start_leave(&mut self, id: PageId) -> bool {
        let page_ui: &mut PageUi = self.stack.get_mut(id);
        page_ui.runtime.play_hook(&page_ui.page, ROOT, Hook::Hide);
        if self.note_way(id, Way::Leaves) {
            return true;
        }
        self.on_their_way.retain(|moving| moving.page != id);
        self.end_leave(id);
        false
    }

    /// Page `id` is off screen for good: its `left`, then cached page
    /// is cached, any other page freed.
    fn end_leave(&mut self, id: PageId) {
        self.call_with_placed(id, |page, host| page.left(host));
        let is_cached_page: bool = self.objects[id.index()].as_ref().is_some_and(|object| object.is_cached_page());
        if is_cached_page && self.nav.is_page_cache_enabled {
            self.order_remove_with_placed(id);
            self.cached.push(id);
        } else {
            self.free(id);
        }
    }

    /// Opens `fresh` over page on screen. Cached page of that type, or one
    /// still on its way out, shows again and takes `fresh` over. Its
    /// root's `on:show` plays, as a new page's does.
    fn open(&mut self, fresh: Box<dyn PageObject<A>>) {
        let cached: Option<PageId> = if fresh.is_cached_page() {
            let page_type: TypeId = fresh.page_type();
            let leaving = self.on_their_way.iter().filter(|moving| moving.way == Way::Leaves).map(|moving| moving.page);
            self.cached.iter().copied().chain(leaving).find(|id| self.page_type(*id) == Some(page_type))
        } else {
            None
        };
        let id: PageId = match cached {
            Some(id) => {
                self.cached.retain(|other| *other != id);
                self.on_their_way.retain(|moving| moving.page != id);
                self.objects[id.index()].as_mut().expect("a live page has its struct").reopened(fresh.into_page());
                let has_played_entrances: bool = std::mem::replace(&mut self.has_played_entrances[id.index()], true);
                let page_ui: &mut PageUi = self.stack.get_mut(id);
                // Page that was on its way out stops there, and comes
                // in anew: gogo stops root's `on:hide` as it plays its
                // `on:show`.
                if has_played_entrances {
                    page_ui.runtime.play_hook(&page_ui.page, ROOT, Hook::Show);
                } else {
                    page_ui.runtime.play_entrances(&page_ui.page);
                }
                id
            }
            None => self.make(fresh, true),
        };
        self.back_stack.push(id);
        self.arrived = None;
        self.note_way(id, Way::ComesIn);
        self.order_drawn_pages();
        self.call_with_placed(id, |page, host| page.show(host));
    }

    /// Makes a cached page ahead of its first open. Its nodes play
    /// their `on:show` at that open.
    fn preload(&mut self, page: Box<dyn PageObject<A>>) {
        let page_type: TypeId = page.page_type();
        let is_alive: bool = (0..self.objects.len()).any(|i| self.objects[i].as_ref().is_some_and(|object| object.page_type() == page_type));
        if !self.nav.is_page_cache_enabled || is_alive {
            return;
        }
        let id: PageId = self.make(page, false);
        self.order_remove_with_placed(id);
        self.cached.push(id);
    }

    /// One back request, without what the asked page asks for in turn.
    fn back_once(&mut self) -> bool {
        let top: PageId = self.top();
        match self.call(top, |page, host| page.back(host)) {
            Back::Stay => true,
            Back::Leave => self.leave(),
        }
    }

    /// Page on screen leaves, page before it shows.
    ///
    /// Returns false for first page: none before it.
    fn leave(&mut self) -> bool {
        if self.back_stack.len() < 2 {
            return false;
        }
        let leaving: PageId = self.back_stack.pop().expect("two pages");
        let revealed: PageId = self.top();
        let is_fullscreen: bool = self.objects[leaving.index()].as_ref().is_some_and(|object| object.kind() == PageKind::Fullscreen);
        let is_on_its_way_out: bool = self.start_leave(leaving);
        // Page under a fullscreen page that takes time to leave comes
        // back with it. Under an overlay it never left the screen, and
        // under a page gone at once there is nothing to come back from.
        if is_on_its_way_out && is_fullscreen {
            let page_ui: &mut PageUi = self.stack.get_mut(revealed);
            page_ui.runtime.play_uncovers(&page_ui.page);
            self.note_way(revealed, Way::ComesBack);
        }
        self.order_drawn_pages();
        self.call_with_placed(revealed, |page, host| page.show(host));
        true
    }

    /// Ends the way of each page whose root's anim is over. Page that
    /// was leaving hears `left`, then is cached or freed.
    ///
    /// Returns true when a way ended.
    fn end_ways(&mut self) -> bool {
        let mut has_ended: bool = false;
        while let Some(at) = (0..self.on_their_way.len()).find(|i| !self.is_way_playing(self.on_their_way[*i].page, self.on_their_way[*i].way)) {
            let ended: PageOnItsWay = self.on_their_way.remove(at);
            has_ended = true;
            if ended.way == Way::Leaves {
                self.end_leave(ended.page);
            }
        }
        if has_ended {
            // A fullscreen page in place now covers what is under it.
            self.order_drawn_pages();
        }
        has_ended
    }

    /// True when page `id` hides everything under it: fullscreen page, in
    /// place. A page still on its way is moved or see-through, so what
    /// is under it shows.
    fn page_covers_screen(&self, id: PageId) -> bool {
        let is_fullscreen: bool = self.objects[id.index()].as_ref().is_some_and(|object| object.kind() == PageKind::Fullscreen);
        is_fullscreen && !self.on_their_way.iter().any(|moving| moving.page == id)
    }

    /// Puts into draw order what shows: back stack pages from topmost
    /// covering page up, then pages on their way out.
    ///
    /// Needed so a page under a fullscreen page costs no drawing, and a
    /// page under an overlay keeps showing.
    fn order_drawn_pages(&mut self) {
        let lowest_drawn: usize = (0..self.back_stack.len()).rev().find(|i| self.page_covers_screen(self.back_stack[*i])).unwrap_or(0);
        for i in 0..self.back_stack.len() {
            let id: PageId = self.back_stack[i];
            if i < lowest_drawn {
                self.order_remove_with_placed(id);
            } else {
                self.order_top_with_placed(id);
            }
        }
        for i in 0..self.on_their_way.len() {
            if self.on_their_way[i].way == Way::Leaves {
                let leaving: PageId = self.on_their_way[i].page;
                self.order_top_with_placed(leaving);
            }
        }
        for i in 0..self.layers.len() {
            let layer: PageId = self.layers[i];
            self.order_top_with_placed(layer);
        }
    }

    /// Debug build: compares `KIND` of each page that has a page under it
    /// with its picture, once, at its first still frame.
    ///
    /// Needed because a wrong `KIND` shows late or never: fullscreen page
    /// with see-through root leaves a hole where page under it was, and
    /// overlay with opaque root makes engine draw a page nobody sees.
    ///
    /// WARNING: panics for fullscreen page whose root does not cover the
    /// window, unless page says it draws its own GPU picture. First page
    /// is not checked: nothing is under it.
    #[cfg(debug_assertions)]
    fn check_page_kinds(&mut self) {
        for i in 1..self.back_stack.len() {
            let id: PageId = self.back_stack[i];
            let page: &PageUi = self.stack.get(id);
            let is_still: bool = page.kept.is_laid_out() && page.pose.is_none() && !page.runtime.is_moving();
            if self.is_kind_checked[id.index()] || !is_still {
                continue;
            }
            self.is_kind_checked[id.index()] = true;
            let Some((kind, draws_gpu_picture)) = self.objects[id.index()].as_ref().map(|object| (object.kind(), object.draws_gpu_picture())) else { continue };
            let root = &page.page.nodes[0];
            let name: &str = if root.name == gogo_ui::NO_STR { "(no name)" } else { page.page.str(root.name) };
            match (kind, page.covers_viewport()) {
                (PageKind::Fullscreen, false) if !draws_gpu_picture => panic!("page `{name}` is `PageKind::Fullscreen`, but its root does not cover the window with an opaque `background=`: the page under it is not drawn, so a hole shows. Give the root an opaque background, or write `const KIND: PageKind = PageKind::Overlay;` in the page. A page that covers the screen with its own GPU picture writes `const DRAWS_GPU_PICTURE: bool = true;`"),
                (PageKind::Overlay, true) => eprintln!("page `{name}` is `PageKind::Overlay`, but its root covers the window with an opaque background: the page under it is drawn and never seen. Take `const KIND` out of the page"),
                _ => {}
            }
        }
    }
}

/// Pages as renderer draws them: bottom first, each page's own pictures
/// under its nodes and moved and faded with its root, a page placed in
/// a node moved and faded with that node.
#[cfg(feature = "gpu")]
impl<A> gogo_wgpu::DrawLists for Program<A> {
    fn for_each_list<'a>(&'a self, f: &mut dyn FnMut(&[Draw], &'a gogo_ui::Page)) {
        for id in &self.stack.order {
            let page: &'a PageUi = self.stack.get(*id);
            let pictures: &[Draw] = self.layer_draws[id.index()].as_slice();
            // Page's own pictures lie under its nodes: where an anim
            // has its root, they are too.
            let pictures_pose: Option<PagePose> = if pictures.is_empty() { None } else { PagePose::both(page.pose, page.drawn_pose(ROOT)) };
            for (draws, pose) in [(pictures, pictures_pose), (page.draws.as_slice(), page.pose)] {
                match pose {
                    _ if draws.is_empty() => {}
                    None => f(draws, &page.page),
                    Some(pose) => {
                        let mut posed = self.posed.borrow_mut();
                        posed.clear();
                        posed.extend(draws.iter().map(|draw| draw.posed(pose.offset_x, pose.offset_y, pose.alpha)));
                        f(posed.as_slice(), &page.page);
                    }
                }
            }
        }
    }
}
