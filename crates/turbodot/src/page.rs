//! What a client writes: a page, and what a page gets with every call.

use std::any::{Any, TypeId};

use gogo_ui::{Draw, Event, EventKind, HandlerName, NodeId, StaticPage};

#[cfg(feature = "gpu")]
use crate::gpu::Gpu;
use crate::pages::HostedPage;

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
    /// Page covers whole screen. Once it slid in, pages under it are not
    /// drawn.
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
    /// `HostedPage::row_key` gives the item.
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
    pub ui: &'a mut HostedPage,
    /// Opens another page, asks for back.
    pub nav: &'a mut Nav<A>,
}

/// GPU device a page draws its own textures with
/// (`Page::gpu_draw_textures`). Without cargo feature `gpu` nothing makes
/// one: page code still compiles, and its GPU functions are never called.
#[cfg(not(feature = "gpu"))]
pub struct Gpu(());

/// What a page puts under its own nodes: pictures it drew on GPU
/// (`Page::gpu_draw_screen`).
///
/// Needed so page's picture and page's nodes reach screen in ONE pass,
/// with no copy: on a phone, one more pass over the screen can cost as
/// much as the whole frame.
pub struct Layers<'a> {
    /// Draw commands page puts under its nodes, in paint order.
    pub(crate) draws: &'a mut Vec<Draw>,
    #[cfg(feature = "gpu")]
    pub(crate) renderer: &'a mut gogo_wgpu::UiRenderer,
    #[cfg(feature = "gpu")]
    pub(crate) device: &'a wgpu::Device,
    #[cfg(feature = "gpu")]
    pub(crate) images: &'a mut crate::pages::ImageTable,
    /// Keys of pictures page has shown, by order of its `image` calls in a
    /// frame. Kept from frame to frame: no key is made per frame.
    #[cfg(feature = "gpu")]
    pub(crate) keys: &'a mut Vec<gogo_ui::ImageKey>,
    /// Count of `image` calls this frame.
    #[cfg(feature = "gpu")]
    pub(crate) used: usize,
    /// Page's place in program's page list: makes its picture keys its own.
    #[cfg(feature = "gpu")]
    pub(crate) page_index: usize,
}

impl Layers<'_> {
    /// Puts one gogo draw command under page's nodes, above the ones put
    /// before it. Rectangles in epx, window's space.
    ///
    /// Needed for what `picture` cannot say: part of a texture, a tint, a
    /// turn. Picture's key comes from `image`.
    pub fn draw(&mut self, draw: Draw) {
        self.draws.push(draw);
    }

    /// Key under which `texture` shows in a `Draw::Image`. Renderer samples
    /// page's own texture: no copy.
    ///
    /// Same texture as last frame costs nothing. Another texture in its
    /// place (page draws into two in turn) is taken over.
    #[cfg(feature = "gpu")]
    pub fn image(&mut self, texture: gogo_wgpu::Tex_Image_Straight_Srgb_4xU8_RGBA<'_>) -> gogo_ui::ImageKey {
        if self.used == self.keys.len() {
            self.keys.push(self.images.intern(&format!("turbodot://page/{}/picture/{}", self.page_index, self.used)));
        }
        let key: gogo_ui::ImageKey = self.keys[self.used];
        self.used += 1;
        let external: gogo_wgpu::ExternalImage = self.renderer.external_image(key);
        self.renderer.set_external_image(self.device, external, texture);
        key
    }

    /// Shows `texture` stretched over `rect` (epx, window's space), under
    /// page's nodes and above pictures put before it.
    #[cfg(feature = "gpu")]
    pub fn picture(&mut self, texture: gogo_wgpu::Tex_Image_Straight_Srgb_4xU8_RGBA<'_>, rect: gogo_ui::Rect) {
        let image: gogo_ui::ImageKey = self.image(texture);
        self.draw(Draw::Image { rect, image, tint: gogo_ui::Color_Straight_Srgb_4xU8_RGBA::opaque(0xFFFFFF), fit: gogo_ui::Fit::Distort, window: None, sampling: gogo_ui::Sampling::Smooth });
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
    /// True for page that covers screen with its own GPU picture
    /// (`gpu_draw_screen`) under a see-through root.
    ///
    /// Needed because debug check of `KIND` reads root's background, and
    /// such a page has none. Matters only for page opened over another.
    const DRAWS_GPU_PICTURE: bool = false;

    /// Writes program's data into page: texts, rows, selection.
    ///
    /// Called each time page comes into view: on open, and when page above
    /// it left.
    fn show(&mut self, host: &mut Host<'_, A>);

    /// Says what one user action means.
    fn handle(&mut self, host: &mut Host<'_, A>, event: PageEvent<Self::Handler>);

    /// Page's heavy work. Called once, when page is made.
    ///
    /// Needed because `show` runs on every return to page: costly writes
    /// (long list) go here.
    fn loaded(&mut self, host: &mut Host<'_, A>) {
        let _ = host;
    }

    /// Answers back request: Android back key, Esc on PC, page's own
    /// `Nav::back`.
    ///
    /// Returns `Back::Leave` unless page writes it.
    fn back(&mut self, host: &mut Host<'_, A>) -> Back {
        let _ = host;
        Back::Leave
    }

    /// Page's own GPU work, into its own textures: strokes of a painting,
    /// a game's world. Called each frame before screen is drawn.
    ///
    /// Called only while page is drawn: not for page under a fullscreen
    /// page, not for cached page. Never called without a GPU (a test
    /// that runs `Program` alone).
    fn gpu_draw_textures(&mut self, host: &mut Host<'_, A>, gpu: &Gpu) {
        let _ = (host, gpu);
    }

    /// Says what goes under page's nodes: page's pictures, in paint
    /// order (`Layers::picture`). Called each frame, after
    /// `gpu_draw_textures`, under the same conditions.
    ///
    /// Needed so page's picture reaches screen in the pass that draws
    /// every page: no copy, no pass of its own.
    fn gpu_draw_screen(&self, host: &Host<'_, A>, layers: &mut Layers<'_>) {
        let _ = (host, layers);
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
    /// As `Page::DRAWS_GPU_PICTURE`.
    const DRAWS_GPU_PICTURE: bool = false;

    /// As `Page::show`.
    fn show(&mut self, host: &mut Host<'_, A>);

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

    /// As `Page::back`.
    fn back(&mut self, host: &mut Host<'_, A>) -> Back {
        let _ = host;
        Back::Leave
    }

    /// As `Page::gpu_draw_textures`.
    fn gpu_draw_textures(&mut self, host: &mut Host<'_, A>, gpu: &Gpu) {
        let _ = (host, gpu);
    }

    /// As `Page::gpu_draw_screen`.
    fn gpu_draw_screen(&self, host: &Host<'_, A>, layers: &mut Layers<'_>) {
        let _ = (host, layers);
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
    fn draws_gpu_picture(&self) -> bool;
    #[cfg_attr(not(feature = "gpu"), allow(dead_code))]
    fn gpu_draw_textures(&mut self, host: &mut Host<'_, A>, gpu: &Gpu);
    #[cfg_attr(not(feature = "gpu"), allow(dead_code))]
    fn gpu_draw_screen(&self, host: &Host<'_, A>, layers: &mut Layers<'_>);
    fn show(&mut self, host: &mut Host<'_, A>);
    /// Hands `event` to page under handler name from page's file. Event
    /// with no handler name is not for code.
    fn handle(&mut self, host: &mut Host<'_, A>, event: &Event);
    fn loaded(&mut self, host: &mut Host<'_, A>);
    fn back(&mut self, host: &mut Host<'_, A>) -> Back;
    /// `CachedPage::reopened` with `fresh`: struct of this page's own
    /// type.
    fn reopened(&mut self, fresh: Box<dyn Any>);
    /// Client's struct, for `reopened`.
    fn into_page(self: Box<Self>) -> Box<dyn Any>;
    /// Client's struct, for a test to read.
    fn page(&self) -> &dyn Any;
}

/// Handler `event` is for, in page's own enum.
fn handler_of<H: HandlerName>(ui: &HostedPage, event: &Event) -> Option<H> {
    ui.handler_name(event).and_then(H::from_name)
}

pub(crate) struct Plain<P>(pub P);

impl<A: 'static, P: Page<A>> PageObject<A> for Plain<P> {
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
    fn draws_gpu_picture(&self) -> bool {
        P::DRAWS_GPU_PICTURE
    }
    fn gpu_draw_textures(&mut self, host: &mut Host<'_, A>, gpu: &Gpu) {
        self.0.gpu_draw_textures(host, gpu);
    }
    fn gpu_draw_screen(&self, host: &Host<'_, A>, layers: &mut Layers<'_>) {
        self.0.gpu_draw_screen(host, layers);
    }
    fn show(&mut self, host: &mut Host<'_, A>) {
        self.0.show(host);
    }
    fn handle(&mut self, host: &mut Host<'_, A>, event: &Event) {
        if let Some(handler) = handler_of::<P::Handler>(host.ui, event) {
            self.0.handle(host, PageEvent { handler, kind: event.kind, node: event.node });
        }
    }
    fn loaded(&mut self, host: &mut Host<'_, A>) {
        self.0.loaded(host);
    }
    fn back(&mut self, host: &mut Host<'_, A>) -> Back {
        self.0.back(host)
    }
    fn reopened(&mut self, _fresh: Box<dyn Any>) {}
    fn into_page(self: Box<Self>) -> Box<dyn Any> {
        Box::new(self.0)
    }
    fn page(&self) -> &dyn Any {
        &self.0
    }
}

pub(crate) struct Cached<P>(pub P);

impl<A: 'static, P: CachedPage<A>> PageObject<A> for Cached<P> {
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
    fn draws_gpu_picture(&self) -> bool {
        P::DRAWS_GPU_PICTURE
    }
    fn gpu_draw_textures(&mut self, host: &mut Host<'_, A>, gpu: &Gpu) {
        self.0.gpu_draw_textures(host, gpu);
    }
    fn gpu_draw_screen(&self, host: &Host<'_, A>, layers: &mut Layers<'_>) {
        self.0.gpu_draw_screen(host, layers);
    }
    fn show(&mut self, host: &mut Host<'_, A>) {
        self.0.show(host);
    }
    fn handle(&mut self, host: &mut Host<'_, A>, event: &Event) {
        if let Some(handler) = handler_of::<P::Handler>(host.ui, event) {
            self.0.handle(host, PageEvent { handler, kind: event.kind, node: event.node });
        }
    }
    fn loaded(&mut self, host: &mut Host<'_, A>) {
        self.0.loaded(host);
    }
    fn back(&mut self, host: &mut Host<'_, A>) -> Back {
        self.0.back(host)
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
        PageBox(Box::new(Plain(self)))
    }
}

impl<A: 'static, P: CachedPage<A>> IntoPage<A, AsCachedPage> for P {
    fn into_page_object(self) -> PageBox<A> {
        PageBox(Box::new(Cached(self)))
    }
}

/// What a page asked for during a call. Program carries it out when call
/// returns.
pub(crate) enum Request<A> {
    Go(Box<dyn PageObject<A>>),
    Back,
    Preload(Box<dyn PageObject<A>>),
}

/// Opens another page, asks for back.
pub struct Nav<A = ()> {
    pub(crate) requests: Vec<Request<A>>,
    pub(crate) is_page_cache_enabled: bool,
}

impl<A: 'static> Nav<A> {
    pub(crate) fn new() -> Nav<A> {
        Nav { requests: Vec::new(), is_page_cache_enabled: true }
    }

    /// Opens `page` over page on screen. Back returns to page that asked.
    ///
    /// Cached `CachedPage` shows again same view, and takes `page` over in
    /// its `reopened`.
    pub fn go<P: IntoPage<A, Kind>, Kind>(&mut self, page: P) {
        self.requests.push(Request::Go(page.into_page_object().0));
    }

    /// Asks for back, as back key does: page on screen answers first
    /// (`Page::back`).
    ///
    /// Needed for back button on a page: same road as the key, so both do
    /// same thing on every page.
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
        self.requests.push(Request::Preload(Box::new(Cached(page))));
    }
}
