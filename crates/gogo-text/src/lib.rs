//! `gogo-text` shapes text and rasterizes glyphs for gogo, in pure Rust
//! (`cosmic-text` + `swash`): no GPU, no window.
//!
//! Needed as its own crate so that `gogo-ui` stays free of a text engine
//! (it measures through `TextMeasure`) and a renderer only uploads what
//! this crate rasterizes.
//!
//! Glyphs go into two atlases, shelf-packed: an 8-bit coverage atlas for
//! letters, tinted by the text color, and an RGBA color atlas for emoji,
//! drawn in their own colors. Each doubles when it fills, and every glyph
//! of the grown one is rasterized again on demand. All sizes here are
//! DEVICE pixels: the caller converts epx (`TextPx::from_epx` for the
//! size of a label's text; `EpxMeasure` does all of it for the layout).
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
use std::ops::Range;
use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};
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
}

impl Slot {
    /// Rewrites the slot's uv for its atlas grown to twice the side
    /// (`uv_in_doubled_atlas`).
    fn move_to_doubled_atlas(&mut self) {
        let mut uv: [f32; 4] = [self.u0, self.v0, self.u1, self.v1];
        uv_in_doubled_atlas(&mut uv);
        [self.u0, self.v0, self.u1, self.v1] = uv;
    }
}

/// `uv` of a glyph in an atlas, as it is in that atlas grown to twice
/// the side with the glyph left where it was (`TextEngine::grow`).
///
/// Exact: a half of an f32 is the number a fresh division by the
/// doubled side gives.
fn uv_in_doubled_atlas(uv: &mut [f32; 4]) {
    for coordinate in uv {
        *coordinate *= 0.5;
    }
}

/// `atlas`, `side` texels a side of `texel_bytes` each, as the top left
/// quarter of a new atlas twice the side; the rest of it empty.
///
/// Needed because an atlas that fills up grows, and the glyphs drawn
/// into it stay where they are: none is drawn a second time.
fn doubled_atlas(atlas: &[u8], side: u32, texel_bytes: u32) -> Vec<u8> {
    let row_bytes: usize = (side * texel_bytes) as usize;
    let mut doubled: Vec<u8> = vec![0; row_bytes * 2 * side as usize * 2];
    for row in 0..side as usize {
        doubled[row * row_bytes * 2..row * row_bytes * 2 + row_bytes].copy_from_slice(&atlas[row * row_bytes..(row + 1) * row_bytes]);
    }
    doubled
}

/// One glyph to draw: a rect in device pixels and its atlas uv.
#[derive(Clone, Copy, Debug)]
pub struct GlyphQuad {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
    pub uv: [f32; 4],
    /// The uv is into the COLOR atlas (`color_atlas`): draw the texels as
    /// they are, the text color's alpha only. False = the coverage atlas,
    /// tinted by the text color.
    pub color: bool,
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

/// An atlas version no atlas of this process has had, never 0.
///
/// Needed for the whole process, not per engine, so a renderer that drew
/// with one engine and is handed another never takes the second engine's
/// atlas for the one it uploaded (`TextEngine::atlas_part_to_send`).
fn next_atlas_version() -> u64 {
    static NEXT_ATLAS_VERSION: AtomicU64 = AtomicU64::new(1);
    NEXT_ATLAS_VERSION.fetch_add(1, Ordering::Relaxed)
}

/// What a renderer has sent to its GPU of one atlas: it keeps one for
/// the letters' atlas and one for the emoji atlas, and hands each to
/// `TextEngine::atlas_part_to_send` or `color_atlas_part_to_send`.
///
/// Needed so a renderer sends only the part it has not sent. A new
/// letter changes a small rectangle; the whole atlas is 4 MB at 2048 a
/// side, and sending it took 11 to 17 ms on an old phone, for one
/// letter.
///
/// `AtlasSent::default()` says nothing was sent: the next send is the
/// whole atlas.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct AtlasSent {
    /// The atlas's version at the send.
    version: u64,
    /// Which atlas was sent (`TextEngine::atlas_id`, `color_atlas_id`).
    atlas_id: u64,
    /// The first row of the shelf that was being filled at the send. A
    /// later glyph may have landed in that shelf; no row above it has
    /// changed.
    open_row: u32,
    /// How far across that shelf was filled at the send: a later glyph
    /// in it lies to the right, and nothing to the left has changed.
    open_x: u32,
}

/// A rectangle of an atlas, in texels: the part a renderer has not sent
/// to its GPU (`TextEngine::atlas_part_to_send`).
///
/// Needed as a rectangle, not as a run of rows: a new letter beside
/// letters already sent changes a few columns of its shelf, and a shelf
/// that holds one big letter is hundreds of rows tall.
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

/// The color atlas's first side; it doubles when it fills (`grow_color`).
const COLOR_ATLAS_START: u32 = 512;

/// A shelf packer over one square atlas: slots go left to right along a
/// row, a new row starts when one does not fit.
///
/// Needed because both atlases (coverage and color) pack the same way and
/// grow the same way — one packer, two sizes.
///
/// It packs one region of the atlas at a time: from `left` to the
/// atlas's right edge, down to `bottom`. A new atlas is one region. A
/// grown one (`grown`) holds the atlas that was in its top left
/// quarter, so it has two: the top right quarter, then the lower half.
/// A slot's row never lies above the row of a slot packed before it,
/// which `part_to_send` relies on.
#[derive(Clone, Copy, Debug)]
struct Shelf {
    size: u32,
    /// The left edge of the region being packed.
    left: u32,
    /// The row the region being packed ends above.
    bottom: u32,
    x: u32,
    y: u32,
    h: u32,
    /// Whether the lower half of the atlas is still to be packed, after
    /// the region being packed.
    is_lower_half_free: bool,
    /// The side of the atlas this one grew from, which it holds in its
    /// top left quarter; 0 for an atlas that has not grown.
    quarter: u32,
}

impl Shelf {
    fn new(size: u32) -> Shelf {
        Shelf { size, left: 0, bottom: size, x: 0, y: 0, h: 0, is_lower_half_free: false, quarter: 0 }
    }

    /// The packer of this shelf's atlas grown to twice the side, with
    /// what was packed left in the top left quarter.
    ///
    /// Needed so the glyphs drawn stay where they are when the atlas
    /// grows. What was left free of the top left quarter is not packed
    /// again: a strip under the last shelf, less than one slot tall.
    fn grown(self) -> Shelf {
        Shelf { size: self.size * 2, left: self.size, bottom: self.size, x: self.size, y: 0, h: 0, is_lower_half_free: true, quarter: self.size }
    }

    /// The top-left of a `w` × `h` slot, or `None` when the atlas is full.
    fn pack(&mut self, w: u32, h: u32) -> Option<(u32, u32)> {
        loop {
            if self.x + w > self.size {
                self.y += self.h;
                self.x = self.left;
                self.h = 0;
            }
            if self.x + w <= self.size && self.y + h <= self.bottom {
                break;
            }
            if !self.is_lower_half_free {
                return None;
            }
            self.is_lower_half_free = false;
            (self.left, self.x, self.y, self.h, self.bottom) = (0, 0, self.bottom, 0, self.size);
        }
        let pos = (self.x, self.y);
        self.x += w;
        self.h = self.h.max(h);
        Some(pos)
    }

    /// The part a renderer at `sent` has not got of the atlas `atlas_id`
    /// this shelf packs, now at `version`.
    ///
    /// Needed in one place because both atlases answer it the same way.
    /// Slots are packed left to right, shelf below shelf. So nothing
    /// above the shelf that was being filled at the last send has
    /// changed, and in that shelf nothing to the left of where it was
    /// filled to.
    ///
    /// A send that starts a new shelf sends the region's whole rows,
    /// the earlier slots of the shelf above included: once per shelf,
    /// not per letter.
    ///
    /// Returns a part, and what the renderer has sent once it sent it.
    /// The caller asks again with that until the part is empty: slots
    /// packed in two regions since `sent` are two parts.
    /// - Nothing when the version is the one sent.
    /// - The whole atlas when it is another than the one sent.
    /// - The new slots' columns of the shelf when that shelf is still
    ///   being filled.
    /// - The rest of the top right quarter of a grown atlas, when the
    ///   packer has gone on to the lower half since: the lower half's
    ///   rows are the next part. One part for both would cover the left
    ///   half of the quarter's rows, the atlas that was.
    /// - Else the rows from that shelf down to the last one used, across
    ///   the region being packed.
    fn part_to_send(&self, atlas_id: u64, version: u64, sent: AtlasSent) -> (AtlasPart, AtlasSent) {
        let sent_all = AtlasSent { version, atlas_id, open_row: self.y, open_x: self.x };
        if sent.version == version {
            (AtlasPart { columns: 0..0, rows: 0..0 }, sent_all)
        } else if sent.atlas_id != atlas_id {
            (AtlasPart { columns: 0..self.size, rows: 0..self.size }, sent_all)
        } else if sent.open_row == self.y {
            (AtlasPart { columns: sent.open_x..self.x.min(self.size), rows: self.y..self.y + self.h }, sent_all)
        } else if self.left == 0 && sent.open_row < self.quarter {
            // The version stays the one sent: the lower half is still to
            // send, from its first row and its left edge.
            (AtlasPart { columns: self.quarter..self.size, rows: sent.open_row..self.quarter }, AtlasSent { version: sent.version, atlas_id, open_row: self.quarter, open_x: 0 })
        } else {
            (AtlasPart { columns: self.left..self.size, rows: sent.open_row..self.y + self.h }, sent_all)
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
/// origin. Their uvs are into the atlases as they are now: a grow
/// rewrites them.
#[derive(Clone, Debug)]
struct KeptShape {
    quads: Vec<GlyphQuad>,
    /// See `TextEngine::is_settled`.
    is_settled: bool,
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
    /// Texts shaped to draw them (`shape`, `shape_into`): one a text not
    /// shaped before, and one more for the text whose letter made a
    /// letters' texture grow.
    pub shapes_to_draw: u32,
    /// Time of those shapes, without the letters they drew.
    pub seconds_shaping_to_draw: f32,
    /// Letters drawn into a letters' texture, each at first sight.
    pub letters_drawn: u32,
    pub seconds_drawing_letters: f32,
    /// Times a letters' texture filled and doubled. The letters drawn
    /// and the texts shaped are kept; a renderer sends the grown
    /// texture whole.
    pub texture_grows: u32,
}

pub struct TextEngine {
    fs: FontSystem,
    cache: SwashCache,
    /// path key (as written in the .gogo) → face.
    faces: HashMap<String, Face>,
    default_face: Option<Face>,
    atlas: Vec<u8>,
    atlas_size: u32,
    /// A number no other atlas of this process has; a new one when the
    /// atlas grows, since every row of the grown one differs.
    atlas_id: u64,
    shelf: Shelf,
    /// (glyph, faux bold) → its coverage-atlas slot; a bold glyph is its
    /// own raster.
    slots: HashMap<(CacheKey, bool), Option<Slot>>,
    /// The emoji atlas: straight RGBA, `color_shelf.size` square.
    color_atlas: Vec<u8>,
    /// `atlas_id`, for the emoji atlas.
    color_atlas_id: u64,
    color_shelf: Shelf,
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
    /// `atlas_version`, for the emoji atlas. 0 until the first emoji
    /// packs, where a renderer's `AtlasSent` starts too: a start without
    /// an emoji sends no emoji atlas.
    color_atlas_version: u64,
    /// The OS font scan ran (once per engine).
    system_loaded: bool,
    /// Family names the scan found, sorted, each once.
    system_families: Vec<String>,
    /// A number that is new each time the letters' atlas changes: a
    /// glyph packed into it, or the atlas grown. No two engines of a
    /// process have the same one.
    ///
    /// Needed so each renderer knows by itself whether the atlas it
    /// uploaded is still this one (`atlas_part_to_send` compares it with
    /// the version that renderer sent). With one flag that the first
    /// reader cleared, a second renderer over the same engine drew every
    /// plate and picture and no letters.
    atlas_version: u64,
    /// Counts the grows of both atlases: every uv handed out before a
    /// grow is stale, since the glyph it names is in the same texels of
    /// an atlas twice the side. The engine's own slots and kept shapes
    /// are put right in the grow; a caller that holds quads (a renderer
    /// in the middle of a frame) shapes them again when this changed.
    pub generation: u32,
    /// Keyed by a hash of (text, size, width, font) — no String is built
    /// to look a measure up (a page lays out every animated frame). The
    /// engine serves every page, and their `StrId`s overlap, so the text
    /// itself is hashed, not its id. A 64-bit collision would return a
    /// wrong size for one string in ~10^19; accepted.
    measure_cache: HashMap<u64, KeptMeasure>,
    /// Keyed by a hash of (text, size, width, font, box_w, halign).
    shape_cache: HashMap<u64, KeptShape>,
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
        let atlas_size = 1024;
        TextEngine {
            fs,
            cache: SwashCache::new(),
            faces,
            default_face,
            atlas: vec![0; (atlas_size * atlas_size) as usize],
            atlas_size,
            atlas_id: next_atlas_version(),
            shelf: Shelf::new(atlas_size),
            slots: HashMap::new(),
            color_atlas: vec![0; (COLOR_ATLAS_START * COLOR_ATLAS_START * 4) as usize],
            color_atlas_id: next_atlas_version(),
            color_shelf: Shelf::new(COLOR_ATLAS_START),
            color_slots: HashMap::new(),
            host_slots: HashMap::new(),
            color_faces: HashMap::new(),
            host_color_glyph_painter: None,
            // 0, where a renderer starts too: nothing points into it
            // until the first emoji packs, so a boot without one uploads
            // nothing.
            color_atlas_version: 0,
            system_loaded: false,
            system_families: Vec::new(),
            atlas_version: next_atlas_version(),
            generation: 0,
            measure_cache: HashMap::new(),
            shape_cache: HashMap::new(),
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

    pub fn atlas(&self) -> (&[u8], u32) {
        (&self.atlas, self.atlas_size)
    }

    /// The emoji atlas: straight RGBA rows, `side` square.
    pub fn color_atlas(&self) -> (&[u8], u32) {
        (&self.color_atlas, self.color_shelf.size)
    }

    /// Installs (or removes) the host's emoji painter. A load-time call:
    /// drops the shape cache and every host-painted slot, so a cluster
    /// the old painter drew is painted again by the new one.
    pub fn set_host_color_glyph_painter(&mut self, host: Option<HostColorGlyphPainter>) {
        self.host_color_glyph_painter = host;
        self.host_slots.clear();
        self.shape_cache.clear();
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

    /// A part of the letters' atlas that a renderer at `sent` has not
    /// sent to its GPU. A renderer calls it each frame, after the
    /// frame's texts are shaped, and writes that rectangle of `atlas`
    /// into its texture; it asks again with the `AtlasSent` it got back
    /// until the part is empty. All the frame's new letters are one
    /// part, or two when they went on from the top right quarter of a
    /// grown atlas to its lower half.
    ///
    /// Needed because a frame that draws one new letter otherwise sends
    /// the whole atlas: the largest part of such a frame's text time.
    /// Each renderer keeps its own `AtlasSent`, so several renderers
    /// over one engine each get the part they lack.
    ///
    /// Returns the part, and what the renderer keeps as sent once it
    /// wrote it. An empty part when nothing was packed since `sent`. The
    /// whole atlas when it is not the one `sent` names: nothing sent
    /// yet, another engine's, or this one after it grew. The texel at
    /// column `x` of row `y` is the byte `y * size + x` of `atlas`.
    pub fn atlas_part_to_send(&self, sent: AtlasSent) -> (AtlasPart, AtlasSent) {
        self.shelf.part_to_send(self.atlas_id, self.atlas_version, sent)
    }

    /// `atlas_part_to_send`, for the emoji atlas (4 bytes a texel).
    ///
    /// Returns an empty part until the first emoji packs: a start
    /// without an emoji sends no emoji atlas.
    pub fn color_atlas_part_to_send(&self, sent: AtlasSent) -> (AtlasPart, AtlasSent) {
        self.color_shelf.part_to_send(self.color_atlas_id, self.color_atlas_version, sent)
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
            return;
        }
        let kept: KeptShape = self.shape_uncached(text, size, max_width, font, box_w, halign);
        if self.shape_cache.len() >= CACHE_CAP {
            self.shape_cache.clear();
        }
        out.extend(kept.quads.iter().map(|q| GlyphQuad { x: q.x + dx, y: q.y + dy, ..*q }));
        self.shape_cache.insert(key, kept);
    }

    /// Shapes `text` into quads with the pen at the origin, every uv into
    /// the atlas as it is when the run ends.
    ///
    /// Needed because a glyph packed past the atlas's edge grows the
    /// atlas (`pack` → `grow`), and every uv handed out before, this
    /// run's earlier glyphs included, is into the atlas that was. A run
    /// kept that way shows without its first letters ("ve changes?" for
    /// "Save changes?") and stays that way, because it goes into the
    /// shape cache with the stale uvs.
    ///
    /// The run is shaped again against the grown atlas, in the same
    /// buffer: its glyphs are all in the atlas by then, so none is
    /// drawn again. A run that grows nothing (the usual one) is shaped
    /// once.
    fn shape_uncached(&mut self, text: &str, size: TextPx, max_width: Option<f32>, font: Option<&str>, box_w: f32, halign: u8) -> KeptShape {
        let mut out: Vec<GlyphQuad> = Vec::new();
        loop {
            let generation: u32 = self.generation;
            out.clear();
            let is_settled: bool = self.shape_pass(text, size, max_width, font, box_w, halign, &mut out);
            if self.generation == generation {
                return KeptShape { quads: out, is_settled };
            }
        }
    }

    /// One shaping pass into `out`; its uvs are good only if neither atlas
    /// grew during it (`shape_uncached` checks and runs it again).
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
                });
            }
        }
        is_settled
    }

    /// The slot of one glyph, rasterizing it on first sight. An emoji (a
    /// glyph of a color face) goes to the host painter when there is one,
    /// else to swash; a letter to swash. A raster that grew its atlas is
    /// packed again into the grown one (`None` from the first try).
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
        let generation = self.generation;
        let started: Instant = Instant::now();
        let slot = self.rasterize(key, bold);
        let slot = if slot.is_none() && self.generation != generation {
            // An atlas grew under this glyph: pack it again into the new one.
            self.rasterize(key, bold)
        } else {
            slot
        };
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
    /// = the host drew nothing (the caller falls back to swash). A pack
    /// that grows the atlas packs again into the grown one.
    fn host_slot(&mut self, cluster: &str, font_size_bits: u32) -> Option<Option<Slot>> {
        let painted: HostColorGlyph = (self.host_color_glyph_painter.as_mut()?)(cluster, f32::from_bits(font_size_bits))?;
        if painted.width == 0 || painted.height == 0 || painted.rgba.len() != (painted.width * painted.height * 4) as usize {
            return Some(Some(Slot { u0: 0.0, v0: 0.0, u1: 0.0, v1: 0.0, left: 0, top: 0, width: 0, height: 0, color: true }));
        }
        let first = self.pack_color(&painted.rgba, painted.width, painted.height, painted.left, painted.top, false);
        Some(first.or_else(|| self.pack_color(&painted.rgba, painted.width, painted.height, painted.left, painted.top, false)))
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
                    return Some(Slot { u0: 0.0, v0: 0.0, u1: 0.0, v1: 0.0, left: 0, top: 0, width: 0, height: 0, color: true });
                }
                return self.pack_color(&img.data, w, h, left, top, true);
            }
            if img.content != SwashContent::Mask {
                return None;
            }
            if w == 0 || h == 0 {
                return Some(Slot { u0: 0.0, v0: 0.0, u1: 0.0, v1: 0.0, left: 0, top: 0, width: 0, height: 0, color: false });
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
            let (px, py) = self.pack(w + 1, h + 1)?;
            for row in 0..h {
                let src = &data[(row * w) as usize..((row + 1) * w) as usize];
                let dst = ((py + row) * self.atlas_size + px) as usize;
                self.atlas[dst..dst + w as usize].copy_from_slice(src);
            }
            self.atlas_version = next_atlas_version();
            let s = self.atlas_size as f32;
            Some(Slot { u0: px as f32 / s, v0: py as f32 / s, u1: (px + w) as f32 / s, v1: (py + h) as f32 / s, left, top, width: w, height: h, color: false })
        });
        slot
    }

    /// `rgba` (`w` x `h`) into the color atlas, which holds STRAIGHT
    /// color (the renderer's image quads premultiply on sample):
    /// `premultiplied` rows (swash's) are divided out on the way in. `None`
    /// after a grow (the caller packs again).
    fn pack_color(&mut self, rgba: &[u8], w: u32, h: u32, left: i32, top: i32, premultiplied: bool) -> Option<Slot> {
        let Some((px, py)) = self.color_shelf.pack(w + 1, h + 1) else {
            self.grow_color();
            return None;
        };
        let side = self.color_shelf.size;
        for row in 0..h {
            let src = &rgba[(row * w * 4) as usize..((row + 1) * w * 4) as usize];
            let dst = (((py + row) * side + px) * 4) as usize;
            let out = &mut self.color_atlas[dst..dst + (w * 4) as usize];
            out.copy_from_slice(src);
            if premultiplied {
                for texel in out.chunks_exact_mut(4) {
                    let a = texel[3] as u32;
                    if a != 0 && a != 255 {
                        for c in &mut texel[..3] {
                            *c = ((*c as u32 * 255 + a / 2) / a).min(255) as u8;
                        }
                    }
                }
            }
        }
        self.color_atlas_version = next_atlas_version();
        let s = side as f32;
        Some(Slot { u0: px as f32 / s, v0: py as f32 / s, u1: (px + w) as f32 / s, v1: (py + h) as f32 / s, left, top, width: w, height: h, color: true })
    }

    /// Shelf packing into the coverage atlas; `None` after a grow (the
    /// caller retries via `slot`).
    fn pack(&mut self, w: u32, h: u32) -> Option<(u32, u32)> {
        let pos = self.shelf.pack(w, h);
        if pos.is_none() {
            self.grow();
        }
        pos
    }

    /// The letters' atlas doubles its side and keeps every glyph drawn:
    /// the atlas that was is the top left quarter of the new one.
    ///
    /// Needed because the atlas fills up, a few times in a program's
    /// life. Emptied at a grow, it had every glyph on screen drawn
    /// again and every text shaped again in that frame.
    ///
    /// A glyph stays in its texels, so its uv is a half of what it was:
    /// the slots and the kept shapes are rewritten here. A uv handed out
    /// before is stale (`generation`).
    fn grow(&mut self) {
        self.work.texture_grows += 1;
        self.atlas = doubled_atlas(&self.atlas, self.atlas_size, 1);
        self.atlas_size *= 2;
        self.shelf = self.shelf.grown();
        for slot in self.slots.values_mut().flatten() {
            slot.move_to_doubled_atlas();
        }
        self.kept_shapes_into_doubled_atlas(false);
        self.generation += 1;
        self.atlas_version = next_atlas_version();
        self.atlas_id = next_atlas_version();
    }

    /// `grow`, for the color atlas: every emoji drawn stays, the ones
    /// the host painted too.
    fn grow_color(&mut self) {
        self.work.texture_grows += 1;
        self.color_atlas = doubled_atlas(&self.color_atlas, self.color_shelf.size, 4);
        self.color_shelf = self.color_shelf.grown();
        for slot in self.color_slots.values_mut().chain(self.host_slots.values_mut()).flatten() {
            slot.move_to_doubled_atlas();
        }
        self.kept_shapes_into_doubled_atlas(true);
        self.generation += 1;
        self.color_atlas_version = next_atlas_version();
        self.color_atlas_id = next_atlas_version();
    }

    /// Rewrites the uv of every kept quad of one atlas (`is_color`: the
    /// color atlas) for that atlas grown to twice its side.
    ///
    /// Needed so a text shaped before a grow is not shaped again after
    /// it: its glyphs are where they were.
    fn kept_shapes_into_doubled_atlas(&mut self, is_color: bool) {
        for quad in self.shape_cache.values_mut().flat_map(|kept| kept.quads.iter_mut()).filter(|quad| quad.color == is_color) {
            uv_in_doubled_atlas(&mut quad.uv);
        }
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

    /// A grown shelf packs the three free quarters of the grown atlas
    /// and nothing over the atlas that was, and its slots' rows never
    /// go back up. Red when a glyph packed after a grow lands on a
    /// glyph drawn before it: the user sees one letter inside another.
    #[test]
    fn a_grown_shelf_packs_around_the_atlas_that_was() {
        const SIZES: [(u32, u32); 5] = [(10, 12), (7, 9), (20, 15), (5, 5), (13, 11)];
        // Packs slots of those sizes in turn until one does not fit.
        fn pack_until_full(shelf: &mut Shelf, slots: &mut Vec<(u32, u32, u32, u32)>) {
            for (w, h) in SIZES.iter().cycle() {
                match shelf.pack(*w, *h) {
                    Some((x, y)) => slots.push((x, y, *w, *h)),
                    None => return,
                }
            }
        }
        let mut slots: Vec<(u32, u32, u32, u32)> = Vec::new();
        let mut shelf = Shelf::new(64);
        pack_until_full(&mut shelf, &mut slots);
        let old_count: usize = slots.len();
        assert!(slots.iter().all(|&(x, y, w, h)| x + w <= 64 && y + h <= 64));

        let mut shelf: Shelf = shelf.grown();
        pack_until_full(&mut shelf, &mut slots);
        let new: &[(u32, u32, u32, u32)] = &slots[old_count..];
        assert!(new.len() * 2 >= old_count * 5, "three free quarters hold about three times the slots of one: {} after {old_count}", new.len());
        for &(x, y, w, h) in new {
            assert!(x + w <= 128 && y + h <= 128, "inside the grown atlas: {x},{y} {w}x{h}");
            assert!(x >= 64 || y >= 64, "not over the atlas that was: {x},{y} {w}x{h}");
        }
        for (i, a) in slots.iter().enumerate() {
            for b in &slots[i + 1..] {
                assert!(a.0 + a.2 <= b.0 || b.0 + b.2 <= a.0 || a.1 + a.3 <= b.1 || b.1 + b.3 <= a.1, "two slots overlap: {a:?} {b:?}");
            }
        }
        assert!(new.windows(2).all(|pair| pair[0].1 <= pair[1].1), "a slot's row is never above the row of the slot before it");
    }

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
