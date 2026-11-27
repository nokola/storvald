//! `gogo-text` shapes text and rasterizes glyphs for gogo, in pure Rust
//! (`cosmic-text` + `swash`): no GPU, no window.
//!
//! Needed as its own crate so that `gogo-ui` stays free of a text engine
//! (it measures through `TextMeasure`) and a renderer only uploads what
//! this crate rasterizes.
//!
//! Glyphs go into two atlases, shelf-packed: an 8-bit coverage atlas for
//! letters, tinted by the text color, and an RGBA color atlas for emoji,
//! drawn in their own colors. An atlas is pages of one size (`Atlas`): a
//! glyph that does not fit the page being filled starts a new page, and
//! the pages before keep their glyphs. Past `ATLAS_PAGES_KEPT` pages the
//! page used longest ago is emptied for new glyphs. All sizes here are
//! DEVICE pixels: the caller converts epx (`TextPx::from_epx` for the
//! size of a label's text; `EpxMeasure` does all of it for the layout).
//!
//! A BIG glyph, one drawn wider or taller than `ATLAS_GLYPH_MAX_PX`,
//! goes into neither atlas: the engine keeps its raster by itself
//! (`BigGlyph`), and a renderer gives it a texture of its own. One
//! letter of 400 px otherwise fills a sixth of a new atlas.
//!
//! Emoji come from a COLOR FACE: one with color palettes or color bitmap
//! strikes, such as the phone's own emoji font after
//! `add_fallback_font_file` or the OS scan. A host that installed a
//! painter (`set_host_color_glyph_painter`) paints an emoji first; else swash
//! does (COLRv0 layers and CBDT/sbix bitmaps, so Windows' Segoe UI Emoji
//! draws). Swash draws nothing for the COLRv1 file Android 13+ ships,
//! which is why an Android host paints emoji with the platform's own text
//! stack: a text box then shows exactly the emoji the keyboard offered,
//! on every Android version and from every phone maker.
//!
//! Faces arrive two ways: the page fonts at construction, and later by
//! key. A later face is a file's bytes (`add_face`: the file is parsed
//! when it is added, so a host adds it when the key is first needed and
//! still starts fast) or an OS family after the one system scan
//! (`load_system_fonts` + `add_system_face`). A font key may end in a
//! style suffix (`styled_key`): `#bold` thickens the coverage and
//! `#italic` skews it. Both are faux styles, for a face that has no bold
//! or italic member of its own.

use std::cell::RefCell;
use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::num::NonZeroU32;
use std::ops::Range;
use std::path::Path;
use std::sync::atomic::{AtomicU32, AtomicU64, Ordering};
use std::time::Instant;

use cosmic_text::fontdb;
use cosmic_text::{Align, Attrs, Buffer, CacheKey, CacheKeyFlags, Family, FontSystem, Metrics, Shaping, SwashCache, SwashContent, Weight, Wrap};
use gogo_ui::layout::TextMeasure;
use gogo_ui::ImageKey;

/// The gap between two lines of a label, in epx: none above the first
/// line, none below the last.
///
/// Needed in epx so that wrapped text is set equally tight on every
/// screen. A gap fixed in device pixels is three times as wide, measured
/// against the letters, on a screen with 1 device pixel per epx as on a
/// phone with 3.
pub const LINE_GAP_EPX: f32 = 1.0;

/// The size of a label's text on one screen, in device pixels: its font
/// size and the gap between two of its lines.
///
/// Needed as one value because the layout measures a label and the
/// renderer draws it, and both must turn its size in epx into the same
/// pixels. Measured one pixel bigger than it is drawn, a label wraps a
/// word early; with another gap, its lines sit off the box the layout
/// gave it. `from_epx` is the one place that does the turning.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TextPx {
    /// The font size: a whole number of device pixels, 1 or more.
    pub font_size: f32,
    /// The gap between two lines: a whole number of device pixels.
    pub line_gap: f32,
}

impl TextPx {
    /// The size of text of `font_size_epx` on a screen that has `epx`
    /// device pixels per epx.
    ///
    /// The font size is cut down to a whole pixel. The gap is
    /// `LINE_GAP_EPX` rounded to the nearest whole pixel. Neither goes
    /// under 1.
    ///
    /// Needed whole because glyphs are rasterized on whole pixels: with a
    /// gap of 2.6 pixels the lines of one label would sit 2 and 3 pixels
    /// apart in turns.
    pub fn from_epx(font_size_epx: f32, epx: f32) -> TextPx {
        TextPx { font_size: (font_size_epx * epx).floor().max(1.0), line_gap: (LINE_GAP_EPX * epx).round().max(1.0) }
    }

    /// A size for text that is one line, where no gap shows: one glyph
    /// (`TextEngine::glyph`), or letters rasterized ahead of their first
    /// frame (`TextEngine::warm`).
    fn one_line(font_size: f32) -> TextPx {
        TextPx { font_size, line_gap: 0.0 }
    }
}

/// Entries a measure cache or a shape cache holds before it is emptied.
///
/// Needed because a label that changes every few frames (a frame counter,
/// a zoom readout) is a new string each time, so an unbounded cache would
/// grow for as long as the app runs.
///
/// Emptying costs one re-shape per visible string on the next frame, far
/// below a frame.
pub const CACHE_CAP: usize = 4096;

/// Faux bold: how far the coverage grows on every side, in pixels per
/// pixel of font size. 0.6 px at a 24 px face.
///
/// Needed as a share of the size so that `#bold` looks equally heavy at
/// every font size.
pub const BOLD_GROW_PER_PX: f32 = 0.6 / 24.0;

/// The widest and the tallest a glyph's raster may be, in device pixels,
/// to go into an atlas. A wider or taller one is a big glyph
/// (`BigGlyph`).
///
/// Needed because an atlas is packed in shelves as tall as their tallest
/// glyph. One letter of 400 px takes four tenths of a page 1024 a side
/// and leaves the rest of its shelf to small letters, so a page fills
/// after two of them.
pub const ATLAS_GLYPH_MAX_PX: u32 = 128;

/// The side of one page of the letters' atlas, in texels: 1 MB of
/// coverage.
///
/// Needed as pages of one size so a full atlas never doubles. A doubling
/// made a texture of 4 MB, sent it to the GPU whole and drew every letter
/// on screen again: 20 to 40 ms in one frame on an old phone.
pub const ATLAS_PAGE_SIDE: u32 = 1024;

/// The side of one page of the emoji atlas, in texels: 1 MB of RGBA.
pub const COLOR_ATLAS_PAGE_SIDE: u32 = 512;

/// How many pages an atlas holds before it empties the page used longest
/// ago for new glyphs.
///
/// Needed so a session that shows many fonts and sizes does not keep
/// every letter it ever drew.
///
/// A page that a text of the frame being drawn uses is never emptied
/// (`TextEngine::start_frame`): an atlas whose pages are all on screen
/// gets one more.
pub const ATLAS_PAGES_KEPT: usize = 4;

/// The bytes of big glyphs' texels an engine keeps before
/// `TextEngine::start_frame` drops some.
///
/// Needed because a glyph stretched over a box (`TextEngine::glyph`) is
/// a new raster at every size of the box: a box resized by a drag leaves
/// one big glyph behind per frame.
pub const BIG_GLYPH_KEPT_BYTES: usize = 4 * 1024 * 1024;

/// The font-key style suffix (`"Title#bold"`) is gogo vocabulary: the
/// app spells it, this engine reads it.
pub use gogo_ui::font_style::{split_style, styled_key};

/// A face's vertical metrics in font units, read from the file at load.
#[derive(Clone, Copy, Debug)]
pub struct FaceMetrics {
    pub ascent: f32,
    pub descent: f32,
    pub units_per_em: f32,
}

impl FaceMetrics {
    /// The metrics a face gets when its file carries none: Roboto's hhea
    /// metrics (1900 / 500 over 2048).
    ///
    /// Needed so that text in such a face still gets a line height. The
    /// engine answers with it too while no face is loaded.
    pub const FALLBACK: FaceMetrics = FaceMetrics { ascent: 1900.0, descent: 500.0, units_per_em: 2048.0 };

    /// One line's own height in px at `font_size` px (no spacing): a
    /// single-line label's box. The scaled ascent and the scaled descent
    /// are each rounded UP to a whole pixel, the way FreeType sizes a
    /// face (`FT_Size_Metrics`).
    ///
    /// Needed because glyphs are rasterized on whole pixels, so a line
    /// needs both rounded up: 24 px Roboto is 23 + 6 = 29 px. Rounding
    /// the sum instead gives round(28.13) = 28, one pixel short on every
    /// row of a list.
    pub fn font_height(&self, font_size: f32) -> f32 {
        let scale = font_size / self.units_per_em;
        (self.ascent * scale).ceil() + (self.descent * scale).ceil()
    }

    /// The pitch between lines: the face height plus the line gap.
    pub fn line_height(&self, size: TextPx) -> f32 {
        self.font_height(size.font_size) + size.line_gap
    }

    /// The box height of `lines` lines: the gap sits only BETWEEN lines
    /// (lines × font height + (lines − 1) × the line gap).
    pub fn block_height(&self, size: TextPx, lines: f32) -> f32 {
        let n = lines.max(1.0);
        n * self.font_height(size.font_size) + (n - 1.0) * size.line_gap
    }
}

/// Where a glyph sits in its atlas plus its placement relative to the pen.
#[derive(Clone, Copy, Debug)]
struct Slot {
    u0: f32,
    v0: f32,
    u1: f32,
    v1: f32,
    left: i32,
    top: i32,
    width: u32,
    height: u32,
    /// In the color atlas (an emoji), not the coverage atlas.
    color: bool,
    /// The page of its atlas the uv is into.
    page: u32,
    /// The glyph is a big one: its uv is into that glyph's own raster,
    /// not into an atlas.
    big_glyph: Option<BigGlyphId>,
}

impl Slot {
    /// The slot of a glyph that draws nothing (a space).
    fn without_ink(color: bool) -> Slot {
        Slot { u0: 0.0, v0: 0.0, u1: 0.0, v1: 0.0, left: 0, top: 0, width: 0, height: 0, color, page: 0, big_glyph: None }
    }

    /// Whether the glyph's texels are on page `page` of its atlas: a
    /// glyph with ink, not a big one.
    fn is_on_page(&self, page: usize) -> bool {
        self.big_glyph.is_none() && self.width > 0 && self.page as usize == page
    }
}

/// One glyph to draw: a rect in device pixels and the uv of its texels.
#[derive(Clone, Copy, Debug)]
pub struct GlyphQuad {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
    pub uv: [f32; 4],
    /// The texels are colors (an emoji): draw them as they are, the text
    /// color's alpha only. False = coverage, tinted by the text color.
    ///
    /// Unless `big_glyph` names one, the uv is into the color atlas
    /// (`TextEngine::color_atlas`) when true, into the letters' atlas
    /// (`TextEngine::atlas`) when false.
    pub color: bool,
    /// The page of that atlas the uv is into (`Atlas::page`). 0 for a big
    /// glyph and for one with no ink.
    pub page: u32,
    /// The glyph is a big one (`BigGlyph`): the uv is into that glyph's
    /// own raster (`TextEngine::big_glyph`), not into an atlas.
    pub big_glyph: Option<BigGlyphId>,
}

/// Names one big glyph (`BigGlyph`). No two big glyphs of a process have
/// the same one, and a glyph drawn again after its raster was dropped
/// gets a new one.
///
/// Needed for the whole process, not per engine, so a renderer that drew
/// with one engine and is handed another never takes the second
/// engine's glyph for one it has a texture of.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct BigGlyphId(NonZeroU32);

impl BigGlyphId {
    /// An id no big glyph of this process has had.
    fn next() -> BigGlyphId {
        static NEXT_BIG_GLYPH_ID: AtomicU32 = AtomicU32::new(1);
        loop {
            // 0 comes up once the counter has gone round: skipped.
            if let Some(id) = NonZeroU32::new(NEXT_BIG_GLYPH_ID.fetch_add(1, Ordering::Relaxed)) {
                return BigGlyphId(id);
            }
        }
    }
}

/// The raster of one big glyph: a glyph drawn wider or taller than
/// `ATLAS_GLYPH_MAX_PX`, which the engine keeps out of its atlases.
///
/// Needed by a renderer: it makes one texture of `width` x `height` for
/// the glyph, from `texels`, the first time a quad names it
/// (`GlyphQuad::big_glyph`), and draws the quad's uv from that texture.
/// It frees the texture once `TextEngine::big_glyph` answers `None` for
/// the id.
///
/// The ink has one empty texel on every side, and a quad's uv leaves
/// that border out. So a glyph stretched over a box fades out at its
/// edges into nothing, whatever the texture does outside 0..1.
#[derive(Clone, Copy, Debug)]
pub struct BigGlyph<'a> {
    pub width: u32,
    pub height: u32,
    /// The texels are straight RGBA, 4 bytes each (an emoji). False =
    /// coverage, 1 byte each.
    pub is_color: bool,
    /// Rows from the top, `width` texels each.
    pub texels: &'a [u8],
}

/// A big glyph as the engine keeps it.
#[derive(Clone, Debug)]
struct KeptBigGlyph {
    width: u32,
    height: u32,
    is_color: bool,
    texels: Vec<u8>,
    /// `TextEngine::big_glyph_clock` when a text last shaped the glyph.
    shaped_at: u64,
}

/// Turns `rgba` texels from premultiplied color to straight color.
///
/// Needed because swash hands an emoji's rows premultiplied, and the
/// engine keeps straight color (a renderer's image quads premultiply on
/// sample).
fn straight_from_premultiplied(rgba: &mut [u8]) {
    for texel in rgba.chunks_exact_mut(4) {
        let a = texel[3] as u32;
        if a != 0 && a != 255 {
            for c in &mut texel[..3] {
                *c = ((*c as u32 * 255 + a / 2) / a).min(255) as u8;
            }
        }
    }
}

/// One emoji the host painted (`HostColorGlyphPainter`): straight (not
/// premultiplied) RGBA rows, and where the ink sits against the pen —
/// `left` px right of it, `top` px above the baseline (FreeType's
/// bitmap_left / bitmap_top).
///
/// Needed so a raster from outside the engine (Android's text stack)
/// lands in the color atlas like swash's own: same rows, same placement.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HostColorGlyph {
    pub width: u32,
    pub height: u32,
    pub left: i32,
    pub top: i32,
    pub rgba: Vec<u8>,
}

/// The host's emoji painter: the text of one shaped cluster ("😀",
/// "👨‍👩‍👧", a flag pair) and the font size in device px; `None` = the
/// host draws nothing for it (the engine then tries swash).
pub type HostColorGlyphPainter = Box<dyn FnMut(&str, f32) -> Option<HostColorGlyph>>;

/// A page id or a page version no page of this process has had, never 0.
///
/// Needed for the whole process, not per engine, so a renderer that drew
/// with one engine and is handed another never takes the second engine's
/// page for the one it uploaded (`Atlas::part_to_send`).
fn next_atlas_version() -> u64 {
    static NEXT_ATLAS_VERSION: AtomicU64 = AtomicU64::new(1);
    NEXT_ATLAS_VERSION.fetch_add(1, Ordering::Relaxed)
}

/// What a renderer has sent to its GPU of one page of an atlas: it keeps
/// one per page texture it holds, and hands it to `Atlas::part_to_send`.
///
/// Needed so a renderer sends only the part it has not sent. A new
/// letter changes a small rectangle; sending 4 MB of letters took 11 to
/// 17 ms on an old phone, for one letter.
///
/// `AtlasSent::default()` stands for a texture that holds nothing yet: a
/// renderer's new texture, all zeros. Its first send is what the page
/// holds so far, not the whole page.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct AtlasSent {
    /// The page's version at the send.
    version: u64,
    /// Which page was sent (`AtlasPage::id`); 0 for none.
    page_id: u64,
    /// The first row of the shelf that was being filled at the send. A
    /// later glyph may have landed in that shelf; no row above it has
    /// changed.
    open_row: u32,
    /// How far across that shelf was filled at the send: a later glyph
    /// in it lies to the right, and nothing to the left has changed.
    open_x: u32,
}

/// A rectangle of an atlas page, in texels: the part a renderer has not
/// sent to its GPU (`Atlas::part_to_send`).
///
/// Needed as a rectangle, not as a run of rows: a new letter beside
/// letters already sent changes a few columns of its shelf.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AtlasPart {
    pub columns: Range<u32>,
    pub rows: Range<u32>,
}

impl AtlasPart {
    /// Whether there is nothing to send.
    pub fn is_empty(&self) -> bool {
        self.columns.is_empty() || self.rows.is_empty()
    }

    /// How many texels the part holds.
    pub fn texel_count(&self) -> u64 {
        u64::from(self.columns.end.saturating_sub(self.columns.start)) * u64::from(self.rows.end.saturating_sub(self.rows.start))
    }
}

/// A shelf packer over one square page: slots go left to right along a
/// row, a new row starts when one does not fit.
///
/// Needed because both atlases (coverage and color) pack the same way:
/// one packer, two sizes.
#[derive(Clone, Copy, Debug)]
struct Shelf {
    size: u32,
    x: u32,
    y: u32,
    h: u32,
}

impl Shelf {
    fn new(size: u32) -> Shelf {
        Shelf { size, x: 0, y: 0, h: 0 }
    }

    /// The top-left of a `w` × `h` slot, or `None` when the page is full.
    fn pack(&mut self, w: u32, h: u32) -> Option<(u32, u32)> {
        if self.x + w > self.size {
            self.y += self.h;
            self.x = 0;
            self.h = 0;
        }
        if self.y + h > self.size {
            return None;
        }
        let pos = (self.x, self.y);
        self.x += w;
        self.h = self.h.max(h);
        Some(pos)
    }

    /// The part a renderer at `sent` has not got of the page `page_id`
    /// this shelf packs, now at `version`.
    ///
    /// Needed in one place because every page answers it the same way.
    /// Slots are packed left to right, shelf below shelf. So nothing
    /// above the shelf that was being filled at the last send has
    /// changed, and in that shelf nothing to the left of where it was
    /// filled to.
    ///
    /// A send that starts a new shelf sends whole rows, the earlier
    /// slots of the shelf above included: once per shelf, not per
    /// letter.
    ///
    /// Returns the part, and what the renderer has sent once it sent
    /// it. Nothing when the version is the one sent. For a texture that
    /// holds nothing, what the page holds so far: as if the page had
    /// been sent when it was empty. The whole page for a texture that
    /// holds another page. The new slots' columns of the shelf when that
    /// shelf is still being filled. Else whole rows, from that shelf down
    /// to the last one used.
    fn part_to_send(&self, page_id: u64, version: u64, sent: AtlasSent) -> (AtlasPart, AtlasSent) {
        let from: AtlasSent = if sent.page_id == 0 { AtlasSent { version: 0, page_id, open_row: 0, open_x: 0 } } else { sent };
        let part: AtlasPart = if from.version == version {
            AtlasPart { columns: 0..0, rows: 0..0 }
        } else if from.page_id != page_id {
            AtlasPart { columns: 0..self.size, rows: 0..self.size }
        } else if from.open_row == self.y {
            AtlasPart { columns: from.open_x..self.x.min(self.size), rows: self.y..self.y + self.h }
        } else {
            AtlasPart { columns: 0..self.size, rows: from.open_row..self.y + self.h }
        };
        (part, AtlasSent { version, page_id, open_row: self.y, open_x: self.x })
    }
}

/// One page of an atlas: `side` x `side` texels, shelf-packed.
struct AtlasPage {
    texels: Vec<u8>,
    shelf: Shelf,
    /// A number no other page of this process has: new when the page is
    /// made and each time it is emptied, since any texel may differ then.
    id: u64,
    /// New each time a slot is packed into the page.
    version: u64,
    /// `TextEngine::frame` when a text last used a glyph of the page.
    used_in_frame: u64,
}

impl AtlasPage {
    fn new(side: u32, texel_bytes: u32, frame: u64) -> AtlasPage {
        AtlasPage { texels: vec![0; (side * side * texel_bytes) as usize], shelf: Shelf::new(side), id: next_atlas_version(), version: next_atlas_version(), used_in_frame: frame }
    }

    /// Clears every texel and packs from the top left again.
    fn empty(&mut self, frame: u64) {
        self.texels.fill(0);
        self.shelf = Shelf::new(self.shelf.size);
        self.id = next_atlas_version();
        self.version = next_atlas_version();
        self.used_in_frame = frame;
    }
}

/// What a pack did to its atlas's pages besides packing.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum PageChange {
    None,
    /// The slot is on a new page.
    Added,
    /// The slot is on a page that held other glyphs, emptied for it.
    Emptied,
}

/// Where `Atlas::pack` put a slot: its page and its top left corner.
struct Packed {
    page: usize,
    x: u32,
    y: u32,
    change: PageChange,
}

/// The bit of page `page` in a set of pages (`KeptShape::letter_pages`).
/// Page 63 and every page after it share the last bit.
fn page_bit(page: u32) -> u64 {
    1u64 << page.min(63)
}

/// One atlas of a text engine, the letters' (8-bit coverage) or the
/// emoji's (straight RGBA), in pages of one size.
///
/// Needed as pages so a full atlas never doubles: a glyph that does not
/// fit the page being filled starts a new page, and the glyphs on the
/// pages before stay where they are. No glyph is drawn again, no text is
/// shaped again, and a renderer sends only the new page's part.
///
/// A renderer holds a texture per page: it makes one the first time a
/// quad names the page (`GlyphQuad::page`), and sends each page's new
/// part once a frame (`part_to_send`).
pub struct Atlas {
    side: u32,
    texel_bytes: u32,
    pages: Vec<AtlasPage>,
    /// The page slots are packed into.
    filling: usize,
}

impl Atlas {
    /// An atlas of no page yet: the first glyph makes one.
    fn new(side: u32, texel_bytes: u32) -> Atlas {
        Atlas { side, texel_bytes, pages: Vec::new(), filling: 0 }
    }

    /// The side of every page, in texels.
    pub fn side(&self) -> u32 {
        self.side
    }

    /// Bytes a texel: 1 for the letters' coverage, 4 for the emoji's
    /// RGBA.
    pub fn texel_bytes(&self) -> u32 {
        self.texel_bytes
    }

    /// How many pages the atlas has: 0 until its first glyph.
    pub fn page_count(&self) -> usize {
        self.pages.len()
    }

    /// The texels of page `index`: rows from the top, `side` texels each.
    /// The texel at column `x` of row `y` starts at byte
    /// `(y * side + x) * texel_bytes`.
    pub fn page(&self, index: usize) -> &[u8] {
        &self.pages[index].texels
    }

    /// The part of page `index` that a renderer at `sent` has not sent to
    /// its GPU. A renderer calls it once a frame for each page it holds a
    /// texture of, after the frame's texts are shaped, and writes that
    /// rectangle of `page(index)` into the texture: all the frame's new
    /// glyphs on that page in one write.
    ///
    /// Needed because a frame that draws one new letter otherwise sends a
    /// whole page. Each renderer keeps its own `AtlasSent` per page, so
    /// several renderers over one engine each get the part they lack.
    ///
    /// Returns the part, and what the renderer keeps as sent once it
    /// wrote it. An empty part when nothing was packed since `sent`. What
    /// the page holds so far for a new texture (`AtlasSent::default()`).
    /// The whole page for a texture that holds another page: another
    /// engine's, or this one before it was emptied.
    pub fn part_to_send(&self, index: usize, sent: AtlasSent) -> (AtlasPart, AtlasSent) {
        let page: &AtlasPage = &self.pages[index];
        page.shelf.part_to_send(page.id, page.version, sent)
    }

    /// A place for a `w` x `h` slot: on the page being filled, else on a
    /// new page, and past `ATLAS_PAGES_KEPT` pages on the page used
    /// longest ago, emptied. A page a text of frame `frame` used is never
    /// emptied: then a new page is made.
    ///
    /// Panics on a slot larger than a page; `ATLAS_GLYPH_MAX_PX` keeps
    /// every glyph far smaller.
    fn pack(&mut self, w: u32, h: u32, frame: u64) -> Packed {
        if let Some(page) = self.pages.get_mut(self.filling) {
            if let Some((x, y)) = page.shelf.pack(w, h) {
                page.version = next_atlas_version();
                page.used_in_frame = frame;
                return Packed { page: self.filling, x, y, change: PageChange::None };
            }
        }
        let unused: Option<usize> = if self.pages.len() < ATLAS_PAGES_KEPT { None } else { self.pages.iter().enumerate().filter(|(_, page)| page.used_in_frame < frame).min_by_key(|(_, page)| page.used_in_frame).map(|(index, _)| index) };
        let change: PageChange = match unused {
            Some(index) => {
                self.pages[index].empty(frame);
                self.filling = index;
                PageChange::Emptied
            }
            None => {
                self.pages.push(AtlasPage::new(self.side, self.texel_bytes, frame));
                self.filling = self.pages.len() - 1;
                PageChange::Added
            }
        };
        let page: &mut AtlasPage = &mut self.pages[self.filling];
        let (x, y): (u32, u32) = page.shelf.pack(w, h).expect("a slot fits an empty page");
        page.version = next_atlas_version();
        Packed { page: self.filling, x, y, change }
    }

    /// Notes that a text of frame `frame` uses the pages `pages` names
    /// (`page_bit`): none of them is emptied in that frame.
    fn note_used(&mut self, pages: u64, frame: u64) {
        let mut left: u64 = pages;
        while left != 0 {
            let index: usize = left.trailing_zeros() as usize;
            left &= left - 1;
            // The last bit stands for page 63 and every page after it.
            let last: usize = if index == 63 { self.pages.len() } else { index + 1 };
            for page in self.pages.iter_mut().take(last).skip(index) {
                page.used_in_frame = frame;
            }
        }
    }
}

/// A loaded font file: which family/weight `Attrs` asks for, and its
/// vertical metrics.
#[derive(Clone, Debug)]
struct Face {
    /// The face in the font database: a glyph the shaper took from
    /// another face carries another id (`is_settled`).
    id: fontdb::ID,
    family: String,
    weight: Weight,
    metrics: FaceMetrics,
}

/// A text's measured size as the engine keeps it.
#[derive(Clone, Copy, Debug)]
struct KeptMeasure {
    size: (f32, f32),
    /// See `TextEngine::is_settled`.
    is_settled: bool,
}

/// A text's shaped quads as the engine keeps them, the pen at the
/// origin. Their uvs are into atlas pages: a page emptied for new glyphs
/// drops every kept shape that names it.
#[derive(Clone, Debug)]
struct KeptShape {
    quads: Vec<GlyphQuad>,
    /// See `TextEngine::is_settled`.
    is_settled: bool,
    /// Whether a quad names a big glyph. Such a shape tells its big
    /// glyphs each time it is asked for (`note_big_glyphs_shaped`), and
    /// is dropped with the first of them that is dropped.
    has_big_glyph: bool,
    /// The pages of the letters' atlas its quads are on (`page_bit`).
    /// The shape tells them each time it is asked for, so no page it
    /// shows is emptied in that frame.
    letter_pages: u64,
    /// `letter_pages`, of the emoji atlas.
    emoji_pages: u64,
}

/// Marks the big glyphs `quads` name as shaped now: they are the last
/// that `TextEngine::start_frame` drops.
///
/// A function over the two fields, not a method, so a caller can hold a
/// kept shape of the same engine while it calls.
fn note_big_glyphs_shaped(big_glyphs: &mut HashMap<BigGlyphId, KeptBigGlyph>, clock: &mut u64, quads: &[GlyphQuad]) {
    for id in quads.iter().filter_map(|quad| quad.big_glyph) {
        if let Some(kept) = big_glyphs.get_mut(&id) {
            *clock += 1;
            kept.shaped_at = *clock;
        }
    }
}

/// The engine's costly steps since the last `TextEngine::take_work`:
/// how often each ran, and for how long.
///
/// Needed to see where a slow frame's text time goes, and whether a text
/// is shaped more than once. A text is shaped when it is measured and
/// again when it is drawn: each shows here as a count.
///
/// A step is counted only when it ran. A text the engine has measured or
/// shaped before costs a lookup and counts nothing, so a frame that
/// shows nothing new reads all zeros.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct TextWork {
    /// Texts shaped to measure them (`measure`, `narrowest_wrap`): each
    /// a text, size, width and font not measured before.
    pub shapes_to_measure: u32,
    pub seconds_shaping_to_measure: f32,
    /// Texts shaped to draw them (`shape`, `shape_into`): each a text not
    /// shaped before, or one whose kept shape went with its page or with
    /// a big glyph.
    pub shapes_to_draw: u32,
    /// Time of those shapes, without the letters they drew.
    pub seconds_shaping_to_draw: f32,
    /// Letters drawn into an atlas or as a big glyph: at first sight, and
    /// again after their page was emptied or their big glyph dropped.
    pub letters_drawn: u32,
    pub seconds_drawing_letters: f32,
    /// Atlas pages made (`ATLAS_PAGE_SIDE`, `COLOR_ATLAS_PAGE_SIDE`): a
    /// renderer makes a texture for each.
    pub pages_added: u32,
    /// Atlas pages emptied for new glyphs (`ATLAS_PAGES_KEPT`). Each
    /// drops the kept shapes and the glyphs on that page: a renderer
    /// sends the page whole again.
    pub pages_emptied: u32,
}

pub struct TextEngine {
    fs: FontSystem,
    cache: SwashCache,
    /// path key (as written in the .gogo) → face.
    faces: HashMap<String, Face>,
    default_face: Option<Face>,
    /// The letters' atlas: 8-bit coverage.
    letters: Atlas,
    /// (glyph, faux bold) → its coverage-atlas slot; a bold glyph is its
    /// own raster.
    slots: HashMap<(CacheKey, bool), Option<Slot>>,
    /// The emoji atlas: straight RGBA.
    emoji: Atlas,
    /// A swash-painted color glyph → its color-atlas slot.
    color_slots: HashMap<CacheKey, Option<Slot>>,
    /// A host-painted cluster (its text, the size's bits) → its
    /// color-atlas slot.
    host_slots: HashMap<(String, u32), Option<Slot>>,
    /// Whether a face is a color face (palettes or color strikes), by id —
    /// read from the file once.
    color_faces: HashMap<fontdb::ID, bool>,
    /// The host's emoji painter, when it has one (Android).
    host_color_glyph_painter: Option<HostColorGlyphPainter>,
    /// The OS font scan ran (once per engine).
    system_loaded: bool,
    /// Family names the scan found, sorted, each once.
    system_families: Vec<String>,
    /// Goes up by one at each `start_frame`. A page a text used in this
    /// frame (`AtlasPage::used_in_frame`) is not emptied.
    frame: u64,
    /// Keyed by a hash of (text, size, width, font) — no String is built
    /// to look a measure up (a page lays out every animated frame). The
    /// engine serves every page, and their `StrId`s overlap, so the text
    /// itself is hashed, not its id. A 64-bit collision would return a
    /// wrong size for one string in ~10^19; accepted.
    measure_cache: HashMap<u64, KeptMeasure>,
    /// Keyed by a hash of (text, size, width, font, box_w, halign).
    shape_cache: HashMap<u64, KeptShape>,
    /// The big glyphs kept: the ones a slot or a kept shape may name.
    big_glyphs: HashMap<BigGlyphId, KeptBigGlyph>,
    /// The bytes of all `big_glyphs`' texels.
    big_glyph_bytes: usize,
    /// Goes up by one each time a text shapes a big glyph. The order in
    /// which big glyphs were last shaped is the order of their
    /// `shaped_at`.
    big_glyph_clock: u64,
    /// `big_glyph_clock` at the last `start_frame`: a big glyph shaped
    /// later was shaped since that call.
    big_glyphs_checked_at: u64,
    /// What `take_work` returns.
    work: TextWork,
}

/// Stands where the text alignment goes in the cache key of a
/// `TextEngine::narrowest_wrap` answer. Needed so that answer never shares
/// an entry with a `measure` of the same text: alignments are 0, 1 and 2.
const NARROWEST_WRAP_KEY: u8 = u8::MAX;

/// The measure / shape cache key: a hash over the request, no allocation.
fn cache_key(text: &str, size: TextPx, max_width: Option<f32>, font: Option<&str>, box_w: f32, halign: u8) -> u64 {
    let mut h = std::collections::hash_map::DefaultHasher::new();
    text.hash(&mut h);
    size.font_size.to_bits().hash(&mut h);
    size.line_gap.to_bits().hash(&mut h);
    max_width.unwrap_or(-1.0).to_bits().hash(&mut h);
    font.unwrap_or("").hash(&mut h);
    box_w.to_bits().hash(&mut h);
    halign.hash(&mut h);
    h.finish()
}

impl TextEngine {
    /// `fonts`: (key as the .gogo names it, file bytes). The first is the
    /// default face.
    pub fn new(fonts: &[(&str, Vec<u8>)]) -> TextEngine {
        let mut db = cosmic_text::fontdb::Database::new();
        let mut loaded: Vec<(String, cosmic_text::fontdb::ID, String, Weight)> = Vec::new();
        for (key, bytes) in fonts {
            let before: Vec<_> = db.faces().map(|f| f.id).collect();
            db.load_font_data(bytes.clone());
            if let Some(face) = db.faces().find(|f| !before.contains(&f.id)) {
                let family = face.families.first().map(|(n, _)| n.clone()).unwrap_or_default();
                loaded.push((key.to_string(), face.id, family, face.weight));
            }
        }
        // Only the fonts handed in. The system-font scan costs hundreds of
        // milliseconds on a phone, so it runs when a host asks for it
        // (`load_system_fonts`), never here.
        let mut fs = FontSystem::new_with_locale_and_db("en-US".to_string(), db);
        let mut faces = HashMap::new();
        let mut default_face = None;
        for (key, id, family, weight) in loaded {
            let metrics = metrics_of(&mut fs, id, weight);
            let f = Face { id, family, weight, metrics };
            if default_face.is_none() {
                default_face = Some(f.clone());
            }
            faces.insert(key, f);
        }
        TextEngine {
            fs,
            cache: SwashCache::new(),
            faces,
            default_face,
            letters: Atlas::new(ATLAS_PAGE_SIDE, 1),
            slots: HashMap::new(),
            emoji: Atlas::new(COLOR_ATLAS_PAGE_SIDE, 4),
            color_slots: HashMap::new(),
            host_slots: HashMap::new(),
            color_faces: HashMap::new(),
            host_color_glyph_painter: None,
            system_loaded: false,
            system_families: Vec::new(),
            frame: 0,
            measure_cache: HashMap::new(),
            shape_cache: HashMap::new(),
            big_glyphs: HashMap::new(),
            big_glyph_bytes: 0,
            big_glyph_clock: 0,
            big_glyphs_checked_at: 0,
            work: TextWork::default(),
        }
    }

    /// Returns what the engine's costly steps did since the last call,
    /// and starts counting anew.
    ///
    /// Needed by a host that prints where a slow frame went: it takes
    /// the work once a frame, after the frame's text was drawn.
    pub fn take_work(&mut self) -> TextWork {
        std::mem::take(&mut self.work)
    }

    /// Adds a face that `font=` or a draw can name by `key`; `bytes` is a
    /// TTF/OTF file. An existing key keeps its face.
    ///
    /// Needed for fonts that are not page fonts: the ones a user picks
    /// from a list, or a file fetched after start.
    ///
    /// WARNING: parses the file now. Call it when the key is first
    /// needed, not at start: parsing 28 font files took about a second on
    /// a phone.
    ///
    /// Returns false when the bytes hold no face.
    pub fn add_face(&mut self, key: &str, bytes: Vec<u8>) -> bool {
        if self.faces.contains_key(key) {
            return true;
        }
        let db = self.fs.db_mut();
        let before: usize = db.faces().count();
        db.load_font_data(bytes);
        let Some((id, family, weight)) = db.faces().nth(before).map(|f| (f.id, f.families.first().map(|(n, _)| n.clone()).unwrap_or_default(), f.weight)) else {
            return false;
        };
        let metrics = metrics_of(&mut self.fs, id, weight);
        self.faces.insert(key.to_string(), Face { id, family, weight, metrics });
        true
    }

    /// Whether `key` (style suffix ignored) names a registered face.
    pub fn has_face(&self, key: &str) -> bool {
        self.faces.contains_key(split_style(key).0)
    }

    /// The one OS font scan: every face under the system font folders
    /// joins the database (mapped, not copied), and their family names
    /// become `system_families`. A second call does nothing.
    ///
    /// Needed for a font picker that lists the device's fonts, and for
    /// script fallback: a glyph no page font has is then looked up across
    /// the OS faces instead of drawing as a box.
    ///
    /// WARNING: hundreds of milliseconds on a phone. Call it when such a
    /// picker first opens, never at start. A host that needs the letters
    /// of one script adds that one file instead
    /// (`add_fallback_font_file`), and scans only when the file is
    /// missing.
    ///
    /// Returns the family count.
    pub fn load_system_fonts(&mut self) -> usize {
        if self.system_loaded {
            return self.system_families.len();
        }
        self.system_loaded = true;
        let db = self.fs.db_mut();
        let before: usize = db.faces().count();
        #[cfg(target_os = "android")]
        {
            // fontdb scans no Android folder itself; these are where every
            // Android build keeps its fonts.
            for dir in ["/system/fonts", "/system/font", "/data/fonts", "/product/fonts"] {
                db.load_fonts_dir(dir);
            }
        }
        #[cfg(not(target_os = "android"))]
        {
            db.load_system_fonts();
        }
        let mut names: Vec<String> = db.faces().skip(before).filter_map(|f| f.families.first().map(|(n, _)| n.clone())).collect();
        names.sort();
        names.dedup();
        self.system_families = names;
        self.forget_unsettled_texts();
        self.system_families.len()
    }

    /// The family names `load_system_fonts` found (empty before it ran).
    pub fn system_families(&self) -> &[String] {
        &self.system_families
    }

    /// One font FILE joins the database as a fallback face: a glyph no
    /// page font has is then looked up in it. Mapped, not copied (a .ttc
    /// adds every face it holds).
    ///
    /// Needed so that a language in another script costs ONE file, not
    /// the whole OS scan: Roboto has no Arabic letters, so an Arabic page
    /// draws every label as a row of boxes until the phone's Arabic font
    /// (Noto Naskh Arabic) is added here.
    ///
    /// The face gets no key, so `font=` cannot name it. The kept measure
    /// and shape of a label measured as boxes are dropped, because they
    /// are wrong now (`forget_unsettled_texts`); the atlas and its
    /// glyphs stay.
    ///
    /// Returns false when the file is missing, unreadable, or holds no
    /// face.
    pub fn add_fallback_font_file(&mut self, path: &Path) -> bool {
        let db = self.fs.db_mut();
        let before: usize = db.faces().count();
        if db.load_font_file(path).is_err() || db.faces().count() == before {
            return false;
        }
        self.forget_unsettled_texts();
        true
    }

    /// Whether `buffer`, a text laid out in the face `font` names, is
    /// settled: every glyph of it is a glyph of that face.
    ///
    /// Needed when a font is added later. The shaper asks the text's own
    /// face first, so a text that face drew whole comes out the same
    /// with any number of fonts added. A glyph from another face, or the
    /// box of a letter no face had, may come out of the new font.
    fn is_settled(&self, buffer: &Buffer, font: Option<&str>) -> bool {
        let Some(face) = self.face(font) else { return false };
        buffer.layout_runs().all(|run| run.glyphs.iter().all(|glyph| glyph.font_id == face.id && glyph.glyph_id != 0))
    }

    /// Drops the kept measure and shape of every text that is not
    /// settled (`is_settled`). Called when a font joins the database.
    ///
    /// Needed in place of dropping them all: the frame after the
    /// device's fonts were read measured and shaped every text on
    /// screen again, 80 texts in 35 ms on an old phone, and none came
    /// out different.
    fn forget_unsettled_texts(&mut self) {
        self.measure_cache.retain(|_, kept| kept.is_settled);
        self.shape_cache.retain(|_, kept| kept.is_settled);
    }

    /// Whether any loaded face (page fonts, fallback files, the OS scan)
    /// has a glyph for `ch`. Parses every face on the way: a load-time
    /// or test answer, never per frame.
    pub fn has_glyph(&mut self, ch: char) -> bool {
        let ids: Vec<fontdb::ID> = self.fs.db_mut().faces().map(|f| f.id).collect();
        // The cmap, not `unicode_codepoints` (empty without cosmic's
        // monospace_fallback feature); glyph 0 is .notdef, the box.
        ids.into_iter().any(|id| self.fs.get_font(id, Weight::NORMAL).is_some_and(|f| f.as_swash().charmap().map(ch) != 0))
    }

    /// A face `key` names from the OS family `family` (one of
    /// `system_families`): its regular member when it has one. Returns
    /// false for an unknown family.
    pub fn add_system_face(&mut self, key: &str, family: &str) -> bool {
        if self.faces.contains_key(key) {
            return true;
        }
        let found = self
            .fs
            .db()
            .faces()
            .filter(|f| f.families.iter().any(|(n, _)| n == family))
            .min_by_key(|f| (f.style != fontdb::Style::Normal, (f.weight.0 as i32 - Weight::NORMAL.0 as i32).abs()))
            .map(|f| (f.id, f.weight));
        let Some((id, weight)) = found else {
            return false;
        };
        let metrics = metrics_of(&mut self.fs, id, weight);
        self.faces.insert(key.to_string(), Face { id, family: family.to_string(), weight, metrics });
        true
    }

    /// The one glyph of `ch` at `font_size` px with the pen at the origin:
    /// its ink box and atlas uv, or None when the face draws nothing for
    /// it. The `text_fit=stretch` word maps this box onto a Label's rect.
    pub fn glyph(&mut self, ch: &str, font_size: f32, font: Option<&str>) -> Option<GlyphQuad> {
        self.shape(ch, TextPx::one_line(font_size), None, font, 0.0, 0.0, 0.0, 0).into_iter().find(|g| g.w > 0.0 && g.h > 0.0)
    }

    /// The letters' atlas: 8-bit coverage, in pages.
    pub fn atlas(&self) -> &Atlas {
        &self.letters
    }

    /// The emoji atlas: straight RGBA, in pages.
    pub fn color_atlas(&self) -> &Atlas {
        &self.emoji
    }

    /// Installs (or removes) the host's emoji painter. A load-time call:
    /// drops the shape cache and every host-painted slot, so a cluster
    /// the old painter drew is painted again by the new one.
    pub fn set_host_color_glyph_painter(&mut self, host: Option<HostColorGlyphPainter>) {
        self.host_color_glyph_painter = host;
        // A big glyph the old painter drew goes with its slot.
        for id in self.host_slots.values().filter_map(|slot| slot.and_then(|s| s.big_glyph)) {
            if let Some(kept) = self.big_glyphs.remove(&id) {
                self.big_glyph_bytes -= kept.texels.len();
            }
        }
        self.host_slots.clear();
        self.shape_cache.clear();
    }

    /// The raster of the big glyph a quad names (`GlyphQuad::big_glyph`).
    ///
    /// Needed by a renderer, to make the glyph's texture the first time
    /// it draws it, and to free that texture later: see `BigGlyph`.
    ///
    /// Returns `None` once the engine dropped the glyph (`start_frame`),
    /// and for another engine's id. An id out of a quad shaped since the
    /// last `start_frame` always has its raster.
    pub fn big_glyph(&self, id: BigGlyphId) -> Option<BigGlyph<'_>> {
        self.big_glyphs.get(&id).map(|kept| BigGlyph { width: kept.width, height: kept.height, is_color: kept.is_color, texels: &kept.texels })
    }

    /// Starts a frame. A renderer calls it once a frame, before it shapes
    /// the frame's texts.
    ///
    /// Drops big glyphs while their texels hold more than
    /// `BIG_GLYPH_KEPT_BYTES`: the one shaped longest ago first, and
    /// never one a text shaped since the call before this one. From the
    /// call on, an atlas page a text of the frame uses is not emptied
    /// for new glyphs (`ATLAS_PAGES_KEPT`).
    ///
    /// Needed because the engine cannot tell by itself what is still
    /// shown, and a box resized by a drag leaves a big glyph behind per
    /// frame (`BIG_GLYPH_KEPT_BYTES`). Between two calls a renderer
    /// shapes all it shows, so what the last frame showed is never
    /// dropped, however much it is: a screen of big letters does not
    /// have them drawn again every frame.
    ///
    /// WARNING: without this call no big glyph is ever dropped and no
    /// page is emptied: an atlas whose pages are full gets more.
    ///
    /// A dropped glyph's id is dead: the kept shapes that name it are
    /// dropped with it, and the glyph is drawn again, under a new id,
    /// when a text next needs it.
    pub fn start_frame(&mut self) {
        self.frame += 1;
        let shaped_since: u64 = std::mem::replace(&mut self.big_glyphs_checked_at, self.big_glyph_clock);
        let mut is_any_dropped: bool = false;
        while self.big_glyph_bytes > BIG_GLYPH_KEPT_BYTES {
            let longest_ago: Option<BigGlyphId> = self.big_glyphs.iter().filter(|(_, kept)| kept.shaped_at <= shaped_since).min_by_key(|(_, kept)| kept.shaped_at).map(|(id, _)| *id);
            let Some(id) = longest_ago else { break };
            if let Some(kept) = self.big_glyphs.remove(&id) {
                self.big_glyph_bytes -= kept.texels.len();
            }
            is_any_dropped = true;
        }
        if !is_any_dropped {
            return;
        }
        let big_glyphs: &HashMap<BigGlyphId, KeptBigGlyph> = &self.big_glyphs;
        let is_kept = |slot: &Option<Slot>| slot.and_then(|s| s.big_glyph).is_none_or(|id| big_glyphs.contains_key(&id));
        self.slots.retain(|_, slot| is_kept(slot));
        self.color_slots.retain(|_, slot| is_kept(slot));
        self.host_slots.retain(|_, slot| is_kept(slot));
        self.shape_cache.retain(|_, kept| !kept.has_big_glyph || kept.quads.iter().all(|quad| quad.big_glyph.is_none_or(|id| big_glyphs.contains_key(&id))));
    }

    /// Whether the face `id` carries color glyphs: color palettes (COLR,
    /// any version) or color bitmap strikes (CBDT, sbix).
    ///
    /// Needed to tell an emoji from a letter: an emoji goes to the host
    /// painter, a letter never does. Read once per face; a face that
    /// cannot be opened is not one.
    fn is_color_face(&mut self, id: fontdb::ID) -> bool {
        if let Some(c) = self.color_faces.get(&id) {
            return *c;
        }
        let is_color: bool = self.fs.get_font(id, Weight::NORMAL).is_some_and(|f| {
            let s = f.as_swash();
            s.color_palettes().len() > 0 || s.color_strikes().len() > 0
        });
        self.color_faces.insert(id, is_color);
        is_color
    }

    /// The language text is shaped for (a BCP-47 tag: `de`, `bn-IN`). Line
    /// breaking and script fallback follow it, so this moves with the
    /// string table (`gogo_ui::Locale`) — translated strings alone would break
    /// mid-glyph in a script the old tag never expected. Rebuilds the
    /// font system on the SAME font database (no file re-read) and drops
    /// the measure/shape caches; the atlas and its glyphs stay. A load-time
    /// call, not per frame.
    pub fn set_locale(&mut self, bcp47: &str) {
        if self.fs.locale() == bcp47 {
            return;
        }
        let placeholder = FontSystem::new_with_locale_and_db(String::new(), fontdb::Database::new());
        let (_, db) = std::mem::replace(&mut self.fs, placeholder).into_locale_and_db();
        self.fs = FontSystem::new_with_locale_and_db(bcp47.to_string(), db);
        self.measure_cache.clear();
        self.shape_cache.clear();
    }

    /// The face `font` names (its style suffix ignored), else the default.
    fn face(&self, font: Option<&str>) -> Option<&Face> {
        font.and_then(|f| self.faces.get(split_style(f).0)).or(self.default_face.as_ref())
    }

    /// (bold, italic) as the key's suffix spells them.
    fn style(font: Option<&str>) -> (bool, bool) {
        font.map_or((false, false), |f| {
            let (_, b, i) = split_style(f);
            (b, i)
        })
    }

    fn attrs<'a>(&'a self, font: Option<&str>) -> Attrs<'a> {
        // Faux italic is the shaper's own 14° skew; faux bold is ours, at
        // raster time (`rasterize`).
        let (_, italic) = Self::style(font);
        let flags = if italic { CacheKeyFlags::FAKE_ITALIC } else { CacheKeyFlags::empty() };
        match self.face(font) {
            Some(f) => Attrs::new().family(Family::Name(&f.family)).weight(f.weight).cache_key_flags(flags),
            None => Attrs::new().cache_key_flags(flags),
        }
    }

    /// The metrics of the face `font` names, else the default face's, else
    /// `FaceMetrics::FALLBACK`.
    pub fn metrics(&self, font: Option<&str>) -> FaceMetrics {
        self.face(font).map_or(FaceMetrics::FALLBACK, |f| f.metrics)
    }

    fn buffer(&mut self, text: &str, size: TextPx, max_width: Option<f32>, font: Option<&str>) -> Buffer {
        self.buffer_wrapped(text, size, max_width, font, if max_width.is_some() { Wrap::WordOrGlyph } else { Wrap::None })
    }

    /// `text` shaped into lines no wider than `max_width`, broken the way
    /// `wrap` says.
    fn buffer_wrapped(&mut self, text: &str, size: TextPx, max_width: Option<f32>, font: Option<&str>, wrap: Wrap) -> Buffer {
        let line_height = self.metrics(font).line_height(size);
        let mut buffer = Buffer::new(&mut self.fs, Metrics::new(size.font_size, line_height));
        buffer.set_wrap(wrap);
        buffer.set_size(max_width, None);
        let attrs = self.attrs(font);
        buffer.set_text(text, &attrs, Shaping::Advanced, None);
        // Every line starts at the box's left edge; `halign` then shifts it
        // (`shape_pass`). Needed because cosmic-text's default puts a
        // right-to-left line (Arabic, Urdu, Hebrew) against the RIGHT edge
        // of the wrap width: a left-aligned label as wide as its row would
        // show عربي at the far right of the row. The letters still run
        // right to left inside the line; only where the line sits changes.
        for line in buffer.lines.iter_mut() {
            line.set_align(Some(Align::Left));
        }
        buffer.shape_until_scroll(&mut self.fs, false);
        buffer
    }

    /// Width and height in device pixels.
    pub fn measure(&mut self, text: &str, size: TextPx, max_width: Option<f32>, font: Option<&str>) -> (f32, f32) {
        let key = cache_key(text, size, max_width, font, 0.0, 0);
        if let Some(kept) = self.measure_cache.get(&key) {
            return kept.size;
        }
        let started: Instant = Instant::now();
        let buffer = self.buffer(text, size, max_width, font);
        self.work.shapes_to_measure += 1;
        self.work.seconds_shaping_to_measure += started.elapsed().as_secs_f32();
        let mut w: f32 = 0.0;
        let mut lines: f32 = 0.0;
        for run in buffer.layout_runs() {
            w = w.max(run.line_w);
            lines += 1.0;
        }
        let h = self.metrics(font).block_height(size, lines);
        let v = (w.ceil(), h);
        if self.measure_cache.len() >= CACHE_CAP {
            self.measure_cache.clear();
        }
        let is_settled: bool = self.is_settled(&buffer, font);
        self.measure_cache.insert(key, KeptMeasure { size: v, is_settled });
        v
    }

    /// The narrowest `text` folds to, in device pixels: the width of its
    /// longest word, with lines ending only where a word ends.
    ///
    /// Needed because `measure` at a wrap width of 0 breaks between
    /// letters and answers one letter's width; a wrapping label sized by
    /// that wraps one letter per line.
    pub fn narrowest_wrap(&mut self, text: &str, size: TextPx, font: Option<&str>) -> f32 {
        // The answer shares the measure cache under a key of its own: a
        // text alignment no `measure` asks with (it asks with 0).
        let key = cache_key(text, size, None, font, 0.0, NARROWEST_WRAP_KEY);
        if let Some(kept) = self.measure_cache.get(&key) {
            return kept.size.0;
        }
        let started: Instant = Instant::now();
        let buffer = self.buffer_wrapped(text, size, Some(0.0), font, Wrap::Word);
        self.work.shapes_to_measure += 1;
        self.work.seconds_shaping_to_measure += started.elapsed().as_secs_f32();
        let mut w: f32 = 0.0;
        for run in buffer.layout_runs() {
            // The ink of the line: from its first letter to its last, so
            // the space a line ends on does not widen the word.
            //
            // WARNING: a glyph's `start..end` are bytes of ITS line
            // (`run.text`), not of `text`: they agree only on a text's
            // first line.
            let from: f32 = run.glyphs.iter().filter(|g| !run.text[g.start..g.end].trim().is_empty()).map(|g| g.x).fold(f32::MAX, f32::min);
            let to: f32 = run.glyphs.iter().filter(|g| !run.text[g.start..g.end].trim().is_empty()).map(|g| g.x + g.w).fold(0.0, f32::max);
            if to > from {
                w = w.max(to - from);
            }
        }
        let w: f32 = w.ceil();
        if self.measure_cache.len() >= CACHE_CAP {
            self.measure_cache.clear();
        }
        let is_settled: bool = self.is_settled(&buffer, font);
        self.measure_cache.insert(key, KeptMeasure { size: (w, 0.0), is_settled });
        w
    }

    /// Shapes `text` into glyph quads with the pen at (x, y) = the top-left
    /// of the text box; rasterizes new glyphs into the atlas. `halign`:
    /// 0 start, 1 center, 2 end within `box_w` (when wrapping).
    pub fn shape(&mut self, text: &str, size: TextPx, max_width: Option<f32>, font: Option<&str>, x: f32, y: f32, box_w: f32, halign: u8) -> Vec<GlyphQuad> {
        let mut out: Vec<GlyphQuad> = Vec::new();
        self.shape_into(text, size, max_width, font, x, y, box_w, halign, &mut out);
        out
    }

    /// `shape` appending to `out` instead of returning a vector.
    ///
    /// Needed because a renderer shapes every label of every frame
    /// through here: a run the cache holds then costs no allocation, so a
    /// frame that shows only cached text allocates nothing.
    #[allow(clippy::too_many_arguments)]
    pub fn shape_into(&mut self, text: &str, size: TextPx, max_width: Option<f32>, font: Option<&str>, x: f32, y: f32, box_w: f32, halign: u8, out: &mut Vec<GlyphQuad>) {
        let (dx, dy) = (x.round(), y.round());
        let key = cache_key(text, size, max_width, font, box_w, halign);
        if let Some(kept) = self.shape_cache.get(&key) {
            out.extend(kept.quads.iter().map(|q| GlyphQuad { x: q.x + dx, y: q.y + dy, ..*q }));
            if kept.has_big_glyph {
                note_big_glyphs_shaped(&mut self.big_glyphs, &mut self.big_glyph_clock, &kept.quads);
            }
            self.letters.note_used(kept.letter_pages, self.frame);
            self.emoji.note_used(kept.emoji_pages, self.frame);
            return;
        }
        let kept: KeptShape = self.shape_uncached(text, size, max_width, font, box_w, halign);
        if self.shape_cache.len() >= CACHE_CAP {
            self.shape_cache.clear();
        }
        out.extend(kept.quads.iter().map(|q| GlyphQuad { x: q.x + dx, y: q.y + dy, ..*q }));
        self.shape_cache.insert(key, kept);
    }

    /// Shapes `text` into quads with the pen at the origin, and notes
    /// which atlas pages and big glyphs they use.
    fn shape_uncached(&mut self, text: &str, size: TextPx, max_width: Option<f32>, font: Option<&str>, box_w: f32, halign: u8) -> KeptShape {
        let mut out: Vec<GlyphQuad> = Vec::new();
        let is_settled: bool = self.shape_pass(text, size, max_width, font, box_w, halign, &mut out);
        // A big glyph this run found drawn is shaped now, like one it
        // drew.
        note_big_glyphs_shaped(&mut self.big_glyphs, &mut self.big_glyph_clock, &out);
        let has_big_glyph: bool = out.iter().any(|quad| quad.big_glyph.is_some());
        let (mut letter_pages, mut emoji_pages): (u64, u64) = (0, 0);
        for quad in out.iter().filter(|quad| quad.big_glyph.is_none()) {
            if quad.color {
                emoji_pages |= page_bit(quad.page);
            } else {
                letter_pages |= page_bit(quad.page);
            }
        }
        KeptShape { quads: out, is_settled, has_big_glyph, letter_pages, emoji_pages }
    }

    /// One shaping pass into `out`.
    ///
    /// Returns whether the text is settled (`is_settled`).
    fn shape_pass(&mut self, text: &str, size: TextPx, max_width: Option<f32>, font: Option<&str>, box_w: f32, halign: u8, out: &mut Vec<GlyphQuad>) -> bool {
        let (x, y) = (0.0f32, 0.0f32);
        let (bold, _) = Self::style(font);
        let started: Instant = Instant::now();
        let buffer = self.buffer(text, size, max_width, font);
        self.work.shapes_to_draw += 1;
        self.work.seconds_shaping_to_draw += started.elapsed().as_secs_f32();
        let is_settled: bool = self.is_settled(&buffer, font);
        // Per glyph: its raster key, its face, and the bytes of its line it
        // covers (the cluster an emoji painter is asked for).
        let lines: Vec<&str> = buffer.lines.iter().map(|l| l.text()).collect();
        let runs: Vec<(f32, f32, Vec<(cosmic_text::PhysicalGlyph, fontdb::ID, &str)>)> = buffer
            .layout_runs()
            .map(|run| (run.line_w, run.line_y, run.glyphs.iter().map(|g| (g.physical((0.0, 0.0), 1.0), g.font_id, &lines[run.line_i][g.start..g.end])).collect()))
            .collect();
        for (line_w, line_y, glyphs) in runs {
            let shift = match halign {
                1 => ((box_w - line_w) / 2.0).max(0.0),
                2 => (box_w - line_w).max(0.0),
                _ => 0.0,
            };
            for (g, font_id, cluster) in glyphs {
                let Some(slot) = self.slot(g.cache_key, bold, font_id, cluster) else { continue };
                if slot.width == 0 || slot.height == 0 {
                    continue;
                }
                // Before the next glyph is packed: a glyph that empties a
                // page for itself never takes this one's.
                if slot.big_glyph.is_none() {
                    let atlas: &mut Atlas = if slot.color { &mut self.emoji } else { &mut self.letters };
                    atlas.note_used(page_bit(slot.page), self.frame);
                }
                out.push(GlyphQuad {
                    x: (x + shift).round() + g.x as f32 + slot.left as f32,
                    // cosmic-text puts half of the line gap above each
                    // line; `block_height` counts none above the first, so
                    // every row moves up by half a gap.
                    y: y.round() + (line_y - size.line_gap / 2.0).round() + g.y as f32 - slot.top as f32,
                    w: slot.width as f32,
                    h: slot.height as f32,
                    uv: [slot.u0, slot.v0, slot.u1, slot.v1],
                    color: slot.color,
                    page: slot.page,
                    big_glyph: slot.big_glyph,
                });
            }
        }
        is_settled
    }

    /// The slot of one glyph, rasterizing it on first sight. An emoji (a
    /// glyph of a color face) goes to the host painter when there is one,
    /// else to swash; a letter to swash.
    fn slot(&mut self, key: CacheKey, bold: bool, font_id: fontdb::ID, cluster: &str) -> Option<Slot> {
        if self.host_color_glyph_painter.is_some() && self.is_color_face(font_id) {
            let host_key = (cluster.to_string(), key.font_size_bits);
            if let Some(s) = self.host_slots.get(&host_key) {
                return *s;
            }
            if let Some(slot) = self.host_slot(cluster, key.font_size_bits) {
                self.host_slots.insert(host_key, slot);
                return slot;
            }
            // The host drew nothing for it: swash's turn, below.
        }
        if let Some(s) = self.slots.get(&(key, bold)) {
            return *s;
        }
        if let Some(s) = self.color_slots.get(&key) {
            return *s;
        }
        let started: Instant = Instant::now();
        let slot = self.rasterize(key, bold);
        self.work.letters_drawn += 1;
        self.work.seconds_drawing_letters += started.elapsed().as_secs_f32();
        if slot.is_some_and(|s| s.color) {
            self.color_slots.insert(key, slot);
        } else {
            self.slots.insert((key, bold), slot);
        }
        slot
    }

    /// The host's raster of `cluster` packed into the color atlas; `None`
    /// = the host drew nothing (the caller falls back to swash).
    fn host_slot(&mut self, cluster: &str, font_size_bits: u32) -> Option<Option<Slot>> {
        let painted: HostColorGlyph = (self.host_color_glyph_painter.as_mut()?)(cluster, f32::from_bits(font_size_bits))?;
        if painted.width == 0 || painted.height == 0 || painted.rgba.len() != (painted.width * painted.height * 4) as usize {
            return Some(Some(Slot::without_ink(true)));
        }
        if painted.width.max(painted.height) > ATLAS_GLYPH_MAX_PX {
            return Some(Some(self.keep_big_glyph(&painted.rgba, painted.width, painted.height, painted.left, painted.top, true)));
        }
        Some(Some(self.pack_color(&painted.rgba, painted.width, painted.height, painted.left, painted.top, false)))
    }

    fn rasterize(&mut self, key: CacheKey, bold: bool) -> Option<Slot> {
        let image = self.cache.get_image(&mut self.fs, key).clone();
        let slot = image.and_then(|img| {
            let (mut w, mut h) = (img.placement.width, img.placement.height);
            let (mut left, mut top) = (img.placement.left, img.placement.top);
            if img.content == SwashContent::Color {
                // A color bitmap or COLRv0 layers (swash hands RGBA rows):
                // the color atlas, never emboldened.
                if w == 0 || h == 0 {
                    return Some(Slot::without_ink(true));
                }
                if w.max(h) > ATLAS_GLYPH_MAX_PX {
                    let mut straight: Vec<u8> = img.data;
                    straight_from_premultiplied(&mut straight);
                    self.cache.image_cache.remove(&key);
                    return Some(self.keep_big_glyph(&straight, w, h, left, top, true));
                }
                return Some(self.pack_color(&img.data, w, h, left, top, true));
            }
            if img.content != SwashContent::Mask {
                return None;
            }
            if w == 0 || h == 0 {
                return Some(Slot::without_ink(false));
            }
            let mut data: std::borrow::Cow<[u8]> = std::borrow::Cow::Borrowed(&img.data);
            if bold {
                let grow: u32 = (f32::from_bits(key.font_size_bits) * BOLD_GROW_PER_PX).round().max(1.0) as u32;
                data = std::borrow::Cow::Owned(embolden(&img.data, w, h, grow));
                w += 2 * grow;
                h += 2 * grow;
                left -= grow as i32;
                top += grow as i32;
            }
            if w.max(h) > ATLAS_GLYPH_MAX_PX {
                // The shaper's own copy of the raster goes: the engine
                // keeps the one of the big glyph.
                self.cache.image_cache.remove(&key);
                return Some(self.keep_big_glyph(&data, w, h, left, top, false));
            }
            let (page, px, py): (usize, u32, u32) = self.pack(false, w + 1, h + 1);
            let side: u32 = self.letters.side;
            let texels: &mut [u8] = &mut self.letters.pages[page].texels;
            for row in 0..h {
                let src = &data[(row * w) as usize..((row + 1) * w) as usize];
                let dst = ((py + row) * side + px) as usize;
                texels[dst..dst + w as usize].copy_from_slice(src);
            }
            let s = side as f32;
            Some(Slot { u0: px as f32 / s, v0: py as f32 / s, u1: (px + w) as f32 / s, v1: (py + h) as f32 / s, left, top, width: w, height: h, color: false, page: page as u32, big_glyph: None })
        });
        slot
    }

    /// Keeps `texels`, the `w` x `h` raster of a glyph too big for an
    /// atlas (`ATLAS_GLYPH_MAX_PX`), as a big glyph of its own: coverage
    /// bytes, or straight RGBA when `is_color`.
    ///
    /// Needed in one place because a letter, a swash-painted emoji and a
    /// host-painted emoji all end here.
    ///
    /// The kept raster is two texels wider and taller: the ink with one
    /// empty texel on every side (`BigGlyph`).
    ///
    /// Returns the glyph's slot, its uv over the ink alone.
    fn keep_big_glyph(&mut self, texels: &[u8], w: u32, h: u32, left: i32, top: i32, is_color: bool) -> Slot {
        let texel_bytes: usize = if is_color { 4 } else { 1 };
        let (width, height): (u32, u32) = (w + 2, h + 2);
        let (ink_row_bytes, row_bytes): (usize, usize) = (w as usize * texel_bytes, width as usize * texel_bytes);
        let mut bordered: Vec<u8> = vec![0; row_bytes * height as usize];
        for row in 0..h as usize {
            let first: usize = (row + 1) * row_bytes + texel_bytes;
            bordered[first..first + ink_row_bytes].copy_from_slice(&texels[row * ink_row_bytes..(row + 1) * ink_row_bytes]);
        }
        let id: BigGlyphId = BigGlyphId::next();
        self.big_glyph_clock += 1;
        self.big_glyph_bytes += bordered.len();
        self.big_glyphs.insert(id, KeptBigGlyph { width, height, is_color, texels: bordered, shaped_at: self.big_glyph_clock });
        let (width, height): (f32, f32) = (width as f32, height as f32);
        Slot { u0: 1.0 / width, v0: 1.0 / height, u1: (w + 1) as f32 / width, v1: (h + 1) as f32 / height, left, top, width: w, height: h, color: is_color, page: 0, big_glyph: Some(id) }
    }

    /// `rgba` (`w` x `h`) into the color atlas, which holds STRAIGHT
    /// color (the renderer's image quads premultiply on sample):
    /// `premultiplied` rows (swash's) are divided out on the way in.
    fn pack_color(&mut self, rgba: &[u8], w: u32, h: u32, left: i32, top: i32, premultiplied: bool) -> Slot {
        let (page, px, py): (usize, u32, u32) = self.pack(true, w + 1, h + 1);
        let side: u32 = self.emoji.side;
        let texels: &mut [u8] = &mut self.emoji.pages[page].texels;
        for row in 0..h {
            let src = &rgba[(row * w * 4) as usize..((row + 1) * w * 4) as usize];
            let dst = (((py + row) * side + px) * 4) as usize;
            let out = &mut texels[dst..dst + (w * 4) as usize];
            out.copy_from_slice(src);
            if premultiplied {
                straight_from_premultiplied(out);
            }
        }
        let s = side as f32;
        Slot { u0: px as f32 / s, v0: py as f32 / s, u1: (px + w) as f32 / s, v1: (py + h) as f32 / s, left, top, width: w, height: h, color: true, page: page as u32, big_glyph: None }
    }

    /// A place for a `w` x `h` slot in the letters' atlas, or in the
    /// emoji atlas when `is_color` (`Atlas::pack`).
    ///
    /// Needed in one place because a page emptied for the slot takes the
    /// slots of its glyphs and every kept shape that names it along: a
    /// text that showed them is shaped again, and its letters drawn again
    /// on another page, when it is next shown.
    ///
    /// Returns the slot's page and its top left corner.
    fn pack(&mut self, is_color: bool, w: u32, h: u32) -> (usize, u32, u32) {
        let atlas: &mut Atlas = if is_color { &mut self.emoji } else { &mut self.letters };
        let packed: Packed = atlas.pack(w, h, self.frame);
        match packed.change {
            PageChange::None => {}
            PageChange::Added => self.work.pages_added += 1,
            PageChange::Emptied => {
                self.work.pages_emptied += 1;
                let bit: u64 = page_bit(packed.page as u32);
                if is_color {
                    self.color_slots.retain(|_, slot| !slot.is_some_and(|s| s.is_on_page(packed.page)));
                    self.host_slots.retain(|_, slot| !slot.is_some_and(|s| s.is_on_page(packed.page)));
                    self.shape_cache.retain(|_, kept| kept.emoji_pages & bit == 0);
                } else {
                    self.slots.retain(|_, slot| !slot.is_some_and(|s| s.is_on_page(packed.page)));
                    self.shape_cache.retain(|_, kept| kept.letter_pages & bit == 0);
                }
            }
        }
        (packed.page, packed.x, packed.y)
    }

    /// Rasterizes every glyph of `chars` at `font_size`.
    ///
    /// Needed so that the first frame that shows them rasterizes nothing:
    /// a host calls it at start with the letters its first screen shows.
    pub fn warm(&mut self, chars: &str, font_size: f32, font: Option<&str>) {
        let _ = self.shape(chars, TextPx::one_line(font_size), None, font, 0.0, 0.0, 0.0, 0);
    }
}

/// A face's vertical metrics as its file carries them, or
/// `FaceMetrics::FALLBACK` when the file has none.
///
/// Needed because the line height comes from them, not from a constant
/// fitted to one size.
fn metrics_of(fs: &mut FontSystem, id: fontdb::ID, weight: Weight) -> FaceMetrics {
    fs.get_font(id, weight)
        .map(|font| {
            let m = font.as_swash().metrics(&[]);
            FaceMetrics { ascent: m.ascent, descent: m.descent, units_per_em: m.units_per_em as f32 }
        })
        .filter(|m| m.units_per_em > 0.0 && m.ascent > 0.0)
        .unwrap_or(FaceMetrics::FALLBACK)
}

/// Faux bold: the coverage mask grown by `grow` pixels on every side (a
/// square max filter — FreeType's EmboldenXY on the raster instead of the
/// outline). The result is `(w + 2·grow) × (h + 2·grow)`.
fn embolden(mask: &[u8], w: u32, h: u32, grow: u32) -> Vec<u8> {
    let (w2, h2) = (w + 2 * grow, h + 2 * grow);
    let mut out = vec![0u8; (w2 * h2) as usize];
    // Output (x, y) is source (x − grow, y − grow); its neighbours within
    // `grow` are the source pixels in [x − 2·grow, x], clipped to the mask.
    for y in 0..h2 {
        let (sy0, sy1) = (y.saturating_sub(2 * grow), y.min(h - 1));
        for x in 0..w2 {
            let (sx0, sx1) = (x.saturating_sub(2 * grow), x.min(w - 1));
            let mut best: u8 = 0;
            for sy in sy0..=sy1 {
                for sx in sx0..=sx1 {
                    best = best.max(mask[(sy * w + sx) as usize]);
                }
            }
            out[(y * w2 + x) as usize] = best;
        }
    }
    out
}

/// `gogo-ui`'s `TextMeasure` answered by a `TextEngine`: epx in, epx out,
/// device pixels inside.
///
/// Needed because the layout works in epx and knows no font: it asks a
/// `TextMeasure` how big a label is, and this one converts to the device
/// pixels the engine shapes in, and back.
pub struct EpxMeasure<'a> {
    pub engine: &'a RefCell<TextEngine>,
    pub epx: f32,
    /// The page's default font key (the `theme { font = … }` line).
    pub font: Option<String>,
    /// (width, height) px by `ImageKey` for `aspect=natural`; `None` = no
    /// readable header (a host picture, or a bad file).
    pub images: &'a [Option<(u32, u32)>],
}

impl TextMeasure for EpxMeasure<'_> {
    fn measure(&self, text: &str, font_size: f32, max_width: Option<f32>, font: Option<&str>) -> (f32, f32) {
        let mut e = self.engine.borrow_mut();
        // `TextPx::from_epx` is how a renderer sizes the label when it
        // draws it, so the label is measured at the size it is drawn at.
        let (w, h) = e.measure(text, TextPx::from_epx(font_size, self.epx), max_width.map(|w| w * self.epx), font.or(self.font.as_deref()));
        (w / self.epx, h / self.epx)
    }

    fn narrowest_wrap(&self, text: &str, font_size: f32, font: Option<&str>) -> f32 {
        let mut e = self.engine.borrow_mut();
        e.narrowest_wrap(text, TextPx::from_epx(font_size, self.epx), font.or(self.font.as_deref())) / self.epx
    }

    fn image_size(&self, key: ImageKey) -> Option<(f32, f32)> {
        self.images.get(key.index()).copied().flatten().map(|(w, h)| (w as f32, h as f32))
    }

    /// Everything that changes an answer without the page changing: the
    /// epx, the default font, and the image sizes (a picture code
    /// uploads later lands a size a `natural` aspect lays out by). The
    /// fonts themselves are fixed at `TextEngine::new`. A hash, so the
    /// retained layout compares one number per frame.
    fn identity(&self) -> u64 {
        let mut h = std::collections::hash_map::DefaultHasher::new();
        self.epx.to_bits().hash(&mut h);
        self.font.hash(&mut h);
        self.images.hash(&mut h);
        h.finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `TextPx::from_epx` cuts the font size down to a whole pixel and
    /// rounds the 1 epx line gap to a whole pixel; neither goes under 1.
    #[test]
    fn a_text_size_is_whole_pixels() {
        let rows: [(f32, f32, f32, f32); 8] = [
            // font size in epx, device pixels per epx -> font px, gap px
            (16.0, 1.0, 16.0, 1.0),
            (16.0, 1.25, 20.0, 1.0),
            (16.0, 1.5, 24.0, 2.0),
            (16.0, 2.0, 32.0, 2.0),
            (16.0, 2.625, 42.0, 3.0),
            (16.0, 3.0, 48.0, 3.0),
            (15.5, 1.0, 15.0, 1.0),
            (0.2, 0.25, 1.0, 1.0),
        ];
        for (font_size_epx, epx, font_px, gap_px) in rows {
            assert_eq!(TextPx::from_epx(font_size_epx, epx), TextPx { font_size: font_px, line_gap: gap_px }, "{font_size_epx} epx at {epx} device pixels per epx");
        }
    }

    /// A block of lines is that many font heights with a gap between
    /// each two: none above the first line, none below the last.
    #[test]
    fn a_block_has_a_gap_between_lines_only() {
        let m: FaceMetrics = FaceMetrics::FALLBACK;
        // 24 px of Roboto's metrics: 23 up, 6 down.
        assert_eq!(m.font_height(24.0), 29.0);
        let one: TextPx = TextPx { font_size: 24.0, line_gap: 1.0 };
        let three: TextPx = TextPx { font_size: 24.0, line_gap: 3.0 };
        assert_eq!((m.block_height(one, 1.0), m.block_height(three, 1.0)), (29.0, 29.0), "one line: no gap");
        assert_eq!((m.block_height(one, 3.0), m.block_height(three, 3.0)), (89.0, 93.0), "three lines: two gaps");
        assert_eq!((m.line_height(one), m.line_height(three)), (30.0, 32.0), "the pitch from one line to the next");
        assert_eq!(m.block_height(one, 0.0), 29.0, "an empty label is one line tall");
    }
}
