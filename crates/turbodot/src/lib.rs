//! TurboDot: an app and game engine built for AI agents.
//!
//! TurboDot is in development (<https://github.com/nokola/storvald>). This
//! version runs one gogo page three ways, with the same app code in each:
//!
//! - [`Ui`] holds a page with its state and its text engine. A touch goes
//!   in as numbers and events come out as values, with no window and no
//!   GPU, so a test or an AI agent drives a page exactly as a user would.
//!   [`Ui::outline`] prints where every node sits, to check a layout by
//!   numbers.
//! - [`Gpu`] is a device with no window. [`Gpu::draw`] draws a `Ui` to a
//!   [`Picture`] with the renderer a window uses, to check pixels or to
//!   save a PNG and look at it.
//! - [`run`] shows a `Ui` in a window and runs an [`App`] on it.
//! - [`pages`] holds several pages in one window: a [`pages::PageStack`]
//!   lays them out and draws them in one order, and gives a touch to the
//!   topmost page that takes it.
//! - [`alloc_count`] counts heap allocations, for a test that says "this
//!   frame allocates nothing".
//!
//! An app says what it writes into its page (`App::show`) and what the
//! page's events mean (`App::handle`):
//!
//! ```text
//! let ui = Ui::new(PageSource::Text(PAGE), &[("Roboto", font_bytes)])?;
//! turbodot::run(WindowOptions { title: "Hello", width: 420.0, height: 640.0 }, ui, Hello::default())?;
//! ```
//!
//! With the cargo feature `hot-reload`, a page that comes from a file
//! (`PageSource::File`) follows that file while the program runs.
//!
//! A crate that holds an app's own code takes `turbodot` with
//! `default-features = false`. It then gets `Ui` and `App` alone, and
//! builds no window and no GPU crate. The cargo feature `gpu` adds `Gpu`,
//! and `window`, which is on by default, adds `run`.

pub mod alloc_count;
mod app;
#[cfg(feature = "gpu")]
mod gpu;
pub mod pages;
mod ui;
#[cfg(feature = "window")]
mod window;

pub use app::App;
#[cfg(feature = "gpu")]
pub use gpu::{Buf_Straight_Srgb_4xU8_RGBA, Gpu, Picture};
pub use ui::{PageSource, Ui};
#[cfg(feature = "window")]
pub use window::{WindowOptions, run};
