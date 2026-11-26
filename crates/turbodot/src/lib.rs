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
//!   `host.ui` (the page's nodes), `host.nav` (open a page, ask for back),
//!   `host.fonts` (add a font no page's file names: one the user picks).
//!   A page that lets the page under it show, such as a dialog, says
//!   `const KIND: PageKind = PageKind::Overlay;`.
//! - A page with a part its own code draws (a painting, a game's world)
//!   has a `Surface` there in its file. `Surface draw=picture` shows a
//!   texture the page draws in `before_draw`, in the pass that draws
//!   every page, with no copy. `Surface draw=commands` shows gogo draw
//!   commands the page writes in `draw_surface`. `Surface draw=gpu` is
//!   where the page makes GPU draw calls of its own in that same pass
//!   (`draw_surface_gpu`): 3D, a shader of its own. The engine calls
//!   none of them while another page covers the page.
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
/// Texture a page hands `PageUi::set_picture_texture`: its own, wrapped
/// to say what its texels hold.
#[cfg(feature = "gpu")]
pub use gogo_wgpu::Tex_Image_Straight_Srgb_4xU8_RGBA;
/// Texture `PageUi::draw_to_texture` returns: nodes drawn over nothing,
/// wrapped to say that its colors are multiplied by alpha.
#[cfg(feature = "gpu")]
pub use gogo_wgpu::Tex_Tile_Premul_Srgb_4xU8_RGBA;
/// Where a `Surface draw=gpu` is drawn this frame: what
/// `Page::draw_surface_gpu` gets with the pass.
#[cfg(feature = "gpu")]
pub use gogo_wgpu::SurfacePlace;
/// wgpu, which a page draws its own textures with, under TurboDot's own
/// name: client names no wgpu version.
#[cfg(feature = "gpu")]
pub use wgpu;

#[cfg(feature = "gpu")]
pub use gpu::{Buf_Straight_Srgb_4xU8_RGBA, DEPTH_FORMAT, Gpu, Picture};
#[cfg(feature = "ui")]
pub use page::{AsCachedPage, AsPage, Back, BuiltPage, CachedPage, Fonts, Host, IntoPage, Nav, OffScreenPage, Page, PageBox, PageEvent, PageGpu, PageKind};
#[cfg(feature = "ui")]
pub use pages::TexturePlacement;
#[cfg(feature = "ui")]
pub use program::{Built, Program};
#[cfg(feature = "window")]
pub use start::start;
#[cfg(feature = "window")]
pub use window::WindowOptions;

/// What a file with a page needs: `use turbodot::prelude::*;`.
#[cfg(feature = "ui")]
pub mod prelude {
    pub use crate::pages::{PageUi, TexturePlacement};
    pub use crate::{Back, BuiltPage, CachedPage, Fonts, Host, OffScreenPage, Page, PageEvent, PageGpu, PageKind};
    pub use gogo_ui::runtime::{Item, ItemKey, ItemValue};
    pub use gogo_ui::{Buttons, CommandSurface, EventKind, GpuSurface, HoverPhase, Mods, NodeId, Phase, PictureSurface, PointerSample, SurfaceCommands, Tool};
}
