//! TurboDot: an app and game engine built for AI agents.
//!
//! TurboDot is in development (<https://github.com/nokola/storvald>).
//!
//! A program on TurboDot is pages. Each page is a `.gogo` file and a
//! struct, and the engine does the rest: the window, the back stack,
//! pages that come and go as their files say.
//!
//! ```text
//! // build.rs: compiles every page in ui/ when the program is built
//! fn main() { turbodot::build::pages("ui"); }
//!
//! // main.rs
//! turbodot::built_pages!();               // the module `ui`
//! fn main() -> Result<(), String> {
//!     let program: Program<AppState> = Program::following_page_files(&ui::BUILT, AppState::default(), HomePage)?;
//!     turbodot::start(WindowOptions { title: "Notes", width: 960.0, height: 640.0 }, program)
//! }
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
//!   A page that lets the page under it show, such as a dialog, says
//!   `const KIND: PageKind = PageKind::Overlay;`.
//! - A page with a GPU picture of its own (a painting, a game's world)
//!   writes `gpu_draw_textures` and `gpu_draw_screen`. The picture goes
//!   to the screen in the pass that draws every page, under the page's
//!   nodes, with no copy. The engine calls neither while another page
//!   covers the page.
//! - [`Program`] runs the pages with no window and no GPU: a touch goes in
//!   as numbers, and what the pages show is read back. A test and an AI
//!   agent drive a program exactly as a user would.
//! - [`start()`] shows a `Program` in a window. Esc is the back request.
//! - [`Gpu`] is a device with no window. [`Gpu::draw_program`] draws a
//!   program's pages to a [`Picture`] with the renderer a window uses, to
//!   check pixels or to save a PNG and look at it.
//! - [`gpu_start`] holds what a GPU start decides by: the backends to ask
//!   for, the Vulkan drivers known to be bad, and the file a phone keeps
//!   its backend in.
//! - [`pictures`] holds how a program's pictures get their sizes and
//!   their pixels. A `Program` does it by itself: a page names
//!   `res://assets/x.png` and the picture shows.
//! - [`build`] holds what a client's `build.rs` calls.
//! - [`alloc_count`] counts heap allocations, for a test that says "this
//!   frame allocates nothing".
//! - [`pages`] holds the page stack a `Program` is built on, for a host
//!   that lays out and draws its pages its own way.
//!
//! Cargo features: `window` (the default) adds `start`; `gpu` adds `Gpu`;
//! `ui` is `Program` and the pages alone, with no window and no GPU crate.
//! A build script takes `turbodot` with `default-features = false`, which
//! builds gogo's language and nothing else. With `hot-reload`, the pages
//! of a debug build follow their `.gogo` files while the program runs.

pub mod alloc_count;
pub mod build;
#[cfg(feature = "gpu")]
mod gpu;
#[cfg(feature = "gpu")]
pub mod gpu_start;
#[cfg(feature = "ui")]
mod page;
#[cfg(feature = "ui")]
mod page_ui;
#[cfg(feature = "ui")]
pub mod pages;
#[cfg(feature = "ui")]
pub mod pictures;
#[cfg(feature = "ui")]
mod program;
#[cfg(feature = "window")]
mod start;
#[cfg(feature = "window")]
mod window;

/// gogo, the UI language pages are written in, under TurboDot's own name:
/// client has one dependency line and names no gogo crate.
pub use gogo_ui as gogo;
/// Texture a page hands `Layers::picture`: its own, wrapped to say what
/// its texels hold.
#[cfg(feature = "gpu")]
pub use gogo_wgpu::Tex_Image_Straight_Srgb_4xU8_RGBA;
/// wgpu, which a page draws its own textures with, under TurboDot's own
/// name: client names no wgpu version.
#[cfg(feature = "gpu")]
pub use wgpu;

#[cfg(feature = "gpu")]
pub use gpu::{Buf_Straight_Srgb_4xU8_RGBA, Gpu, Picture};
#[cfg(feature = "ui")]
#[cfg(all(feature = "ui", not(feature = "gpu")))]
pub use page::Gpu;
#[cfg(feature = "ui")]
pub use page::{AsCachedPage, AsPage, Back, BuiltPage, CachedPage, Host, IntoPage, Layers, Nav, Page, PageBox, PageEvent, PageKind};
#[cfg(feature = "ui")]
pub use program::{Built, Program};
#[cfg(feature = "window")]
pub use start::start;
#[cfg(feature = "window")]
pub use window::WindowOptions;

/// What a file with a page needs: `use turbodot::prelude::*;`.
#[cfg(feature = "ui")]
pub mod prelude {
    pub use crate::pages::PageUi;
    pub use crate::{Back, BuiltPage, CachedPage, Gpu, Host, Layers, Page, PageEvent, PageKind};
    pub use gogo_ui::runtime::{Item, ItemKey, ItemValue};
    pub use gogo_ui::{EventKind, NodeId};
}
