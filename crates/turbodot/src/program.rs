//! A program's pages with no window: back stack, page cache, slides
//! between pages.

use std::any::TypeId;
use std::cell::RefCell;
use std::path::Path;

use gogo_text::TextEngine;
use gogo_ui::reload::{BuildTables, Reloader, Sources};
use gogo_ui::{Draw, Event, Images, Key, MOUSE_ID, Mods, NodeId, Orientation, Phase, Rect, ScreenClass};

use crate::page::{Back, Host, IntoPage, Nav, PageObject, Request};
use crate::pages::{HostedPage, ImageTable, PageId, PagePlatform, PagePose, PageStack, UiAssets};

/// Time an opened page takes to slide in.
const OPEN_SLIDE_SEC: f32 = 0.3;
/// Time a page takes to slide out on back, and page under it to slide
/// back in.
const BACK_SLIDE_SEC: f32 = 0.2;
/// Distance to the right where an opened page starts and a leaving page
/// ends.
const OPEN_SLIDE_FROM_EPX: f32 = 60.0;
/// Distance to the left where page under a leaving page starts.
const REVEAL_SLIDE_FROM_EPX: f32 = -30.0;
/// Part of a slide in over which page fades in.
const FADE_IN_PART_OF_SLIDE: f32 = 0.8;

/// What build wrote for a program: `ui::BUILT`, added to built pages by
/// `turbodot::build::pages`.
pub struct Built {
    /// (key, file bytes) of every font a page names. First one draws every
    /// text that names no font.
    pub fonts: &'static [(&'static str, &'static [u8])],
    /// gogo's tables of the build: pages, keys, pictures.
    pub tables: &'static BuildTables<'static>,
    /// Folder the pages' files were in at build.
    pub folder: &'static str,
}

/// Which way a page slides.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum SlideKind {
    /// Opened page comes in from right.
    Open,
    /// Page under a leaving page comes back in from left.
    Reveal,
    /// Page leaves to right.
    Leave,
}

/// One page on its way in or out.
#[derive(Clone, Copy, Debug)]
struct Slide {
    page: PageId,
    kind: SlideKind,
    elapsed_sec: f32,
}

impl Slide {
    fn duration_sec(&self) -> f32 {
        match self.kind {
            SlideKind::Open => OPEN_SLIDE_SEC,
            SlideKind::Reveal | SlideKind::Leave => BACK_SLIDE_SEC,
        }
    }

    fn has_ended(&self) -> bool {
        self.elapsed_sec >= self.duration_sec()
    }

    /// Where page is drawn now. Page coming in slows down into place, page
    /// leaving speeds up out of it.
    fn pose(&self) -> PagePose {
        let duration: f32 = self.duration_sec();
        let part: f32 = (self.elapsed_sec / duration).clamp(0.0, 1.0);
        let rest: f32 = 1.0 - part;
        let slowing: f32 = 1.0 - rest * rest * rest;
        let speeding: f32 = part * part * part;
        let fade_in: f32 = (self.elapsed_sec / (duration * FADE_IN_PART_OF_SLIDE)).clamp(0.0, 1.0);
        match self.kind {
            SlideKind::Open => PagePose { offset_x: OPEN_SLIDE_FROM_EPX * (1.0 - slowing), offset_y: 0.0, alpha: fade_in },
            SlideKind::Reveal => PagePose { offset_x: REVEAL_SLIDE_FROM_EPX * (1.0 - slowing), offset_y: 0.0, alpha: fade_in },
            SlideKind::Leave => PagePose { offset_x: OPEN_SLIDE_FROM_EPX * speeding, offset_y: 0.0, alpha: rest },
        }
    }
}

/// A program's pages, run with no window and no GPU.
///
/// Needed so same pages run in a window (`start`), in a test and under an
/// AI agent: touch goes in as numbers, what pages show is read back from
/// `ui`.
///
/// Holds program-wide state (`A`), back stack, cached pages, slides
/// between pages. Positions and sizes in epx.
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
    slides: Vec<Slide>,
    nav: Nav<A>,
    reload: Reloader,
    /// Device pixels per epx.
    epx: f32,
    /// Width and height of last frame, in epx.
    size: (f32, f32),
    /// Draw commands of a sliding page, moved to where it is now. Filled
    /// and read within one `for_each_list` call.
    #[cfg_attr(not(feature = "gpu"), allow(dead_code))]
    posed: RefCell<Vec<Draw>>,
}

impl<A: 'static> Program<A> {
    /// Program that opens on `first`. `built`: `&ui::BUILT`. `app`:
    /// program-wide state, every page's `Host::app`.
    ///
    /// Returns why pages cannot show: no page names a font, or a page
    /// names a picture (`Program` shows no pictures yet).
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

    fn with_reloader<P: IntoPage<A, Kind>, Kind>(built: &'static Built, app: A, first: P, reload: Reloader) -> Result<Program<A>, String> {
        if built.fonts.is_empty() {
            return Err("no page names a font, so no text can be drawn: write `theme { font = \"res://assets/<file>.ttf\" }` in a page".to_string());
        }
        if let Some((path, _)) = built.tables.images.first() {
            return Err(format!("a page names the picture `{path}`: a `Program` cannot show pictures yet"));
        }
        let fonts: Vec<(&str, Vec<u8>)> = built.fonts.iter().map(|(key, bytes)| (*key, bytes.to_vec())).collect();
        let assets: UiAssets = UiAssets::new(TextEngine::new(&fonts), ImageTable::from_keys(Images::default()));
        let platform: PagePlatform = PagePlatform::new([0.0; 4], Orientation::from_view(1.0, 1.0), ScreenClass::from_view(1.0, 1.0), None);
        let mut program: Program<A> = Program { app, assets, platform, stack: PageStack::new(), objects: Vec::new(), back_stack: Vec::new(), cached: Vec::new(), slides: Vec::new(), nav: Nav::new(), reload, epx: 1.0, size: (0.0, 0.0), posed: RefCell::new(Vec::new()) };
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

    /// Page on screen: its nodes, texts, rows, rectangles.
    pub fn ui(&self) -> &HostedPage {
        self.stack.get(self.top())
    }

    /// Page on screen, to write into from outside a page.
    pub fn ui_mut(&mut self) -> &mut HostedPage {
        let top: PageId = self.top();
        self.stack.get_mut(top)
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

    /// Count of pages drawn: page on screen, plus a sliding page or the
    /// page a sliding page uncovers.
    pub fn drawn_page_count(&self) -> usize {
        self.stack.order.len()
    }

    /// True while a page is on its way in or out.
    pub fn is_sliding(&self) -> bool {
        !self.slides.is_empty()
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

    /// One frame, `dt` seconds after last: moves slides and page
    /// animations, lays out every changed page for `width` x `height`.
    ///
    /// Returns true while something still moves. Host may stop asking for
    /// frames on false.
    pub fn frame(&mut self, width: f32, height: f32, dt: f32) -> bool {
        if (width, height) != self.size {
            self.size = (width, height);
            self.platform.set_screen_and_insets([0.0; 4], Orientation::from_view(width, height), ScreenClass::from_view(width, height));
        }
        for (_, page) in self.stack.iter_mut() {
            if let Err(why) = page.refresh(&mut self.reload) {
                eprintln!("the saved page cannot be shown, keeping the last one:\n{why}");
            }
        }
        let is_sliding: bool = self.move_slides(dt);
        let size_px: (u32, u32) = (((width * self.epx).round() as u32).max(1), ((height * self.epx).round() as u32).max(1));
        let is_moving: bool = self.stack.frame(dt, size_px, self.epx, &self.platform, &self.assets);
        is_sliding | is_moving
    }

    /// Hands one touch or mouse press to page on screen. `t`: time in
    /// seconds, from any start.
    ///
    /// Before first frame there is nothing to touch.
    pub fn touch(&mut self, phase: Phase, x: f32, y: f32, t: f64) {
        let top: PageId = self.top();
        if !self.stack.get(top).kept.is_laid_out() {
            return;
        }
        let events: Vec<Event> = self.stack.touch_to(top, phase, x, y, t, MOUSE_ID, self.epx, &self.assets);
        self.hand_events(top, &events);
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
    pub fn hover(&mut self, x: f32, y: f32) {
        let top: PageId = self.top();
        if self.stack.get(top).kept.is_laid_out() {
            self.stack.hover(Some(top), x, y);
        }
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

    /// Mouse wheel turned by `notches` at (`x`, `y`) over page on screen.
    /// Positive count: turn away from user.
    pub fn wheel(&mut self, x: f32, y: f32, notches: f32) {
        let top: PageId = self.top();
        let events: Vec<Event> = self.stack.get_mut(top).wheel(x, y, notches);
        self.hand_events(top, &events);
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
        let mut host: Host<'_, A> = Host { app: &mut self.app, ui: self.stack.get_mut(id), nav: &mut self.nav };
        let result: R = f(object.as_mut(), &mut host);
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
    /// way may ask for more.
    fn carry_out_requests(&mut self) {
        while !self.nav.requests.is_empty() {
            for request in std::mem::take(&mut self.nav.requests) {
                match request {
                    Request::Go(page) => self.open(page),
                    Request::Back => {
                        self.back_once();
                    }
                    Request::Preload(page) => self.preload(page),
                }
            }
        }
    }

    /// Type of client's struct of page `id`.
    fn page_type(&self, id: PageId) -> Option<TypeId> {
        self.objects[id.index()].as_ref().map(|object| object.page_type())
    }

    /// Makes a page from its compiled file and calls its `loaded`. Page
    /// lands on top of draw order.
    fn make(&mut self, page: Box<dyn PageObject<A>>) -> PageId {
        let mut hosted: HostedPage = HostedPage::new(self.reload.open(page.built()));
        // Plays each visible node's `on:show` once.
        hosted.runtime.play_entrances(&hosted.page);
        let id: PageId = self.stack.add(hosted);
        if self.objects.len() <= id.index() {
            self.objects.resize_with(id.index() + 1, || None);
        }
        self.objects[id.index()] = Some(page);
        self.call(id, |page, host| page.loaded(host));
        id
    }

    /// Frees a page for good.
    fn free(&mut self, id: PageId) {
        self.slides.retain(|slide| slide.page != id);
        self.stack.remove(id);
        self.objects[id.index()] = None;
    }

    /// Opens `fresh` over page on screen. Cached page of that type, or one
    /// still sliding out, shows again and takes `fresh` over.
    fn open(&mut self, fresh: Box<dyn PageObject<A>>) {
        let cached: Option<PageId> = if fresh.is_cached_page() {
            let page_type: TypeId = fresh.page_type();
            let leaving = self.slides.iter().filter(|slide| slide.kind == SlideKind::Leave).map(|slide| slide.page);
            self.cached.iter().copied().chain(leaving).find(|id| self.page_type(*id) == Some(page_type))
        } else {
            None
        };
        let id: PageId = match cached {
            Some(id) => {
                self.cached.retain(|other| *other != id);
                self.slides.retain(|slide| slide.page != id);
                self.objects[id.index()].as_mut().expect("a live page has its struct").reopened(fresh.into_page());
                id
            }
            None => self.make(fresh),
        };
        let has_page_under: bool = !self.back_stack.is_empty();
        self.back_stack.push(id);
        self.stack.order_top(id);
        self.stack.get_mut(id).pose = None;
        if has_page_under {
            self.slide(id, SlideKind::Open);
        }
        self.call(id, |page, host| page.show(host));
    }

    /// Makes a cached page ahead of its first open.
    fn preload(&mut self, page: Box<dyn PageObject<A>>) {
        let page_type: TypeId = page.page_type();
        let is_alive: bool = (0..self.objects.len()).any(|i| self.objects[i].as_ref().is_some_and(|object| object.page_type() == page_type));
        if !self.nav.is_page_cache_enabled || is_alive {
            return;
        }
        let id: PageId = self.make(page);
        self.stack.order_remove(id);
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
        self.stack.order_below(revealed, leaving);
        self.slide(revealed, SlideKind::Reveal);
        self.slide(leaving, SlideKind::Leave);
        self.call(revealed, |page, host| page.show(host));
        true
    }

    /// Starts a slide of `page`, in place of one it is in.
    fn slide(&mut self, page: PageId, kind: SlideKind) {
        self.slides.retain(|slide| slide.page != page);
        let slide: Slide = Slide { page, kind, elapsed_sec: 0.0 };
        self.stack.get_mut(page).pose = Some(slide.pose());
        self.slides.push(slide);
    }

    /// Moves every slide on by `dt` seconds, ends the ones that are over.
    ///
    /// Returns true while a page still slides.
    fn move_slides(&mut self, dt: f32) -> bool {
        for i in 0..self.slides.len() {
            self.slides[i].elapsed_sec += dt;
            let slide: Slide = self.slides[i];
            self.stack.get_mut(slide.page).pose = if slide.has_ended() { None } else { Some(slide.pose()) };
        }
        let ended: Vec<Slide> = self.slides.iter().copied().filter(Slide::has_ended).collect();
        self.slides.retain(|slide| !slide.has_ended());
        for slide in ended {
            match slide.kind {
                // Opened page covers the screen now: pages under it are
                // drawn no more, until back shows one again.
                SlideKind::Open => {
                    if self.back_stack.last() == Some(&slide.page) {
                        for i in 0..self.back_stack.len() - 1 {
                            let under: PageId = self.back_stack[i];
                            self.stack.order_remove(under);
                        }
                    }
                }
                SlideKind::Reveal => {}
                SlideKind::Leave => {
                    let is_cached_page: bool = self.objects[slide.page.index()].as_ref().is_some_and(|object| object.is_cached_page());
                    if is_cached_page && self.nav.is_page_cache_enabled {
                        self.stack.order_remove(slide.page);
                        self.cached.push(slide.page);
                    } else {
                        self.free(slide.page);
                    }
                }
            }
        }
        !self.slides.is_empty()
    }
}

/// Pages as renderer draws them: bottom first, a sliding page moved to
/// where it is now.
#[cfg(feature = "gpu")]
impl<A> gogo_wgpu::DrawLists for Program<A> {
    fn for_each_list<'a>(&'a self, f: &mut dyn FnMut(&[Draw], &'a gogo_ui::Page)) {
        self.stack.for_each_draws(&mut |draws, page, pose| match pose {
            None => f(draws, page),
            Some(pose) => {
                let mut posed = self.posed.borrow_mut();
                posed.clear();
                posed.extend(draws.iter().map(|draw| draw.posed(pose.offset_x, pose.offset_y, pose.alpha)));
                f(posed.as_slice(), page);
            }
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pose_at(kind: SlideKind, elapsed_sec: f32) -> PagePose {
        slide_at(kind, elapsed_sec).pose()
    }

    fn slide_at(kind: SlideKind, elapsed_sec: f32) -> Slide {
        Slide { page: PageStack::new().add(HostedPage::new(gogo_ui::compile("View v", &|_| None).page)), kind, elapsed_sec }
    }

    /// Opened page starts 60 epx to the right and clear, ends in place and
    /// solid. Leaving page does the reverse. Page it uncovers comes from
    /// 30 epx to the left.
    #[test]
    fn a_slide_starts_and_ends_where_it_says() {
        let open: PagePose = pose_at(SlideKind::Open, 0.0);
        assert_eq!((open.offset_x, open.alpha), (OPEN_SLIDE_FROM_EPX, 0.0));
        let open: PagePose = pose_at(SlideKind::Open, OPEN_SLIDE_SEC);
        assert_eq!((open.offset_x, open.alpha), (0.0, 1.0));
        // Fade is over before slide is.
        assert_eq!(pose_at(SlideKind::Open, OPEN_SLIDE_SEC * 0.9).alpha, 1.0);
        let reveal: PagePose = pose_at(SlideKind::Reveal, 0.0);
        assert_eq!((reveal.offset_x, reveal.alpha), (REVEAL_SLIDE_FROM_EPX, 0.0));
        let leave: PagePose = pose_at(SlideKind::Leave, 0.0);
        assert_eq!((leave.offset_x, leave.alpha), (0.0, 1.0));
        let leave: PagePose = pose_at(SlideKind::Leave, BACK_SLIDE_SEC);
        assert_eq!((leave.offset_x, leave.alpha), (OPEN_SLIDE_FROM_EPX, 0.0));
        assert!(slide_at(SlideKind::Leave, BACK_SLIDE_SEC).has_ended());
    }
}
