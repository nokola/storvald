//! What a client writes: a page, and what a page gets with every call.

use std::any::{Any, TypeId};

use gogo_ui::{Event, EventKind, HandlerName, NodeId, StaticPage};

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
