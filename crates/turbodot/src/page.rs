//! What a client writes: a page, and what a page gets with every call.

use std::any::{Any, TypeId};

use gogo_ui::{CommandSurface, Event, EventKind, HandlerName, NodeId, StaticPage, StrId, SurfaceCommands};

use crate::pages::{PageId, PageUi};
#[cfg(feature = "gpu")]
use crate::pages::{OtherPages, TexturePlacement, UiAssets};

/// Page as build compiled it from its `.gogo` file: `PAGE` of page's
/// module in `ui`.
pub type BuiltPage = StaticPage;

/// Page's answer to back request.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Back {
    /// Page leaves, page before it shows.
    Leave,
    /// Page stays: it used the request itself, to close a panel or to ask
    /// "keep or discard".
    Stay,
}

/// How much of the screen a page covers. Page says it with `const KIND`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PageKind {
    /// Page covers whole screen. Once it is in place, pages under it
    /// are not drawn.
    Fullscreen,
    /// Page lets page under it show: a dialog, a translucent sheet. Pages
    /// under it stay drawn.
    Overlay,
}

/// One thing user did on a page, under handler name from page's file.
///
/// Needed so page code matches on its own `Handler` enum: handler with no
/// arm stops build. With node id compares, a new button does nothing and
/// nothing says so.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PageEvent<H> {
    pub handler: H,
    /// What happened, with its value: slider's new value, finger's place.
    pub kind: EventKind,
    /// Node it happened on. In a list: the row's own node;
    /// `PageUi::row_key` gives the item.
    pub node: NodeId,
}

/// What a page gets with every call.
///
/// Needed as one value so page functions take one parameter, and a page
/// of a program that shares nothing never names `app`.
pub struct Host<'a, A = ()> {
    /// Program-wide state, reached by every page.
    pub app: &'a mut A,
    /// The page being called: its nodes, texts, rows, rectangles.
    pub ui: &'a mut PageUi,
    /// Opens another page, asks for back.
    pub nav: &'a mut Nav<A>,
}

/// The GPU as a page gets it in `Page::before_draw`: device and queue
/// for page's own textures, and what draws nodes into a texture
/// (`PageUi::draw_to_texture`, `PageGpu::draw_to_texture`).
///
/// Needed as a type made for that one call: drawing nodes takes the
/// renderer the screen is drawn with, its letters and the program's
/// language, and a page names none of them. Who looks for "draw my
/// page into a texture" finds a function that takes this, and this
/// only in `before_draw`.
#[cfg(feature = "gpu")]
pub struct PageGpu<'a> {
    pub device: &'a wgpu::Device,
    pub queue: &'a wgpu::Queue,
    pub(crate) renderer: &'a mut gogo_wgpu::UiRenderer,
    pub(crate) assets: &'a UiAssets,
    /// Every live page but the one being called.
    pub(crate) others: OtherPages<'a>,
    /// Every page kept off screen (`Nav::keep_off_screen`).
    pub(crate) kept: &'a [KeptOffScreen],
}

/// The GPU as a page gets it in `Page::before_draw`. Without cargo
/// feature `gpu` nothing makes one: page code still compiles, and
/// `before_draw` is never called.
#[cfg(not(feature = "gpu"))]
pub struct PageGpu<'a>(std::marker::PhantomData<&'a ()>);

#[cfg(feature = "gpu")]
impl PageGpu<'_> {
    /// Draws `node` of `page`, a page kept off screen
    /// (`Nav::keep_off_screen`), and everything inside it to a new
    /// texture, as that page's last layout has them. The walk and the
    /// renderer are the screen's. `place.px_per_epx` is texture pixels
    /// per epx of that page: its layout is in whole epx.
    ///
    /// Needed for a picture made of nodes no screen shows, at a size the
    /// screen does not have: art of a collage at the size of the saved
    /// file.
    ///
    /// WARNING: each call makes a texture, a pass and a submit. For a
    /// change (a new size, a new text), not for every frame.
    ///
    /// Returns a tile with colors multiplied by alpha, transparent where
    /// nothing was drawn. `None` when `page` was freed with the page
    /// that kept it, or has had no frame yet: it was kept in this frame.
    #[inline]
    pub fn draw_to_texture(&mut self, page: OffScreenPage, node: impl gogo_ui::NodeName, place: TexturePlacement) -> Option<gogo_wgpu::Tex_Tile_Premul_Srgb_4xU8_RGBA> {
        self.draw_to_texture_by_id(page, node.node(), place)
    }

    /// What `draw_to_texture` does, by the id of the node.
    fn draw_to_texture_by_id(&mut self, page: OffScreenPage, node: NodeId, place: TexturePlacement) -> Option<gogo_wgpu::Tex_Tile_Premul_Srgb_4xU8_RGBA> {
        let page_ui: &PageUi = off_screen_ui_among(self.kept, &self.others, page)?;
        Some(page_ui.draw_to_texture_with_by_id(self.renderer, self.device, self.queue, self.assets, node, place))
    }

    /// `page`, a page kept off screen, as its last layout has it, to
    /// read: where its nodes sit, what its texts say.
    ///
    /// Needed to draw one node of it whole: the node's rectangle gives
    /// `TexturePlacement` its origin and its size.
    ///
    /// Returns `None` where `draw_to_texture` does.
    pub fn off_screen_ui(&self, page: OffScreenPage) -> Option<&PageUi> {
        off_screen_ui_among(self.kept, &self.others, page)
    }
}

/// Page kept off screen under `page`, when it is among `others` and was
/// laid out.
///
/// Needed apart from `PageGpu`: what it returns is borrowed from the
/// pages alone, so the `PageGpu`'s renderer stays free for the draw.
#[cfg(feature = "gpu")]
fn off_screen_ui_among<'a>(kept: &[KeptOffScreen], others: &OtherPages<'a>, page: OffScreenPage) -> Option<&'a PageUi> {
    let kept: &KeptOffScreen = kept.iter().find(|kept| kept.handle == page)?;
    others.get(kept.page).filter(|page_ui| page_ui.kept.is_laid_out())
}

/// A page kept off screen, as `Nav::keep_off_screen` named it for the
/// page that keeps it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct OffScreenPage(pub(crate) u32);

/// One page kept off screen.
#[derive(Clone, Copy, Debug)]
pub(crate) struct KeptOffScreen {
    pub(crate) handle: OffScreenPage,
    pub(crate) page: PageId,
    /// Page that keeps it: kept page lives and is laid out with it.
    pub(crate) keeper: PageId,
    /// Size it is laid out for, in epx.
    pub(crate) width: f32,
    pub(crate) height: f32,
}

impl KeptOffScreen {
    /// Size the page is laid out for in pixels: one per epx.
    pub(crate) fn size_px(&self) -> (u32, u32) {
        ((self.width.round() as u32).max(1), (self.height.round() as u32).max(1))
    }
}

/// One screen of a program: `.gogo` file plus struct that writes into it
/// and answers its events.
///
/// `A`: program-wide state (`Host::app`). Program whose pages share
/// nothing leaves it out: `impl Page for HomePage`.
///
/// Cached page: write [`CachedPage`] in place of this trait.
pub trait Page<A = ()>: Sized + 'static {
    /// Page's compiled file: `&ui::<file>::PAGE`.
    const PAGE: &'static BuiltPage;
    /// Handler names of page's file: `ui::<file>::Handler`.
    type Handler: HandlerName + 'static;
    /// `PageKind::Overlay` for page that lets page under it show.
    ///
    /// Needed because engine stops drawing pages under a fullscreen
    /// page: under a see-through page that leaves a hole. Debug build
    /// checks this line against page's picture (`Program::frame`).
    const KIND: PageKind = PageKind::Fullscreen;
    /// False for page that lets a touch on none of its nodes go on: to
    /// page drawn under it, else to the host (`Program::touch_finger`
    /// returns false).
    ///
    /// Needed for page over a painting or a world that another page or
    /// the host draws: the fingers no button took go on to it. Any other
    /// page keeps a tap beside its buttons to itself.
    const TAKES_EVERY_TOUCH: bool = true;

    /// Writes program's data into page: texts, rows, selection.
    ///
    /// Called each time page comes into view: on open, and when page above
    /// it left. Page with nothing to write leaves it out.
    fn show(&mut self, host: &mut Host<'_, A>) {
        let _ = host;
    }

    /// Says what one user action means.
    fn handle(&mut self, host: &mut Host<'_, A>, event: PageEvent<Self::Handler>);

    /// Page's heavy work. Called once, when page is made.
    ///
    /// Needed because `show` runs on every return to page: costly writes
    /// (long list) go here.
    fn loaded(&mut self, host: &mut Host<'_, A>) {
        let _ = host;
    }

    /// Called once a frame while page is drawn, before pages are laid
    /// out. `dt_sec`: seconds since last frame.
    ///
    /// Needed for what changes with time and not with a touch: a toast
    /// that goes away, a progress bar.
    ///
    /// A rectangle read here (`PageUi::rect`) is of the frame before:
    /// `after_layout` reads this frame's.
    ///
    /// Returns true while page needs more frames. A program whose pages
    /// all return false, with nothing moving, draws no frame until next
    /// touch.
    fn frame(&mut self, host: &mut Host<'_, A>, dt_sec: f32) -> bool {
        let _ = (host, dt_sec);
        false
    }

    /// Called after page was laid out, while page is drawn: each of its
    /// nodes has its rectangle of this frame (`PageUi::rect`).
    ///
    /// Needed for what page places from a rectangle of its own layout:
    /// a picture fitted into the room its file leaves it, a bar seated
    /// over a node. In `frame` that rectangle is of the frame before.
    ///
    /// Not called in a frame that did not lay page out: nothing of it
    /// changed. Page laid out while it was not drawn (cached, under a
    /// fullscreen page) is called in first frame it is drawn again.
    ///
    /// What page writes here is laid out once more before the draw, and
    /// that layout makes no second call. A page placed in a node of this
    /// page is laid out again too, in that node's new rectangle, before
    /// its own call.
    ///
    /// WARNING: page that writes here at every call is laid out twice
    /// in every frame that lays it out. Write only what changed.
    fn after_layout(&mut self, host: &mut Host<'_, A>) {
        let _ = host;
    }

    /// Called once page is on screen, in place, with no page on its
    /// way: when its root's `on:show` ended (at once for root with
    /// none), and when page above it is off screen.
    ///
    /// Needed for what must wait for page to be in place: a native
    /// view (an ad banner) shown at open is on screen a moment before
    /// the page it belongs to.
    fn arrived(&mut self, host: &mut Host<'_, A>) {
        let _ = host;
    }

    /// Called once page is off screen: its root's `on:hide` ended, or
    /// root has none. Cached page is cached after this call, any other
    /// page is freed.
    ///
    /// Page with no `on:hide` hears it before page under it hears
    /// `show`.
    ///
    /// Needed as page's last call with `host`: what page took for
    /// itself in program's state goes back here. `Drop` has no `host`.
    fn left(&mut self, host: &mut Host<'_, A>) {
        let _ = host;
    }

    /// Answers back request: Android back key, Esc on PC, page's own
    /// `Nav::back`.
    ///
    /// Not asked when a Popup took the request: `back_closed_popup` is
    /// called in its place.
    ///
    /// Returns `Back::Leave` unless page writes it.
    fn back(&mut self, host: &mut Host<'_, A>) -> Back {
        let _ = host;
        Back::Leave
    }

    /// Called when a back request closed a Popup, in place of `back`: a
    /// Popup of this page, or of a page placed in it.
    ///
    /// Needed for what a page does at every back request, whoever
    /// answers it: a hint "back works here" that goes away at first
    /// back. Every back request reaches page on screen once: here, or
    /// as `back`.
    ///
    /// Page cannot stop the close. Which Popup closed is not said: a
    /// Check that showed it raised its `on:change=` before this call
    /// (`handle`). Page placed in another is not called: page on screen
    /// is.
    ///
    /// WARNING: page's own `Nav::back` goes the same road. With a Popup
    /// open it closes the Popup, this is called, and page stays.
    fn back_closed_popup(&mut self, host: &mut Host<'_, A>) {
        let _ = host;
    }

    /// Page's own GPU work, each frame before the pages are drawn: draws
    /// into its own textures (strokes of a painting, a game's world),
    /// hands a `Surface draw=picture` its texture
    /// (`PageUi::set_picture_texture`), draws nodes into a texture
    /// (`PageUi::draw_to_texture`, `PageGpu::draw_to_texture`).
    ///
    /// Needed before the draw: a texture the pages show must hold its
    /// picture when the pass that draws them starts.
    ///
    /// Called after the pages were laid out. What page writes here is
    /// laid out again before the draw; `after_layout` hears of that
    /// layout at next frame.
    ///
    /// Called only while page is drawn: not for page under a fullscreen
    /// page, not for cached page, not for page kept off screen. Never
    /// called without a GPU (a test that runs `Program` alone).
    fn before_draw(&mut self, host: &mut Host<'_, A>, gpu: &mut PageGpu<'_>) {
        let _ = (host, gpu);
    }

    /// Writes the draw commands of `surface`, a `Surface draw=commands`
    /// of page's file, into `out`: in the Surface's own space, (0, 0)
    /// its top left corner, `out.width()` x `out.height()` its size.
    ///
    /// Called when the Surface shows for the first time, when its size
    /// changed, and after `PageUi::surface_changed`. The commands are
    /// kept and drawn every frame until then: a frame in which nothing
    /// changed costs page's code nothing.
    ///
    /// Needs no GPU: called in a test that runs `Program` alone too.
    ///
    /// Needed with `&self` and `app` alone, without `host`: `out` holds
    /// the page while it is written.
    fn draw_surface(&self, app: &A, surface: CommandSurface, out: &mut SurfaceCommands<'_>) {
        let _ = (app, surface, out);
    }

    /// Makes the GPU draw calls of `surface`, a `Surface draw=gpu` of
    /// page's file, inside the pass that draws every page: over what the
    /// pages drew before the Surface, under what comes after it. For 3D,
    /// for a shader of page's own.
    ///
    /// The viewport is on the Surface's rectangle and the scissor on the
    /// part of it that shows (`place`), so page draws as if the Surface
    /// were its whole screen. Page leaves the pass in any state it likes.
    ///
    /// Reads only: what the calls need (buffers written, textures drawn)
    /// was made in `before_draw`. Called each frame while the Surface
    /// shows. Never called without a GPU.
    #[cfg(feature = "gpu")]
    fn draw_surface_gpu(&self, app: &A, surface: gogo_ui::GpuSurface, pass: &mut wgpu::RenderPass<'_>, place: &gogo_wgpu::SurfacePlace) {
        let _ = (app, surface, pass, place);
    }
}

/// Page cached on back, shows again same view next open.
///
/// Needed for page costly to make: nodes stay built, scroll place stays.
/// Written in place of [`Page`]: same functions plus `reopened`.
///
/// Cached while `Nav::set_page_cache_enabled` says so (default: on), until
/// `Program::free_cached_pages`.
pub trait CachedPage<A = ()>: Sized + 'static {
    /// As `Page::PAGE`.
    const PAGE: &'static BuiltPage;
    /// As `Page::Handler`.
    type Handler: HandlerName + 'static;
    /// As `Page::KIND`.
    const KIND: PageKind = PageKind::Fullscreen;
    /// False for page that lets a touch on none of its nodes go on: to
    /// page drawn under it, else to the host (`Program::touch_finger`
    /// returns false).
    ///
    /// Needed for page over a painting or a world that another page or
    /// the host draws: the fingers no button took go on to it. Any other
    /// page keeps a tap beside its buttons to itself.
    const TAKES_EVERY_TOUCH: bool = true;

    /// As `Page::show`.
    fn show(&mut self, host: &mut Host<'_, A>) {
        let _ = host;
    }

    /// As `Page::handle`.
    fn handle(&mut self, host: &mut Host<'_, A>, event: PageEvent<Self::Handler>);

    /// Takes over opener's value when cached page opens again. `fresh`:
    /// value opener made (`Nav::go(EditPage::new(3))`). `show` runs after.
    ///
    /// Needed because opener's value drops after this call: without it,
    /// page shows what its last open got. Page opened with nothing writes
    /// empty body.
    fn reopened(&mut self, fresh: Self);

    /// As `Page::loaded`.
    fn loaded(&mut self, host: &mut Host<'_, A>) {
        let _ = host;
    }

    /// Called once a frame while page is drawn, before pages are laid
    /// out. `dt_sec`: seconds since last frame.
    ///
    /// Needed for what changes with time and not with a touch: a toast
    /// that goes away, a progress bar.
    ///
    /// A rectangle read here (`PageUi::rect`) is of the frame before:
    /// `after_layout` reads this frame's.
    ///
    /// Returns true while page needs more frames. A program whose pages
    /// all return false, with nothing moving, draws no frame until next
    /// touch.
    fn frame(&mut self, host: &mut Host<'_, A>, dt_sec: f32) -> bool {
        let _ = (host, dt_sec);
        false
    }

    /// As `Page::after_layout`.
    fn after_layout(&mut self, host: &mut Host<'_, A>) {
        let _ = host;
    }

    /// As `Page::arrived`.
    fn arrived(&mut self, host: &mut Host<'_, A>) {
        let _ = host;
    }

    /// As `Page::left`.
    fn left(&mut self, host: &mut Host<'_, A>) {
        let _ = host;
    }

    /// As `Page::back`.
    fn back(&mut self, host: &mut Host<'_, A>) -> Back {
        let _ = host;
        Back::Leave
    }

    /// As `Page::back_closed_popup`.
    fn back_closed_popup(&mut self, host: &mut Host<'_, A>) {
        let _ = host;
    }

    /// As `Page::before_draw`.
    fn before_draw(&mut self, host: &mut Host<'_, A>, gpu: &mut PageGpu<'_>) {
        let _ = (host, gpu);
    }

    /// As `Page::draw_surface`.
    fn draw_surface(&self, app: &A, surface: CommandSurface, out: &mut SurfaceCommands<'_>) {
        let _ = (app, surface, out);
    }

    /// As `Page::draw_surface_gpu`.
    #[cfg(feature = "gpu")]
    fn draw_surface_gpu(&self, app: &A, surface: gogo_ui::GpuSurface, pass: &mut wgpu::RenderPass<'_>, place: &gogo_wgpu::SurfacePlace) {
        let _ = (app, surface, pass, place);
    }
}

/// Page behind one set of calls, whatever its struct.
///
/// Needed because a program's pages are structs of different types, kept
/// in one list.
pub(crate) trait PageObject<A> {
    fn built(&self) -> &'static BuiltPage;
    /// Type of client's struct.
    fn page_type(&self) -> TypeId;
    /// True when page cached on back (`CachedPage`).
    fn is_cached_page(&self) -> bool;
    fn kind(&self) -> PageKind;
    /// True for page that takes a touch on none of its nodes.
    fn takes_every_touch(&self) -> bool;
    #[cfg_attr(not(feature = "gpu"), allow(dead_code))]
    fn before_draw(&mut self, host: &mut Host<'_, A>, gpu: &mut PageGpu<'_>);
    fn draw_surface(&self, app: &A, surface: CommandSurface, out: &mut SurfaceCommands<'_>);
    #[cfg(feature = "gpu")]
    fn draw_surface_gpu(&self, app: &A, surface: gogo_ui::GpuSurface, pass: &mut wgpu::RenderPass<'_>, place: &gogo_wgpu::SurfacePlace);
    fn show(&mut self, host: &mut Host<'_, A>);
    /// Reads page's handler names from `ui` (`HandlerTable`). Called when
    /// page is made and when a saved file replaced its strings.
    fn read_handler_names(&mut self, ui: &PageUi);
    /// Hands `event` to page under handler name from page's file. Event
    /// with no handler name is not for code.
    fn handle(&mut self, host: &mut Host<'_, A>, event: &Event);
    fn loaded(&mut self, host: &mut Host<'_, A>);
    fn frame(&mut self, host: &mut Host<'_, A>, dt_sec: f32) -> bool;
    fn after_layout(&mut self, host: &mut Host<'_, A>);
    fn arrived(&mut self, host: &mut Host<'_, A>);
    fn left(&mut self, host: &mut Host<'_, A>);
    fn back(&mut self, host: &mut Host<'_, A>) -> Back;
    fn back_closed_popup(&mut self, host: &mut Host<'_, A>);
    /// `CachedPage::reopened` with `fresh`: struct of this page's own
    /// type.
    fn reopened(&mut self, fresh: Box<dyn Any>);
    /// Client's struct, for `reopened`.
    fn into_page(self: Box<Self>) -> Box<dyn Any>;
    /// Client's struct, for a test to read.
    fn page(&self) -> &dyn Any;
    /// Client's struct, to write into (`Nav::tell`).
    fn page_mut(&mut self) -> &mut dyn Any;
}

/// Page's handlers by id of their names in page's strings.
///
/// Needed so finding an event's handler compares no handler name as
/// text: one table read an event. By text, 30 handler names cost 15 to
/// 22 ns an event (measured, release build).
///
/// Read when page is made, again when a saved file replaces page's
/// strings (hot reload). String made later names no handler.
pub(crate) struct HandlerTable<H> {
    /// What each of page's strings is as a node's name, by string index.
    by_string_index: Vec<NameUse<H>>,
}

/// What one string is as a node's name.
#[derive(Clone, Copy)]
enum NameUse<H> {
    Handler(H),
    NoHandler,
    /// Name of a node inside a widget (`volume/thumb`): event's handler
    /// is named by a node above it.
    InsideWidget,
}

impl<H: HandlerName> HandlerTable<H> {
    pub(crate) fn new() -> HandlerTable<H> {
        HandlerTable { by_string_index: Vec::new() }
    }

    /// Reads handler names from strings of `ui`.
    fn read(&mut self, ui: &PageUi) {
        self.by_string_index.clear();
        self.by_string_index.extend(ui.page.strings.iter().map(|string| match H::from_name(string) {
            Some(handler) => NameUse::Handler(handler),
            None if string.contains('/') => NameUse::InsideWidget,
            None => NameUse::NoHandler,
        }));
    }

    /// Handler `event` is for, in page's own enum.
    fn handler_of(&self, ui: &PageUi, event: &Event) -> Option<H> {
        let name: StrId = ui.handler_name_id_with(event, |name| matches!(self.by_string_index.get(name.index()), Some(NameUse::InsideWidget)))?;
        match self.by_string_index.get(name.index()) {
            Some(NameUse::Handler(handler)) => Some(*handler),
            _ => None,
        }
    }
}

pub(crate) struct Plain<P, H>(pub P, pub HandlerTable<H>);

impl<A: 'static, P: Page<A>> PageObject<A> for Plain<P, P::Handler> {
    fn built(&self) -> &'static BuiltPage {
        P::PAGE
    }
    fn page_type(&self) -> TypeId {
        TypeId::of::<P>()
    }
    fn is_cached_page(&self) -> bool {
        false
    }
    fn kind(&self) -> PageKind {
        P::KIND
    }
    fn takes_every_touch(&self) -> bool {
        P::TAKES_EVERY_TOUCH
    }
    fn before_draw(&mut self, host: &mut Host<'_, A>, gpu: &mut PageGpu<'_>) {
        self.0.before_draw(host, gpu);
    }
    fn draw_surface(&self, app: &A, surface: CommandSurface, out: &mut SurfaceCommands<'_>) {
        self.0.draw_surface(app, surface, out);
    }
    #[cfg(feature = "gpu")]
    fn draw_surface_gpu(&self, app: &A, surface: gogo_ui::GpuSurface, pass: &mut wgpu::RenderPass<'_>, place: &gogo_wgpu::SurfacePlace) {
        self.0.draw_surface_gpu(app, surface, pass, place);
    }
    fn show(&mut self, host: &mut Host<'_, A>) {
        self.0.show(host);
    }
    fn read_handler_names(&mut self, ui: &PageUi) {
        self.1.read(ui);
    }
    fn handle(&mut self, host: &mut Host<'_, A>, event: &Event) {
        if let Some(handler) = self.1.handler_of(host.ui, event) {
            self.0.handle(host, PageEvent { handler, kind: event.kind, node: event.node });
        }
    }
    fn loaded(&mut self, host: &mut Host<'_, A>) {
        self.0.loaded(host);
    }
    fn frame(&mut self, host: &mut Host<'_, A>, dt_sec: f32) -> bool {
        self.0.frame(host, dt_sec)
    }
    fn after_layout(&mut self, host: &mut Host<'_, A>) {
        self.0.after_layout(host);
    }
    fn arrived(&mut self, host: &mut Host<'_, A>) {
        self.0.arrived(host);
    }
    fn left(&mut self, host: &mut Host<'_, A>) {
        self.0.left(host);
    }
    fn back(&mut self, host: &mut Host<'_, A>) -> Back {
        self.0.back(host)
    }
    fn back_closed_popup(&mut self, host: &mut Host<'_, A>) {
        self.0.back_closed_popup(host);
    }
    fn reopened(&mut self, _fresh: Box<dyn Any>) {}
    fn into_page(self: Box<Self>) -> Box<dyn Any> {
        Box::new(self.0)
    }
    fn page(&self) -> &dyn Any {
        &self.0
    }
    fn page_mut(&mut self) -> &mut dyn Any {
        &mut self.0
    }
}

pub(crate) struct Cached<P, H>(pub P, pub HandlerTable<H>);

impl<A: 'static, P: CachedPage<A>> PageObject<A> for Cached<P, P::Handler> {
    fn built(&self) -> &'static BuiltPage {
        P::PAGE
    }
    fn page_type(&self) -> TypeId {
        TypeId::of::<P>()
    }
    fn is_cached_page(&self) -> bool {
        true
    }
    fn kind(&self) -> PageKind {
        P::KIND
    }
    fn takes_every_touch(&self) -> bool {
        P::TAKES_EVERY_TOUCH
    }
    fn before_draw(&mut self, host: &mut Host<'_, A>, gpu: &mut PageGpu<'_>) {
        self.0.before_draw(host, gpu);
    }
    fn draw_surface(&self, app: &A, surface: CommandSurface, out: &mut SurfaceCommands<'_>) {
        self.0.draw_surface(app, surface, out);
    }
    #[cfg(feature = "gpu")]
    fn draw_surface_gpu(&self, app: &A, surface: gogo_ui::GpuSurface, pass: &mut wgpu::RenderPass<'_>, place: &gogo_wgpu::SurfacePlace) {
        self.0.draw_surface_gpu(app, surface, pass, place);
    }
    fn show(&mut self, host: &mut Host<'_, A>) {
        self.0.show(host);
    }
    fn read_handler_names(&mut self, ui: &PageUi) {
        self.1.read(ui);
    }
    fn handle(&mut self, host: &mut Host<'_, A>, event: &Event) {
        if let Some(handler) = self.1.handler_of(host.ui, event) {
            self.0.handle(host, PageEvent { handler, kind: event.kind, node: event.node });
        }
    }
    fn loaded(&mut self, host: &mut Host<'_, A>) {
        self.0.loaded(host);
    }
    fn frame(&mut self, host: &mut Host<'_, A>, dt_sec: f32) -> bool {
        self.0.frame(host, dt_sec)
    }
    fn after_layout(&mut self, host: &mut Host<'_, A>) {
        self.0.after_layout(host);
    }
    fn arrived(&mut self, host: &mut Host<'_, A>) {
        self.0.arrived(host);
    }
    fn left(&mut self, host: &mut Host<'_, A>) {
        self.0.left(host);
    }
    fn back(&mut self, host: &mut Host<'_, A>) -> Back {
        self.0.back(host)
    }
    fn back_closed_popup(&mut self, host: &mut Host<'_, A>) {
        self.0.back_closed_popup(host);
    }
    fn reopened(&mut self, fresh: Box<dyn Any>) {
        if let Ok(fresh) = fresh.downcast::<P>() {
            self.0.reopened(*fresh);
        }
    }
    fn into_page(self: Box<Self>) -> Box<dyn Any> {
        Box::new(self.0)
    }
    fn page(&self) -> &dyn Any {
        &self.0
    }
    fn page_mut(&mut self) -> &mut dyn Any {
        &mut self.0
    }
}

/// Marks `IntoPage` of a `Page`.
#[doc(hidden)]
pub struct AsPage;
/// Marks `IntoPage` of a `CachedPage`.
#[doc(hidden)]
pub struct AsCachedPage;

/// What `Nav::go` takes: `Page` or `CachedPage`.
///
/// Needed so opener writes same `go` for both. `Kind` tells compiler which
/// of the two; caller never names it.
pub trait IntoPage<A, Kind> {
    #[doc(hidden)]
    fn into_page_object(self) -> PageBox<A>;
}

/// Page on its way into a program.
#[doc(hidden)]
pub struct PageBox<A>(pub(crate) Box<dyn PageObject<A>>);

impl<A: 'static, P: Page<A>> IntoPage<A, AsPage> for P {
    fn into_page_object(self) -> PageBox<A> {
        PageBox(Box::new(Plain::<P, P::Handler>(self, HandlerTable::new())))
    }
}

impl<A: 'static, P: CachedPage<A>> IntoPage<A, AsCachedPage> for P {
    fn into_page_object(self) -> PageBox<A> {
        PageBox(Box::new(Cached::<P, P::Handler>(self, HandlerTable::new())))
    }
}

/// What a page asked for during a call. Program carries it out when call
/// returns.
pub(crate) enum Request<A> {
    /// Page to open over page on screen.
    Go(Box<dyn PageObject<A>>),
    /// Page to open in place of page on screen.
    Replace(Box<dyn PageObject<A>>),
    Back,
    Preload(Box<dyn PageObject<A>>),
    /// Page to add as a layer above every page.
    Layer(Box<dyn PageObject<A>>),
    /// Page to place inside node `node` of page `parent`.
    Place { parent: PageId, node: NodeId, page: Box<dyn PageObject<A>> },
    /// Page for page `keeper` to keep off screen, `width` x `height` epx.
    KeepOffScreen { keeper: PageId, handle: OffScreenPage, page: Box<dyn PageObject<A>>, width: f32, height: f32 },
    /// Call for the live page whose struct is of type `page_type`.
    Tell { page_type: TypeId, call: TellCall<A> },
}

/// What `Nav::tell` keeps until program carries it out: caller's
/// function, taking the told page's struct as `Any`.
pub(crate) type TellCall<A> = Box<dyn FnOnce(&mut dyn Any, &mut Host<'_, A>)>;

/// Opens another page, asks for back.
pub struct Nav<A = ()> {
    pub(crate) requests: Vec<Request<A>>,
    pub(crate) is_page_cache_enabled: bool,
    /// Page whose function is running now. `None` between calls.
    pub(crate) caller: Option<PageId>,
    /// Handle the next page kept off screen gets.
    next_off_screen_page: u32,
}

impl<A: 'static> Nav<A> {
    pub(crate) fn new() -> Nav<A> {
        Nav { requests: Vec::new(), is_page_cache_enabled: true, caller: None, next_off_screen_page: 0 }
    }

    /// Opens `page` over page on screen. Back returns to page that asked.
    ///
    /// `page` is in place at once, unless its file says how it comes:
    /// its root's `on:show` plays as it opens, its `on:hide` as it
    /// leaves, its `on:uncover` when a fullscreen page over it leaves.
    ///
    /// Cached `CachedPage` shows again same view, and takes `page` over in
    /// its `reopened`.
    pub fn go<P: IntoPage<A, Kind>, Kind>(&mut self, page: P) {
        self.requests.push(Request::Go(page.into_page_object().0));
    }

    /// Opens `page` in place of page on screen: that page leaves,
    /// `page` comes in. Back from `page` returns to page under both.
    ///
    /// Needed for a dialog that leads to another: user then closes one
    /// dialog, not a pile of them.
    ///
    /// Page on screen is not asked (`Page::back` is not called): user
    /// made no back request. Its `left` is called when it is off screen.
    pub fn replace<P: IntoPage<A, Kind>, Kind>(&mut self, page: P) {
        self.requests.push(Request::Replace(page.into_page_object().0));
    }

    /// Adds `page` as a layer: always there, drawn above every page of
    /// the back stack and above layers added before it. For what any
    /// page may show over itself: a toast, a progress bar.
    ///
    /// Layer takes a touch on its own nodes only: a touch beside them
    /// goes to the page under it, whatever layer's `TAKES_EVERY_TOUCH`
    /// says. Back is not asked of it. It lives as long as the program.
    /// Another page reaches it with `tell`.
    pub fn add_layer<P: IntoPage<A, Kind>, Kind>(&mut self, page: P) {
        self.requests.push(Request::Layer(page.into_page_object().0));
    }

    /// Places `page` inside `node` of the page that asks. `page` is laid
    /// out in that node's rectangle, drawn over it, and takes touches
    /// there. It is a page like any other: its own file, its own struct,
    /// its own handlers.
    ///
    /// Needed for a part several pages share (a tool strip), and for a
    /// part one page shows several of.
    ///
    /// Placed page lives until the asking page is freed, or until
    /// another page is placed in same node. It shows and hides with the
    /// asking page, and moves and fades with the node it is placed in.
    ///
    /// WARNING: panics when no page's function is running: only a page
    /// places a page.
    pub fn place<P: IntoPage<A, Kind>, Kind>(&mut self, node: NodeId, page: P) {
        let parent: PageId = self.caller.expect("`Nav::place` is for a page's own function (`loaded`, `show`, `handle`): no page is running now");
        self.requests.push(Request::Place { parent, node, page: page.into_page_object().0 });
    }

    /// Keeps `page` off screen for the page that asks. `page` is a page
    /// like any other (its own file, its own struct, `show`, `frame`,
    /// `after_layout`), laid out `width` x `height` epx at one pixel
    /// per epx. No screen shows it and no finger reaches it: the asking
    /// page draws it, or one node of it, into a texture when it wants
    /// (`PageGpu::draw_to_texture`, in `Page::before_draw`).
    ///
    /// Needed for a picture made of nodes that no screen shows: art
    /// drawn into a collage at the size of the saved file, a panel a
    /// game shows on a wall of its world.
    ///
    /// Kept page lives until the asking page is freed. Its `frame` is
    /// called, and it is laid out, in the frames in which the asking
    /// page is drawn. Another page writes into it with `tell`. Its
    /// `before_draw` is not called, and a page placed in it is not
    /// drawn with it.
    ///
    /// WARNING: panics when no page's function is running: only a page
    /// keeps a page.
    ///
    /// Returns the name the asking page draws it by. Page is made once
    /// the running function returns, and laid out at next frame: drawn
    /// before that, it draws nothing.
    pub fn keep_off_screen<P: IntoPage<A, Kind>, Kind>(&mut self, page: P, width: f32, height: f32) -> OffScreenPage {
        let keeper: PageId = self.caller.expect("`Nav::keep_off_screen` is for a page's own function (`loaded`, `show`, `handle`): no page is running now");
        let handle: OffScreenPage = OffScreenPage(self.next_off_screen_page);
        self.next_off_screen_page += 1;
        self.requests.push(Request::KeepOffScreen { keeper, handle, page: page.into_page_object().0, width, height });
        handle
    }

    /// Calls `call` with the live page whose struct is a `P`, once the
    /// running function returns. Does nothing when no such page is
    /// alive.
    ///
    /// Needed for one page to say something to another: a placed page
    /// to the page it sits in, a page to its tool strip.
    ///
    /// Allocates once per call: for what a user's action causes, not
    /// for every frame.
    pub fn tell<P: 'static>(&mut self, call: impl FnOnce(&mut P, &mut Host<'_, A>) + 'static) {
        let call: TellCall<A> = Box::new(move |page: &mut dyn Any, host: &mut Host<'_, A>| {
            if let Some(page) = page.downcast_mut::<P>() {
                call(page, host);
            }
        });
        self.requests.push(Request::Tell { page_type: TypeId::of::<P>(), call });
    }

    /// Asks for back, as back key does: page on screen answers first
    /// (`Page::back`).
    ///
    /// Needed for back button on a page: same road as the key, so both do
    /// same thing on every page.
    ///
    /// WARNING: a Popup that shows takes this request, as it takes the
    /// key's (`Page::back_closed_popup`). Page that must leave closes
    /// its Popups first.
    pub fn back(&mut self) {
        self.requests.push(Request::Back);
    }

    /// Turns page cache on or off for this device. Program calls it once,
    /// at start: phone with little memory says false.
    ///
    /// Off: every page freed on back, `preload_if_cache_enabled` does
    /// nothing. Pages cached before stay until
    /// `Program::free_cached_pages`.
    pub fn set_page_cache_enabled(&mut self, is_enabled: bool) {
        self.is_page_cache_enabled = is_enabled;
    }

    /// True when page cached on back.
    pub fn is_page_cache_enabled(&self) -> bool {
        self.is_page_cache_enabled
    }

    /// Makes `page` ahead of first open and caches it: that open shows it
    /// at once.
    ///
    /// Does nothing while page cache is off, or when page of that type is
    /// alive already.
    pub fn preload_if_cache_enabled<P: CachedPage<A>>(&mut self, page: P) {
        self.requests.push(Request::Preload(Box::new(Cached::<P, P::Handler>(page, HandlerTable::new()))));
    }
}

#[cfg(test)]
mod tests {
    use gogo_ui::page::Action;

    use super::*;

    #[derive(Clone, Copy, Debug, PartialEq)]
    enum Handler {
        Save,
        CloseNote,
    }

    impl HandlerName for Handler {
        fn from_name(name: &str) -> Option<Handler> {
            match name {
                "save" => Some(Handler::Save),
                "closeNote" => Some(Handler::CloseNote),
                _ => None,
            }
        }
    }

    /// `save` is wired by its own name, `x` by `on:`, `other` names no
    /// handler.
    const BUTTONS: &str = "Button save text=\"Save\"\n        Button x text=\"Close\", on:activate=closeNote\n        Button other text=\"Other\"\n";

    /// Page with `BUTTONS` after `lines_before`.
    fn page_ui(lines_before: &str) -> PageUi {
        let text: String = format!("View root {{\n    Column col {{\n        {lines_before}{BUTTONS}    }}\n}}\n");
        let compiled = gogo_ui::compile(&text, &|_| None);
        assert!(!gogo_ui::has_errors(&compiled.diags), "{:?}", compiled.diags);
        PageUi::new(compiled.page)
    }

    /// Tap's event on node `name`, with node's `on:` action when `is_wired`.
    fn tap_on(ui: &PageUi, name: &str, is_wired: bool) -> Event {
        let action: Option<Action> = if is_wired { ui.page.handlers.iter().map(|handler| handler.action).find(|action| matches!(action, Action::Named(_))) } else { None };
        assert_eq!(action.is_some(), is_wired);
        Event { node: ui.page.find(name).expect("the node"), kind: EventKind::Activate, action }
    }

    fn handlers_of(table: &HandlerTable<Handler>, ui: &PageUi) -> [Option<Handler>; 3] {
        [table.handler_of(ui, &tap_on(ui, "save", false)), table.handler_of(ui, &tap_on(ui, "x", true)), table.handler_of(ui, &tap_on(ui, "other", false))]
    }

    #[test]
    fn an_event_finds_its_handler_by_number() {
        let ui: PageUi = page_ui("");
        let mut table: HandlerTable<Handler> = HandlerTable::new();
        table.read(&ui);
        assert_eq!(handlers_of(&table, &ui), [Some(Handler::Save), Some(Handler::CloseNote), None]);
    }

    /// Hot reload: saved file with a new line before the buttons moves
    /// handler names to other string ids.
    #[test]
    fn a_saved_file_with_new_strings_has_its_handler_names_read_again() {
        let before: PageUi = page_ui("");
        let saved: PageUi = page_ui("Label title text=\"A new line\"\n        ");
        assert_ne!(before.handler_name_id(&tap_on(&before, "save", false)), saved.handler_name_id(&tap_on(&saved, "save", false)), "the save moved the name's id");
        let mut table: HandlerTable<Handler> = HandlerTable::new();
        table.read(&before);
        assert_ne!(handlers_of(&table, &saved), [Some(Handler::Save), Some(Handler::CloseNote), None], "the table of the file before the save is wrong for the saved one");
        table.read(&saved);
        assert_eq!(handlers_of(&table, &saved), [Some(Handler::Save), Some(Handler::CloseNote), None]);
    }
    /// Event on a node inside a widget is for handler named like the
    /// widget's own node: walk passes names inside it (`save/track`).
    #[test]
    fn an_event_inside_a_widget_is_for_the_widgets_own_name() {
        const WIDGET: &str = "control Handle {\n    Layers width=90, height=30 {\n        slot track width=fill, height=fill { Rectangle width=fill, height=fill, color=#333333FF }\n    }\n}\nView root {\n    Column col {\n        Handle save\n    }\n}\n";
        let compiled = gogo_ui::compile(WIDGET, &|_| None);
        assert!(!gogo_ui::has_errors(&compiled.diags), "{:?}", compiled.diags);
        let ui: PageUi = PageUi::new(compiled.page);
        let inside: usize = ui.page.nodes.iter().position(|node| ui.page.str(node.name).contains('/')).expect("a named node inside the widget");
        let mut table: HandlerTable<Handler> = HandlerTable::new();
        table.read(&ui);
        let event: Event = Event { node: NodeId::from_index(inside), kind: EventKind::Activate, action: None };
        assert_eq!(table.handler_of(&ui, &event), Some(Handler::Save), "node `{}`", ui.page.str(ui.page.nodes[inside].name));
        assert_eq!(ui.handler_name(&event), Some("save"));
    }

    /// Every name an event inside a widget is for is a name the build
    /// writes a `Handler` variant for: a button inside a widget with a
    /// plain root, the value of a control with a `behavior` line, and a
    /// signal that control declares.
    ///
    /// Needed because the two are written in two crates: with a name
    /// missing from the build's list, a page's `handle` is never called
    /// for that event.
    #[test]
    fn a_name_an_event_inside_a_widget_is_for_is_one_the_build_writes() {
        const PAGE: &str = "widget Prompt {\n    Column {\n        Button keep text=\"Keep\"\n    }\n}\ncontrol Fader $value=0, $min=0, $max=100 {\n    behavior slider by $value in $min..$max\n    signal drag_started, drag_ended\n    Layers {\n        slot track width=fill, height=24 {\n            slot thumb automatic=position by $value in $min..$max { Circle 12x12 }\n        }\n    }\n}\nView root {\n    Column col {\n        Prompt save\n        Fader gain value=25\n    }\n}\n";
        let compiled = gogo_ui::compile(PAGE, &|_| None);
        assert!(!gogo_ui::has_errors(&compiled.diags), "{:?}", compiled.diags);
        let ui: PageUi = PageUi::new(compiled.page);
        let written: Vec<String> = gogo_ui::emit::handler_names(&ui.page);
        let fader = ui.page.behaviors[0];
        let events: [(Event, &str); 4] = [
            (Event { node: ui.page.find("save/keep").expect("the button"), kind: EventKind::Activate, action: None }, "save"),
            (Event { node: fader.root, kind: EventKind::Edit(gogo_ui::Value::VNum(30.0)), action: None }, "gain"),
            (Event { node: fader.root, kind: EventKind::Signal, action: Some(Action::Emit(fader.drag_started)) }, "drag_started"),
            (Event { node: fader.root, kind: EventKind::Signal, action: Some(Action::Emit(fader.drag_ended)) }, "drag_ended"),
        ];
        for (event, name) in events {
            assert_eq!(ui.handler_name(&event), Some(name));
            assert!(written.iter().any(|written_name| written_name == name), "`{name}` is not among the build's handler names {written:?}");
        }
        let mut table: HandlerTable<Handler> = HandlerTable::new();
        table.read(&ui);
        assert_eq!(table.handler_of(&ui, &events[0].0), Some(Handler::Save));
    }
}
