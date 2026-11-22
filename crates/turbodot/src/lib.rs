//! TurboDot: an app and game engine built for AI agents.
//!
//! TurboDot is in development (<https://github.com/nokola/storvald>).
//!
//! A program on TurboDot is pages. Each page is a `.gogo` file and a
//! struct, and the engine does the rest: the window, the back stack, the
//! slides between pages.
//!
//! ```text
//! // build.rs: compiles every page in ui/ when the program is built
//! fn main() { turbodot::build::pages("ui"); }
//!
//! // main.rs
//! turbodot::built_pages!();               // the module `ui`
//! turbodot::start!(AppState, HomePage);   // `main`, on every OS
//!
//! // home_page.rs; its picture is ui/home_page.gogo
//! impl Page<AppState> for HomePage {
//!     const PAGE: &'static BuiltPage = &PAGE;
//!     type Handler = Handler;
//!     fn show(&mut self, host: &mut Host<AppState>) { .. }
//!     fn handle(&mut self, host: &mut Host<AppState>, event: PageEvent<Handler>) {
//!         match event.handler {
//!             Handler::AddNote => host.nav.go(EditPage::new(host.app.notes.len())),
//!         }
//!     }
//! }
//! ```
//!
//! `examples/notes` in the repository is a whole program in this shape.
//!
//! - [`Page`] and [`CachedPage`]: what a client writes per page. [`Host`]
//!   is what a page gets with every call: `host.app` (program-wide state),
//!   `host.ui` (the page's nodes), `host.nav` (open a page, ask for back).
//! - [`Program`] runs the pages with no window and no GPU: a touch goes in
//!   as numbers, and what the pages show is read back. A test and an AI
//!   agent drive a program exactly as a user would.
//! - [`start()`] shows a `Program` in a window. Esc is the back request.
//! - [`Gpu`] is a device with no window. [`Gpu::draw_program`] draws a
//!   program's pages to a [`Picture`] with the renderer a window uses, to
//!   check pixels or to save a PNG and look at it.
//! - [`build`] holds what a client's `build.rs` calls.
//! - [`alloc_count`] counts heap allocations, for a test that says "this
//!   frame allocates nothing".
//!
//! Older, and still here: [`Ui`] holds one page with a text engine of its
//! own, [`run`] shows a `Ui` in a window with an [`App`], and [`pages`]
//! holds the page stack a `Program` is built on.
//!
//! Cargo features: `window` (the default) adds `start`; `gpu` adds `Gpu`;
//! `ui` is `Program` and the pages alone, with no window and no GPU crate.
//! A build script takes `turbodot` with `default-features = false`, which
//! builds gogo's language and nothing else. With `hot-reload`, the pages
//! of a debug build follow their `.gogo` files while the program runs.

pub mod alloc_count;
#[cfg(feature = "ui")]
mod app;
pub mod build;
#[cfg(feature = "gpu")]
mod gpu;
#[cfg(feature = "ui")]
mod page;
#[cfg(feature = "ui")]
mod page_ui;
#[cfg(feature = "ui")]
pub mod pages;
#[cfg(feature = "ui")]
mod program;
#[cfg(feature = "window")]
mod start;
#[cfg(feature = "ui")]
mod ui;
#[cfg(feature = "window")]
mod window;

/// gogo, the UI language pages are written in, under TurboDot's own name:
/// client has one dependency line and names no gogo crate.
pub use gogo_ui as gogo;

#[cfg(feature = "ui")]
pub use app::App;
#[cfg(feature = "gpu")]
pub use gpu::{Buf_Straight_Srgb_4xU8_RGBA, Gpu, Picture};
#[cfg(feature = "ui")]
pub use page::{AsCachedPage, AsPage, Back, BuiltPage, CachedPage, Host, IntoPage, Nav, Page, PageBox, PageEvent};
#[cfg(feature = "ui")]
pub use program::{Built, Program};
#[cfg(feature = "window")]
pub use start::{start, start_main};
#[cfg(feature = "ui")]
pub use ui::{PageSource, Ui};
#[cfg(feature = "window")]
pub use window::{WindowOptions, run};

/// What a file with a page needs: `use turbodot::prelude::*;`.
#[cfg(feature = "ui")]
pub mod prelude {
    pub use crate::pages::HostedPage;
    pub use crate::{Back, BuiltPage, CachedPage, Host, Page, PageEvent};
    pub use gogo_ui::runtime::{Item, ItemKey, ItemValue};
    pub use gogo_ui::{EventKind, NodeId};
}
