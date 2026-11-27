//! The text engine on real fonts: it measures, wraps, shapes and
//! rasterizes Roboto, takes more faces by key, scans the OS fonts, and
//! puts emoji in the color atlas.
//!
//! Needed as a file of its own, outside `src`, because every test here
//! reads the Roboto files in `assets/Fonts` at the top of the repository,
//! and a crates.io package does not carry them. `Cargo.toml` leaves this
//! file out of the package (`exclude`), so `cargo test` on a downloaded
//! package runs only what the package can feed.

use gogo_text::{ATLAS_GLYPH_MAX_PX, ATLAS_PAGE_SIDE, ATLAS_PAGES_KEPT, Atlas, AtlasPart, AtlasSent, BIG_GLYPH_KEPT_BYTES, BigGlyph, BigGlyphId, COLOR_ATLAS_PAGE_SIDE, EpxMeasure, FaceMetrics, GlyphQuad, HostColorGlyph, TextEngine, TextPx, TextWork, styled_key};
use gogo_ui::layout::TextMeasure;
use std::cell::RefCell;
use std::path::Path;

/// A run that fills a page of the letters' atlas goes on on a new page:
/// it is shaped once, no letter is drawn twice, and a text shown before
/// keeps its kept shape and its letters' places.
///
/// Red when a full atlas doubles: every text is shaped again and every
/// letter on screen drawn again, 20 to 40 ms in one frame on an old
/// phone.
#[test]
fn a_run_that_fills_a_page_goes_on_on_a_new_page() {
    let mut e = engine();
    let before: Vec<GlyphQuad> = e.shape("Save changes?", px(40.0), None, None, 0.0, 0.0, 0.0, 0);
    let (shaped, work, font_size) = run_that_fills_the_first_page(&mut e);
    assert_eq!((work.pages_added, work.pages_emptied, work.shapes_to_draw), (1, 0, 1), "one page more, the run shaped once: {work:?}");
    assert!(work.letters_drawn <= ALPHABET.len() as u32, "no letter drawn twice: {work:?}");
    assert!(shaped.iter().any(|quad| quad.page == 1), "the run goes on on the new page");
    e.take_work();
    let after: Vec<GlyphQuad> = e.shape("Save changes?", px(40.0), None, None, 0.0, 0.0, 0.0, 0);
    assert_eq!(e.take_work(), TextWork::default(), "the text shown before costs a lookup");
    let places = |quads: &[GlyphQuad]| -> Vec<([f32; 4], u32)> { quads.iter().map(|quad| (quad.uv, quad.page)).collect() };
    assert_eq!(places(&after), places(&before), "its letters where they were, on the first page");
    // Shaped again past the cache: the box width is part of the cache
    // key and moves no glyph of a start-aligned line.
    let again: Vec<GlyphQuad> = e.shape(ALPHABET, px(font_size), None, None, 0.0, 0.0, 100_001.0, 0);
    assert_eq!(places(&again), places(&shaped), "the run that started the page, shaped whole");
}

/// Past `ATLAS_PAGES_KEPT` pages, a glyph that needs room empties the
/// page used longest ago: the texts on it are shaped and drawn again
/// when next shown. A page a text of the frame being drawn uses is never
/// emptied: when every page is in use, the atlas gets one more.
///
/// Red when a page in use is emptied: the frame shows letters from the
/// glyphs packed over them. Red when no page is ever emptied: a session
/// that shows many fonts keeps every letter it drew.
#[test]
fn past_the_kept_pages_the_page_used_longest_ago_is_emptied() {
    /// Shapes the alphabet at `font_size`, makes the size a pixel
    /// smaller, and keeps the run's size for each page its letters are on
    /// (`size_on_page`).
    ///
    /// Returns the run's work.
    fn show_next_size(e: &mut TextEngine, font_size: &mut f32, size_on_page: &mut Vec<f32>) -> TextWork {
        assert!(*font_size > 30.0, "the precondition: the sizes fill the pages");
        e.take_work();
        for quad in e.shape(ALPHABET, px(*font_size), None, None, 0.0, 0.0, 100_000.0, 0) {
            size_on_page.resize(size_on_page.len().max(quad.page as usize + 1), 0.0);
            size_on_page[quad.page as usize] = *font_size;
        }
        *font_size -= 1.0;
        e.take_work()
    }
    let mut e = engine();
    e.start_frame();
    e.shape("Hello", px(24.0), None, None, 0.0, 0.0, 0.0, 0);
    // One frame per size of the alphabet, each showing that size only.
    let mut size_on_page: Vec<f32> = Vec::new();
    let mut font_size: f32 = 120.0;
    while e.atlas().page_count() < ATLAS_PAGES_KEPT {
        e.start_frame();
        show_next_size(&mut e, &mut font_size, &mut size_on_page);
    }
    loop {
        e.start_frame();
        let work: TextWork = show_next_size(&mut e, &mut font_size, &mut size_on_page);
        assert_eq!(work.pages_added, 0, "past the kept pages no page is added while one is unused: {work:?}");
        if work.pages_emptied > 0 {
            assert_eq!(work.pages_emptied, 1);
            break;
        }
    }
    assert_eq!(e.atlas().page_count(), ATLAS_PAGES_KEPT);
    assert!(size_on_page[0] == font_size + 1.0, "the new letters went on the page used longest ago: the first");
    e.start_frame();
    e.take_work();
    e.shape("Hello", px(24.0), None, None, 0.0, 0.0, 0.0, 0);
    let work: TextWork = e.take_work();
    assert!(work.shapes_to_draw == 1 && work.letters_drawn >= 4, "the text on the emptied page is shaped and drawn again: {work:?}");
    // A text on the third page has no letter on the emptied one.
    e.shape(ALPHABET, px(size_on_page[2]), None, None, 0.0, 0.0, 100_000.0, 0);
    assert_eq!(e.take_work(), TextWork::default(), "a text on another page keeps its kept shape");

    // A frame that shows a text on every page, then needs room: a page
    // more, none emptied. Once with the texts' kept shapes, once with
    // each text shaped anew (another box width), finding its letters
    // drawn: both ways tell the engine the pages are in use.
    for (box_w, way) in [(100_000.0, "kept shapes"), (100_002.0, "texts shaped anew")] {
        e.start_frame();
        for size in size_on_page.clone() {
            e.shape(ALPHABET, px(size), None, None, 0.0, 0.0, box_w, 0);
        }
        let pages: usize = e.atlas().page_count();
        loop {
            let work: TextWork = show_next_size(&mut e, &mut font_size, &mut size_on_page);
            assert_eq!(work.pages_emptied, 0, "{way}: no page this frame uses is emptied: {work:?}");
            if work.pages_added > 0 {
                break;
            }
        }
        assert_eq!(e.atlas().page_count(), pages + 1, "{way}");
    }
}

/// A right-to-left line in a wide box starts at the box's left edge
/// like a Latin one, and `halign` still moves it: centre and end land
/// where they would for Latin text, not twice as far. Red when
/// cosmic-text's default stands: the Arabic word sits against the
/// right edge of the wrap width. Ignored unless asked for: it needs an
/// OS font with Arabic letters, which a bare test machine may not
/// have (see `engine_with_emoji`).
#[test]
#[ignore = "needs an OS font with Arabic letters: run with --include-ignored on Windows"]
fn a_right_to_left_line_starts_at_the_left_edge_and_halign_still_moves_it() {
    let mut e = engine();
    e.load_system_fonts(); // the desktop's Arabic face (Arial / Segoe UI)
    assert!(e.has_glyph('ع'), "precondition: an Arabic face");
    let span = |q: &[GlyphQuad]| -> (f32, f32) { (q.iter().map(|g| g.x).fold(f32::MAX, f32::min), q.iter().map(|g| g.x + g.w).fold(f32::MIN, f32::max)) };
    let box_w: f32 = 600.0;
    let (left, right) = span(&e.shape("عربي", px(40.0), Some(box_w), None, 0.0, 0.0, box_w, 0));
    assert!(left < 20.0, "start = the left edge: {left}..{right}");
    let w: f32 = right - left;
    let (c_left, _) = span(&e.shape("عربي", px(40.0), Some(box_w), None, 0.0, 0.0, box_w, 1));
    assert!((c_left - (box_w - w) / 2.0).abs() < 20.0, "centred once: {c_left} for width {w}");
    let (_, e_right) = span(&e.shape("عربي", px(40.0), Some(box_w), None, 0.0, 0.0, box_w, 2));
    assert!((e_right - box_w).abs() < 20.0 && e_right <= box_w + 2.0, "end = the right edge: {e_right}");
    let (l_left, _) = span(&e.shape("English", px(40.0), Some(box_w), None, 0.0, 0.0, box_w, 0));
    assert!(l_left < 20.0, "Latin unchanged: {l_left}");
}

fn engine() -> TextEngine {
    let bytes = std::fs::read(concat!(env!("CARGO_MANIFEST_DIR"), "/../../assets/Fonts/Roboto Regular.ttf")).expect("font");
    TextEngine::new(&[("assets/Fonts/Roboto Regular.ttf", bytes)])
}

const ALPHABET: &str = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz";

/// Shapes the alphabet at 120 px, then a pixel smaller each time, until
/// one run fills the first page of the letters' atlas and starts a
/// second.
///
/// Needed by the tests of a full page. Letters of 120 px are the biggest
/// that still go into the atlas: a bigger one is a big glyph and fills
/// nothing.
///
/// Returns the quads of the run that started the second page, that
/// run's work, and its font size.
fn run_that_fills_the_first_page(e: &mut TextEngine) -> (Vec<GlyphQuad>, TextWork, f32) {
    for font_size in (40..=120).rev().map(|size| size as f32) {
        let pages: usize = e.atlas().page_count().max(1);
        e.take_work();
        let quads: Vec<GlyphQuad> = e.shape(ALPHABET, px(font_size), None, None, 0.0, 0.0, 100_000.0, 0);
        assert!(quads.iter().all(|quad| quad.big_glyph.is_none()), "the precondition: letters of {font_size} px go into the atlas");
        if e.atlas().page_count() > pages {
            return (quads, e.take_work(), font_size);
        }
    }
    panic!("the first page fills before the sizes run out");
}

/// Text of `font_size` device pixels on a screen with 1 device pixel per
/// epx: the line gap is 1 pixel.
fn px(font_size: f32) -> TextPx {
    TextPx::from_epx(font_size, 1.0)
}

/// Roboto plus Windows' Segoe UI Emoji as a fallback face (a color
/// face: COLRv0 layers swash paints itself).
///
/// Needed by the emoji tests, which are `#[ignore]` for that reason:
/// the file exists on Windows only, and a test that returned early
/// elsewhere would count as passed without having run. A Windows host
/// runs them with `cargo test -p gogo-text -- --include-ignored`.
fn engine_with_emoji() -> TextEngine {
    let segoe = Path::new("C:/Windows/Fonts/seguiemj.ttf");
    let mut e = engine();
    assert!(e.add_fallback_font_file(segoe), "this test needs {}", segoe.display());
    e
}

/// The color atlas texel at the top-left of `q`'s slot, RGBA.
fn color_texel_of(e: &TextEngine, q: &GlyphQuad) -> [u8; 4] {
    let (atlas, side): (&[u8], u32) = (e.color_atlas().page(q.page as usize), e.color_atlas().side());
    let (x, y) = ((q.uv[0] * side as f32).round() as usize, (q.uv[1] * side as f32).round() as usize);
    let i = (y * side as usize + x) * 4;
    [atlas[i], atlas[i + 1], atlas[i + 2], atlas[i + 3]]
}

/// An emoji of a color face lands in the COLOR atlas with its own
/// colors (`color: true`, texels that are not gray), a letter beside
/// it in the coverage atlas; both atlases have a part to send. Red when the
/// rasterizer keeps coverage masks only: every other glyph is dropped,
/// and the user sees a box for each emoji.
#[test]
#[ignore = "needs Windows' Segoe UI Emoji font: run with --include-ignored on Windows"]
fn an_emoji_lands_in_the_color_atlas_beside_the_letters() {
    let mut e: TextEngine = engine_with_emoji();
    assert_eq!((e.atlas().page_count(), e.color_atlas().page_count()), (0, 0), "nothing packed yet");
    let quads: Vec<GlyphQuad> = e.shape("a😀", px(40.0), None, None, 0.0, 0.0, 0.0, 0);
    assert_eq!(quads.len(), 2, "{quads:?}");
    assert!(!quads[0].color, "the letter: coverage atlas");
    assert!(quads[1].color, "the emoji: color atlas");
    assert!(quads[1].w > 20.0 && quads[1].h > 20.0, "an emoji at 40 px is a real box: {:?}", quads[1]);
    assert!(!e.atlas().part_to_send(0, AtlasSent::default()).0.is_empty() && !e.color_atlas().part_to_send(0, AtlasSent::default()).0.is_empty());
    // Somewhere in the emoji's slot a texel is opaque and colored.
    let (atlas, side): (&[u8], u32) = (e.color_atlas().page(quads[1].page as usize), e.color_atlas().side());
    let (x0, y0, x1, y1) = ((quads[1].uv[0] * side as f32) as usize, (quads[1].uv[1] * side as f32) as usize, (quads[1].uv[2] * side as f32) as usize, (quads[1].uv[3] * side as f32) as usize);
    let colored = (y0..y1).flat_map(|y| (x0..x1).map(move |x| (x, y))).any(|(x, y)| {
        let i = (y * side as usize + x) * 4;
        let (r, g, b, a) = (atlas[i], atlas[i + 1], atlas[i + 2], atlas[i + 3]);
        // swash's layered coverage tops out a hair under 255.
        a >= 250 && (r != g || g != b)
    });
    assert!(colored, "the slot holds the emoji's own colors");
}

/// With a host painter installed, an emoji is painted by the host
/// (its raster lands in the color atlas, placed by its left/top), a
/// letter still by swash, and a host that declines (`None`) leaves
/// the emoji to swash. The painter runs once per (cluster, size).
#[test]
#[ignore = "needs Windows' Segoe UI Emoji font: run with --include-ignored on Windows"]
fn the_host_paints_an_emoji_when_it_has_one() {
    let mut e: TextEngine = engine_with_emoji();
    let asked: std::rc::Rc<RefCell<Vec<(String, f32)>>> = std::rc::Rc::new(RefCell::new(Vec::new()));
    let log = asked.clone();
    e.set_host_color_glyph_painter(Some(Box::new(move |cluster: &str, px: f32| {
        log.borrow_mut().push((cluster.to_string(), px));
        // A 6×6 solid green square sitting 2 px right of the pen, 5 above the baseline.
        (cluster == "😀").then(|| HostColorGlyph { width: 6, height: 6, left: 2, top: 5, rgba: [0, 255, 0, 255].repeat(36) })
    })));
    let quads: Vec<GlyphQuad> = e.shape("a😀", px(40.0), None, None, 0.0, 0.0, 0.0, 0);
    assert_eq!(quads.len(), 2, "{quads:?}");
    assert!(!quads[0].color, "the letter: swash, coverage atlas");
    assert_eq!(asked.borrow().as_slice(), &[("😀".to_string(), 40.0)], "asked once, the cluster text and the px size");
    assert!(quads[1].color);
    assert_eq!((quads[1].w, quads[1].h), (6.0, 6.0), "the host's raster size");
    assert_eq!(color_texel_of(&e, &quads[1]), [0, 255, 0, 255], "the host's pixels");
    // The second shape of the same run is served from the cache.
    let _ = e.shape("a😀", px(40.0), None, None, 0.0, 0.0, 0.0, 0);
    assert_eq!(asked.borrow().len(), 1, "no second ask for a cached run");
    // The quad sits `left` px right of the pen and `top` px above the
    // baseline: against a painter that reports (0, 0), it is 2 right
    // and 5 up. A new painter repaints (the old slots are dropped).
    e.set_host_color_glyph_painter(Some(Box::new(|_: &str, _: f32| Some(HostColorGlyph { width: 6, height: 6, left: 0, top: 0, rgba: [0, 0, 255, 255].repeat(36) }))));
    let at_pen: Vec<GlyphQuad> = e.shape("a😀", px(40.0), None, None, 0.0, 0.0, 0.0, 0);
    assert_eq!((quads[1].x - at_pen[1].x, quads[1].y - at_pen[1].y), (2.0, -5.0), "{quads:?} vs {at_pen:?}");
    assert_eq!(color_texel_of(&e, &at_pen[1]), [0, 0, 255, 255], "the new painter's pixels");
    // A host that declines leaves the emoji to swash.
    e.set_host_color_glyph_painter(Some(Box::new(|_: &str, _: f32| None)));
    let declined: Vec<GlyphQuad> = e.shape("a😀", px(40.0), None, None, 0.0, 0.0, 0.0, 0);
    assert!(declined[1].color && declined[1].w > 20.0, "swash's own raster: {declined:?}");
}

/// A host raster that fills a page of the emoji atlas goes on on a new
/// page, as letters do (`a_run_that_fills_a_page_goes_on_on_a_new_page`):
/// the run is shaped once, and each emoji's uv is into its own page.
#[test]
#[ignore = "needs Windows' Segoe UI Emoji font: run with --include-ignored on Windows"]
fn a_host_emoji_that_fills_an_emoji_page_goes_on_on_a_new_page() {
    let mut e: TextEngine = engine_with_emoji();
    e.set_host_color_glyph_painter(Some(Box::new(|_: &str, _: f32| Some(HostColorGlyph { width: 120, height: 120, left: 0, top: 120, rgba: [255, 0, 0, 255].repeat(120 * 120) }))));
    e.take_work();
    // Sixteen 120-px squares fill a page 512 a side, four to a shelf:
    // the last four go on a second page. (A bigger square is a big glyph
    // and goes into no atlas.)
    let quads: Vec<GlyphQuad> = e.shape("\u{1F600}\u{1F601}\u{1F602}\u{1F603}\u{1F604}\u{1F605}\u{1F606}\u{1F607}\u{1F608}\u{1F609}\u{1F60A}\u{1F60B}\u{1F60C}\u{1F60D}\u{1F60E}\u{1F60F}\u{1F610}\u{1F611}\u{1F612}\u{1F613}", px(40.0), None, None, 0.0, 0.0, 0.0, 0);
    let work: TextWork = e.take_work();
    assert_eq!((work.pages_added, work.shapes_to_draw), (2, 1), "the first page and one more, the run shaped once: {work:?}");
    assert_eq!((e.color_atlas().page_count(), e.color_atlas().side()), (2, COLOR_ATLAS_PAGE_SIDE));
    assert_eq!(quads.len(), 20, "{quads:?}");
    assert!(quads[..16].iter().all(|q| q.page == 0) && quads[16..].iter().all(|q| q.page == 1), "sixteen on the first page, four on the second: {quads:?}");
    for q in &quads {
        assert!(q.color && q.big_glyph.is_none());
        assert_eq!(color_texel_of(&e, q), [255, 0, 0, 255], "every slot is into its own page: {q:?}");
    }
}

/// Past `ATLAS_PAGES_KEPT` pages of emoji, the page used longest ago is
/// emptied as a letters' page is
/// (`past_the_kept_pages_the_page_used_longest_ago_is_emptied`): an emoji
/// on it is shaped and painted again when next shown.
///
/// Red when an emptied emoji page keeps the kept shapes or the host's
/// slots on it: the emoji is drawn from texels other emoji were packed
/// over.
#[test]
#[ignore = "needs Windows' Segoe UI Emoji font: run with --include-ignored on Windows"]
fn past_the_kept_pages_an_emoji_page_used_longest_ago_is_emptied() {
    let mut e: TextEngine = engine_with_emoji();
    let painted: std::rc::Rc<std::cell::Cell<u32>> = std::rc::Rc::new(std::cell::Cell::new(0));
    let count = painted.clone();
    e.set_host_color_glyph_painter(Some(Box::new(move |_: &str, _: f32| {
        count.set(count.get() + 1);
        Some(HostColorGlyph { width: 120, height: 120, left: 0, top: 120, rgba: [255, 0, 0, 255].repeat(120 * 120) })
    })));
    e.start_frame();
    e.shape("\u{1F600}", px(40.0), None, None, 0.0, 0.0, 0.0, 0);
    // Sixteen 120-px squares fill a page: each frame shows sixteen emoji
    // at a new size, a page each, until one is emptied.
    let sixteen: String = (0x1F600..0x1F610).filter_map(char::from_u32).collect();
    let mut font_size: f32 = 40.0;
    loop {
        assert!(font_size < 50.0, "the precondition: the sizes fill the pages");
        e.start_frame();
        e.take_work();
        e.shape(&sixteen, px(font_size), None, None, 0.0, 0.0, 0.0, 0);
        font_size += 1.0;
        let work: TextWork = e.take_work();
        if work.pages_emptied > 0 {
            assert_eq!(work.pages_added, 0, "{work:?}");
            break;
        }
    }
    assert_eq!(e.color_atlas().page_count(), ATLAS_PAGES_KEPT);
    e.start_frame();
    e.take_work();
    let painted_before: u32 = painted.get();
    e.shape("\u{1F600}", px(40.0), None, None, 0.0, 0.0, 0.0, 0);
    assert_eq!((e.take_work().shapes_to_draw, painted.get() - painted_before), (1, 1), "the emoji on the emptied page is shaped and painted again");

    // A frame that shows the kept shapes of every page, then needs room:
    // a page more, none emptied. The sixteen of the three sizes before
    // are each on a page of their own, the one emoji on the fourth.
    e.start_frame();
    for size in [font_size - 3.0, font_size - 2.0, font_size - 1.0] {
        e.shape(&sixteen, px(size), None, None, 0.0, 0.0, 0.0, 0);
    }
    e.shape("\u{1F600}", px(40.0), None, None, 0.0, 0.0, 0.0, 0);
    e.take_work();
    e.shape(&sixteen, px(font_size), None, None, 0.0, 0.0, 0.0, 0);
    let work: TextWork = e.take_work();
    assert_eq!((work.pages_added, work.pages_emptied), (1, 0), "no page this frame uses is emptied: {work:?}");
    assert_eq!(e.color_atlas().page_count(), ATLAS_PAGES_KEPT + 1);
}

/// A fallback file serves the glyphs no page font has: Roboto has no
/// Arabic, Windows' Arial does. The face gets no key, a missing file
/// is a false, and the measure cache is dropped so a label measured
/// as boxes is measured again. Ignored unless asked for: Arial is a
/// Windows file (see `engine_with_emoji`).
#[test]
#[ignore = "needs Windows' Arial font: run with --include-ignored on Windows"]
fn a_fallback_file_serves_glyphs_no_page_font_has() {
    let arial = Path::new("C:/Windows/Fonts/arial.ttf");
    assert!(arial.exists(), "this test needs {}", arial.display());
    let mut e = engine();
    assert!(!e.has_glyph('س'), "Roboto has no Arabic");
    let boxes_w: f32 = e.measure("سلام", px(20.0), None, None).0;
    assert!(e.add_fallback_font_file(arial));
    assert!(e.has_glyph('س'));
    assert!(!e.has_face("C:/Windows/Fonts/arial.ttf"), "a fallback face has no key");
    let glyphs_w: f32 = e.measure("سلام", px(20.0), None, None).0;
    assert_ne!(boxes_w, glyphs_w, "the measure cache was dropped");
    assert!(!e.add_fallback_font_file(Path::new("C:/Windows/Fonts/no-such-font.ttf")));
}

/// The Roboto faces' metrics as the files carry them (the layout
/// contract reads them, never a fitted constant).
#[test]
fn roboto_faces_carry_their_metrics() {
    for name in ["Roboto Regular", "Roboto Light", "Roboto Medium", "Roboto Bold"] {
        let path = format!("{}/../../assets/Fonts/{name}.ttf", env!("CARGO_MANIFEST_DIR"));
        let bytes = std::fs::read(&path).expect("font");
        let e = TextEngine::new(&[(name, bytes)]);
        let m = e.metrics(None);
        println!("{name}: ascent {} descent {} upem {} -> 24px {} 28px {} 36px {} 48px {}", m.ascent, m.descent, m.units_per_em, m.font_height(24.0), m.font_height(28.0), m.font_height(36.0), m.font_height(48.0));
        assert!(m.units_per_em > 0.0);
    }
}

#[test]
fn measures_wraps_and_rasterizes() {
    let mut e = engine();
    let (w, h) = e.measure("Hello", px(20.0), None, None);
    assert!(w > 30.0 && w < 70.0, "{w}");
    assert_eq!(h, 24.0);
    // Whole-pixel ascent + descent at each size, not a ratio of it.
    let m = e.metrics(None);
    assert_eq!(m.font_height(24.0), 29.0);
    assert_eq!(m.font_height(28.0), 33.0);
    assert_eq!(m.font_height(20.0), 24.0);
    let (_, h2) = e.measure("one two three four five six seven", px(20.0), Some(100.0), None);
    assert!(h2 >= 48.0, "{h2}");
    assert_eq!(e.atlas().page_count(), 0, "no letter yet: no page");
    let quads = e.shape("Hi", px(20.0), None, None, 10.0, 5.0, 0.0, 0);
    assert_eq!(quads.len(), 2);
    assert!(quads[0].x >= 10.0 && quads[1].x > quads[0].x);
    assert_eq!((e.atlas().page_count(), e.atlas().side(), e.atlas().texel_bytes()), (1, ATLAS_PAGE_SIDE, 1));
    assert!(e.atlas().page(0).iter().any(|b| *b > 128), "the page has ink");
    // The page has a part to send when glyphs are packed into it, none
    // when a text's glyphs are in it already, and the whole of it for a
    // renderer that sent another engine's page.
    let (part, packed): (AtlasPart, AtlasSent) = e.atlas().part_to_send(0, AtlasSent::default());
    assert!(!part.is_empty(), "two glyphs were packed");
    e.shape("Hi", px(20.0), None, None, 40.0, 5.0, 0.0, 0);
    assert!(e.atlas().part_to_send(0, packed).0.is_empty(), "nothing new was packed");
    let mut other = engine();
    other.shape("Hi", px(20.0), None, None, 10.0, 5.0, 0.0, 0);
    assert_eq!(other.atlas().part_to_send(0, packed).0, whole(1024), "a second engine with the same two glyphs");
}

/// A style suffix on the font key is a faux style: bold grows the
/// coverage, italic skews it — the same glyph, its own atlas slot.
#[test]
fn a_style_suffix_fakes_bold_and_italic() {
    let mut e = engine();
    let key = "assets/Fonts/Roboto Regular.ttf";
    let plain = e.glyph("H", 24.0, Some(key)).unwrap();
    let bold = e.glyph("H", 24.0, Some(&styled_key(key, true, false))).unwrap();
    let italic = e.glyph("H", 24.0, Some(&styled_key(key, false, true))).unwrap();
    // 24 px → 1 px of growth each side.
    assert_eq!(bold.w, plain.w + 2.0, "bold {bold:?} vs {plain:?}");
    assert_eq!(bold.h, plain.h + 2.0);
    assert!(italic.w > plain.w, "the skew widens the ink box: {italic:?} vs {plain:?}");
    assert_eq!(e.measure("Hi", px(24.0), None, Some(key)), e.measure("Hi", px(24.0), None, Some(&styled_key(key, true, false))), "bold keeps the advances");
}

/// A face added by key after construction shapes under that key.
#[test]
fn a_face_added_later_shapes_under_its_key() {
    let mut e = engine();
    assert!(!e.has_face("Heavy"));
    let bytes = std::fs::read(concat!(env!("CARGO_MANIFEST_DIR"), "/../../assets/Fonts/Roboto Bold.ttf")).expect("font");
    assert!(e.add_face("Heavy", bytes));
    assert!(e.has_face("Heavy") && e.has_face("Heavy#bold"));
    let (regular, _) = e.measure("The quick brown fox", px(24.0), None, None);
    let (heavy, _) = e.measure("The quick brown fox", px(24.0), None, Some("Heavy"));
    assert!((regular - heavy).abs() > 1.0, "a different face measures differently: {regular} vs {heavy}");
    assert!(!e.add_face("junk", vec![0, 1, 2, 3]), "no face in those bytes");
}

/// `take_work` counts each costly step when it ran, and nothing for a
/// text the engine has seen: a new text is shaped once to measure it and
/// once more to draw it, and each of its letters is drawn once.
///
/// Needed so a host can see a text shaped more often than that: every
/// count here is a lookup that found nothing.
#[test]
fn the_work_counts_each_shape_and_each_letter_drawn() {
    let counts = |work: gogo_text::TextWork| (work.shapes_to_measure, work.shapes_to_draw, work.letters_drawn, work.pages_added);
    let mut e = engine();
    assert_eq!(e.take_work(), gogo_text::TextWork::default(), "a new engine did nothing");
    e.measure("Hello", px(24.0), None, None);
    let work = e.take_work();
    assert_eq!(counts(work), (1, 0, 0, 0), "a measure shapes the text and draws no letter");
    assert!(work.seconds_shaping_to_measure > 0.0);
    e.shape("Hello", px(24.0), None, None, 0.0, 0.0, 0.0, 0);
    let work = e.take_work();
    assert_eq!((work.shapes_to_measure, work.shapes_to_draw, work.pages_added), (0, 1, 1), "the draw shapes the same text once more, and its letters make the atlas's first page");
    assert!((4..=5).contains(&work.letters_drawn), "H, e, l, o, and the second l when it sits at another part of a pixel: {}", work.letters_drawn);
    e.measure("Hello", px(24.0), None, None);
    e.shape("Hello", px(24.0), None, None, 0.0, 0.0, 0.0, 0);
    assert_eq!(e.take_work(), gogo_text::TextWork::default(), "a text seen before costs two lookups");
    e.narrowest_wrap("Hello there", px(24.0), None);
    assert_eq!(counts(e.take_work()), (1, 0, 0, 0), "the narrowest width of a wrapping text is a shape of its own");
    // The run that fills the first page: one page more, the text shaped
    // once, no letter drawn twice.
    let (_, work, _) = run_that_fills_the_first_page(&mut e);
    assert!(work.pages_added == 1 && work.shapes_to_draw == 1 && work.letters_drawn <= ALPHABET.len() as u32, "{work:?}");
}

/// The OS scan runs once, names each family once, and a family can
/// become a face. Skips its OS-specific half where the folders are
/// empty (a bare CI box).
#[test]
fn system_fonts_load_once_and_name_families() {
    let mut e = engine();
    assert!(e.system_families().is_empty());
    let n = e.load_system_fonts();
    assert_eq!(e.load_system_fonts(), n, "the second call is a no-op");
    let names = e.system_families().to_vec();
    assert_eq!(names.len(), n);
    let mut sorted = names.clone();
    sorted.dedup();
    assert_eq!(sorted, names, "sorted, no repeats");
    if names.is_empty() {
        eprintln!("SKIP: no system fonts on this box");
        return;
    }
    assert!(!e.add_system_face("os", "no such family 0x1234"));
    assert!(e.add_system_face("os", &names[0]));
    let (w, _) = e.measure("Hello", px(24.0), None, Some("os"));
    assert!(w > 0.0);
}

/// One glyph reports its ink box — what a `TextStretch` maps onto its
/// rect (an ornament font's bubble stretched over a callout).
#[test]
fn glyph_reports_an_ink_box() {
    let mut e = engine();
    let bytes = std::fs::read(concat!(env!("CARGO_MANIFEST_DIR"), "/../../assets/Fonts/Roboto Bold.ttf")).expect("font");
    assert!(e.add_face("second", bytes));
    let g = e.glyph("0", 100.0, Some("second")).expect("ink");
    assert!(g.w > 20.0 && g.h > 20.0, "{g:?}");
    assert!(e.glyph(" ", 100.0, Some("second")).is_none(), "a space has no ink");
}

/// The top of the ink of each line of a shaped run: the smallest glyph
/// `y` among the glyphs that share a row, rows top to bottom.
fn line_tops(quads: &[GlyphQuad]) -> Vec<f32> {
    let mut tops: Vec<f32> = Vec::new();
    for q in quads {
        if !tops.contains(&q.y) {
            tops.push(q.y);
        }
    }
    tops.sort_by(|a, b| a.partial_cmp(b).expect("no NaN"));
    tops
}

/// The lines of a wrapped label sit one font height plus the line gap
/// apart, and its box is that many lines with a gap between each two:
/// 1 pixel on a screen with 1 device pixel per epx, 3 pixels on a phone
/// with 3.
///
/// Red when the gap is a fixed number of device pixels: on the 1-pixel
/// screen the lines of a wrapped label then sit 3 pixels apart, three
/// times as loose as the same label on the phone.
#[test]
fn the_lines_of_a_wrapped_label_are_one_epx_apart_on_every_screen() {
    let mut e = engine();
    let m: FaceMetrics = e.metrics(None);
    // "H" three times, in a box one "H" wide: three lines, the same glyph
    // on each, so the rows of ink are exactly one line pitch apart.
    for (epx, gap) in [(1.0f32, 1.0f32), (2.0, 2.0), (3.0, 3.0)] {
        let size: TextPx = TextPx::from_epx(20.0, epx);
        assert_eq!(size.line_gap, gap, "at {epx} device pixels per epx");
        let (one_w, one_h) = e.measure("H", size, None, None);
        assert_eq!(one_h, m.font_height(size.font_size), "one line has no gap");
        let narrow: Option<f32> = Some(one_w + 1.0);
        let (_, h) = e.measure("H H H", size, narrow, None);
        assert_eq!(h, 3.0 * one_h + 2.0 * gap, "three lines and two gaps at {epx} device pixels per epx");
        let tops: Vec<f32> = line_tops(&e.shape("H H H", size, narrow, None, 0.0, 0.0, one_w + 1.0, 0));
        assert_eq!(tops.len(), 3, "{tops:?}");
        assert_eq!((tops[1] - tops[0], tops[2] - tops[1]), (one_h + gap, one_h + gap), "the rows of ink at {epx} device pixels per epx");
        // The last row of ink ends inside the box the measure gave.
        let one: Vec<GlyphQuad> = e.shape("H", size, None, None, 0.0, 0.0, 0.0, 0);
        assert_eq!(tops[0], one[0].y, "the first line sits where a one-line label's does");
        assert!(tops[2] + one[0].h <= h, "the third line's ink ends at {} in a box {h} tall", tops[2] + one[0].h);
    }
}

/// A one-line label sits at the same height whatever the line gap is: the
/// gap is space BETWEEN lines, none above the first.
#[test]
fn one_line_sits_the_same_at_every_line_gap() {
    let mut e = engine();
    let at = |e: &mut TextEngine, line_gap: f32| -> Vec<(f32, f32)> { e.shape("Hxg", TextPx { font_size: 24.0, line_gap }, None, None, 0.0, 0.0, 0.0, 0).iter().map(|q| (q.x, q.y)).collect() };
    let none: Vec<(f32, f32)> = at(&mut e, 0.0);
    assert_eq!(none.len(), 3);
    for line_gap in [1.0, 2.0, 3.0, 4.0] {
        assert_eq!(at(&mut e, line_gap), none, "a gap of {line_gap}");
    }
}

/// The layout's measure answers in epx for the pixels the renderer will
/// draw: a wrapped label's height is its lines and gaps in device
/// pixels, divided by the scale.
#[test]
fn the_layouts_measure_counts_the_gap_of_its_screen() {
    let engine: RefCell<TextEngine> = RefCell::new(engine());
    let fh_px: f32 = engine.borrow().metrics(None).font_height(40.0);
    let measure = EpxMeasure { engine: &engine, epx: 2.0, font: None, images: &[] };
    // 20 epx is 40 device pixels here; the gap is 2.
    let (one_w, one_h) = measure.measure("H", 20.0, None, None);
    assert_eq!(one_h, fh_px / 2.0);
    let (_, h) = measure.measure("H H H", 20.0, Some(one_w + 0.5), None);
    assert_eq!(h, (3.0 * fh_px + 2.0 * 2.0) / 2.0);
}

/// The narrowest a text folds to is its longest word. `measure` at a wrap
/// width of 0 cannot say it: it breaks between letters.
#[test]
fn the_narrowest_a_text_folds_to_is_its_longest_word() {
    let engine: RefCell<TextEngine> = RefCell::new(engine());
    let measure = EpxMeasure { engine: &engine, epx: 1.0, font: None, images: &[] };
    let text = "hello wonderful world";
    let (word, _) = measure.measure("wonderful", 15.0, None, None);
    let (letter, _) = measure.measure(text, 15.0, Some(0.0), None);
    assert!(letter < word / 3.0, "a wrap width of 0 breaks between letters: {letter} against {word}");
    assert_eq!(measure.narrowest_wrap(text, 15.0, None), word);
    assert_eq!(measure.narrowest_wrap("wonderful", 15.0, None), word, "one word");
    assert_eq!(measure.narrowest_wrap("", 15.0, None), 0.0, "no text");
    // On a screen with two device pixels per epx the answer is still in epx.
    let twice = EpxMeasure { engine: &engine, epx: 2.0, font: None, images: &[] };
    assert_eq!(twice.narrowest_wrap(text, 15.0, None), twice.measure("wonderful", 15.0, None, None).0);
}

/// A text of several lines folds to its longest word, whichever line the
/// word is on.
///
/// Needed because a letter's byte positions count from the start of its
/// OWN line. Read against the whole text, the letters of a second line
/// are taken for the letters of the first: a letter under a space counts
/// as a space, and a position inside a character of several bytes ("—")
/// stops the program.
#[test]
fn a_second_line_folds_to_its_own_longest_word() {
    let engine: RefCell<TextEngine> = RefCell::new(engine());
    let measure = EpxMeasure { engine: &engine, epx: 1.0, font: None, images: &[] };
    let (word, _) = measure.measure("wonderful", 15.0, None, None);
    // The first line has spaces where the second has its word.
    assert_eq!(measure.narrowest_wrap("x         \nwonderful", 15.0, None), word, "a word under a line of spaces");
    // The first line's dashes are three bytes each.
    assert_eq!(measure.narrowest_wrap("—— ——\nab wonderful", 15.0, None), word, "a word under a line of long dashes");
}

/// A wrapping label that fills what is left of a Row too narrow for it
/// keeps its longest word whole. Red when the label's smallest width
/// comes from a measure at a wrap width of 0: with Roboto that is one
/// letter, and the label wraps a letter per line.
#[test]
fn a_wrapping_label_in_a_tight_row_keeps_its_longest_word_whole() {
    let engine: RefCell<TextEngine> = RefCell::new(engine());
    let measure = EpxMeasure { engine: &engine, epx: 1.0, font: None, images: &[] };
    let c = gogo_ui::compile("View v {\n Row r width=60, halign=left, valign=top {\n Rectangle k width=50, height=10\n Label t text=\"hello wonderful world\", wrap=true, width=fill\n }\n}", &|_| None);
    assert!(!gogo_ui::has_errors(&c.diags), "{:#?}", c.diags);
    let rt = gogo_ui::Runtime::new(&c.page);
    let look = gogo_ui::resolve(&c.page, &rt);
    let rects = gogo_ui::layout(&c.page, &look, gogo_ui::Rect { x: 0.0, y: 0.0, w: 400.0, h: 800.0 }, &measure);
    let label = rects[c.page.find("t").unwrap().index()];
    let (word, _) = measure.measure("wonderful", 15.0, None, None);
    assert_eq!(label.w, word, "as wide as `wonderful`");
    assert_eq!(label.h, measure.measure("hello wonderful world", 15.0, Some(word + 0.01), None).1, "one word to a line");
}

/// The whole of an atlas page `size` texels a side.
fn whole(size: u32) -> AtlasPart {
    AtlasPart { columns: 0..size, rows: 0..size }
}

/// What a renderer's texture holds of one atlas page: it starts as a new
/// texture does, all zeros, and takes only the part the engine names.
///
/// Needed by the tests of `Atlas::part_to_send`: the part is right when
/// such a copy equals the engine's page after every take.
#[derive(Clone, Default)]
struct PageCopy {
    sent: AtlasSent,
    bytes: Vec<u8>,
}

impl PageCopy {
    /// Copies the part of page `index` of `atlas` this copy lacks.
    ///
    /// Returns the part the engine named.
    fn take(&mut self, atlas: &Atlas, index: usize) -> AtlasPart {
        let (part, sent): (AtlasPart, AtlasSent) = atlas.part_to_send(index, self.sent);
        let (side, texel_bytes): (u32, u32) = (atlas.side(), atlas.texel_bytes());
        if self.bytes.is_empty() {
            self.bytes = vec![0; (side * side * texel_bytes) as usize];
        }
        let page: &[u8] = atlas.page(index);
        for row in part.rows.clone() {
            let (from, to) = (((row * side + part.columns.start) * texel_bytes) as usize, ((row * side + part.columns.end) * texel_bytes) as usize);
            self.bytes[from..to].copy_from_slice(&page[from..to]);
        }
        self.sent = sent;
        part
    }

    /// Whether the copy equals page `index` of `atlas`.
    fn holds(&self, atlas: &Atlas, index: usize) -> bool {
        self.bytes == atlas.page(index)
    }
}

/// A renderer that sends only the part `Atlas::part_to_send` names holds
/// the engine's whole page after each send: nothing for nothing new, what
/// the page holds so far for a new texture, a new letter's own columns of
/// its shelf for a new letter, whole rows when letters start new shelves,
/// the whole page for a texture that holds another page. Red when a
/// texel that changed is not named: the user sees a new letter as an
/// empty box, or cut off.
#[test]
fn a_copy_that_takes_only_the_part_to_send_holds_the_whole_page() {
    let mut e = engine();
    let mut copy = PageCopy::default();
    let shape = |e: &mut TextEngine, text: &str, font_size: f32| {
        e.shape(text, px(font_size), None, None, 0.0, 0.0, 100_000.0, 0);
    };
    shape(&mut e, "Hi", 20.0);
    let first: AtlasPart = copy.take(e.atlas(), 0);
    assert!(first.rows.start == 0 && first.columns.start == 0 && first.texel_count() < 64 * 64, "a new texture: what the page holds so far, not the whole page, {first:?}");
    assert!(copy.holds(e.atlas(), 0));

    shape(&mut e, "Hi", 20.0);
    assert!(copy.take(e.atlas(), 0).is_empty(), "nothing new was packed");

    shape(&mut e, "o", 20.0);
    let beside: AtlasPart = copy.take(e.atlas(), 0);
    assert!(beside.rows.start == 0 && beside.rows.end < 64, "a new letter beside the first two: rows of their shelf, {beside:?}");
    assert!(beside.columns.start > 0 && beside.columns.end - beside.columns.start < 32, "and its own columns only, {beside:?}");
    assert!(copy.holds(e.atlas(), 0), "the copy after one new letter");

    // A taller letter in the same shelf makes that shelf taller.
    shape(&mut e, "Q", 48.0);
    let taller: AtlasPart = copy.take(e.atlas(), 0);
    assert!(taller.columns.start >= beside.columns.end && taller.rows.start == 0 && taller.rows.end > beside.rows.end, "a taller letter in the same shelf: its columns, down to its foot, {taller:?}");
    assert!(copy.holds(e.atlas(), 0), "the copy after a taller letter");

    // 52 letters at 64 px are wider than the page: they fill the first
    // shelf and start others below it.
    shape(&mut e, ALPHABET, 64.0);
    let over_shelves: AtlasPart = copy.take(e.atlas(), 0);
    assert!(over_shelves.columns == (0..1024) && over_shelves.rows.start == 0, "letters over several shelves: whole rows from the shelf that was being filled, {over_shelves:?}");
    assert!(copy.holds(e.atlas(), 0), "the copy after letters over several shelves");

    shape(&mut e, "?", 64.0);
    let lower: AtlasPart = copy.take(e.atlas(), 0);
    assert!(lower.rows.start > 0 && lower.columns.start > 0 && lower.texel_count() < 128 * 128, "a letter in a shelf below the first: no row of the first shelf and no column sent before, {lower:?}");
    assert!(copy.holds(e.atlas(), 0), "the copy after a letter in a lower shelf");

    let mut second_renderer = PageCopy::default();
    let second_first: AtlasPart = second_renderer.take(e.atlas(), 0);
    assert!(second_first.columns == (0..1024) && second_first.rows.start == 0 && second_first.rows.end < 1024, "a second renderer's new texture: the rows used so far, {second_first:?}");
    assert!(second_renderer.holds(e.atlas(), 0));

    // Another engine's page has the same size and other letters in the
    // same places.
    let mut other = engine();
    shape(&mut other, "xyz", 20.0);
    let mut handed_another_engine: PageCopy = copy.clone();
    assert_eq!(handed_another_engine.take(other.atlas(), 0), whole(1024), "another engine's page: the whole of it");
    assert!(handed_another_engine.holds(other.atlas(), 0));

    run_that_fills_the_first_page(&mut e);
    assert_eq!(e.atlas().page_count(), 2, "the precondition: the first page is full");
    copy.take(e.atlas(), 0);
    assert!(copy.holds(e.atlas(), 0), "the copy of the full first page");
    let mut second_page = PageCopy::default();
    let new_page: AtlasPart = second_page.take(e.atlas(), 1);
    assert!(!new_page.is_empty() && new_page != whole(1024), "a new page: what it holds so far, not the whole of it, {new_page:?}");
    assert!(second_page.holds(e.atlas(), 1));

    shape(&mut e, "#", 20.0);
    assert!(copy.take(e.atlas(), 0).is_empty(), "a new letter on the second page: nothing of the first");
    let on_new_page: AtlasPart = second_page.take(e.atlas(), 1);
    assert!(!on_new_page.is_empty() && on_new_page.texel_count() < 64 * 64, "its own part of the second page, {on_new_page:?}");
    assert!(second_page.holds(e.atlas(), 1), "the copy after a letter on the second page");
}

/// A page emptied for new glyphs is a page the renderer's texture does
/// not hold: the next send is the whole of it, and the copy then equals
/// it. Red when only the new letters are named: the texture keeps the
/// letters of the page before, under quads that point elsewhere now.
#[test]
fn an_emptied_page_is_sent_whole() {
    let mut e = engine();
    let mut copies: Vec<PageCopy> = Vec::new();
    let mut font_size: f32 = 120.0;
    let mut emptied: Option<usize> = None;
    while emptied.is_none() {
        assert!(font_size > 30.0, "the precondition: the sizes fill the pages");
        e.start_frame();
        e.take_work();
        let quads: Vec<GlyphQuad> = e.shape(ALPHABET, px(font_size), None, None, 0.0, 0.0, 100_000.0, 0);
        font_size -= 1.0;
        if e.take_work().pages_emptied > 0 {
            emptied = quads.last().map(|quad| quad.page as usize);
            break;
        }
        copies.resize(e.atlas().page_count(), PageCopy::default());
        for (index, copy) in copies.iter_mut().enumerate() {
            copy.take(e.atlas(), index);
        }
    }
    let index: usize = emptied.expect("a page was emptied");
    // Below the rows the new letters use, the emptied page holds no ink
    // of the letters before: a new letter's edge would show it.
    let used: AtlasPart = e.atlas().part_to_send(index, AtlasSent::default()).0;
    let first_unused: usize = (used.rows.end * ATLAS_PAGE_SIDE) as usize;
    assert!(e.atlas().page(index)[first_unused..].iter().all(|texel| *texel == 0), "the emptied page is cleared below row {}", used.rows.end);
    assert_eq!(copies[index].take(e.atlas(), index), whole(1024), "the emptied page: the whole of it");
    assert!(copies[index].holds(e.atlas(), index));
}

/// The emoji atlas is sent the same way: no page before the first emoji,
/// what the page holds at the first, a new emoji's own columns after.
#[test]
#[ignore = "needs Windows' Segoe UI Emoji font: run with --include-ignored on Windows"]
fn a_copy_that_takes_only_the_part_to_send_holds_the_whole_emoji_page() {
    let mut e: TextEngine = engine_with_emoji();
    let mut copy = PageCopy::default();
    e.shape("a", px(40.0), None, None, 0.0, 0.0, 0.0, 0);
    assert_eq!(e.color_atlas().page_count(), 0, "no emoji was packed: no page to send");

    e.shape("\u{1F600}", px(40.0), None, None, 0.0, 0.0, 0.0, 0);
    let first: AtlasPart = copy.take(e.color_atlas(), 0);
    assert!(first.rows.start == 0 && first.columns.start == 0 && first != whole(COLOR_ATLAS_PAGE_SIDE), "the first emoji: what the page holds, {first:?}");
    assert!(copy.holds(e.color_atlas(), 0));

    e.shape("\u{1F388}", px(40.0), None, None, 0.0, 0.0, 0.0, 0);
    let beside: AtlasPart = copy.take(e.color_atlas(), 0);
    assert!(beside.rows.start == 0 && beside.rows.end < 128 && beside.columns.start > 0, "a second emoji beside the first: its own columns of their shelf, {beside:?}");
    assert!(copy.holds(e.color_atlas(), 0), "the copy after a second emoji");
}

/// Measures `text` both ways and shapes it, as a layout and a renderer
/// do for a label.
///
/// Returns what that cost: texts shaped to measure, texts shaped to
/// draw, letters drawn.
fn work_of_showing(e: &mut TextEngine, text: &str) -> (u32, u32, u32) {
    e.take_work();
    e.measure(text, px(20.0), None, None);
    e.narrowest_wrap(text, px(20.0), None);
    e.shape(text, px(20.0), None, None, 0.0, 0.0, 0.0, 0);
    let work = e.take_work();
    (work.shapes_to_measure, work.shapes_to_draw, work.letters_drawn)
}

/// A font added later, as a fallback file or with the device's fonts,
/// leaves alone a text its own font drew whole: shown again, it is
/// neither measured nor shaped. A text with a letter no font had is
/// measured and shaped again, since the new font may have that letter.
///
/// Red when a font added drops every kept text: the frame after the
/// device's fonts were read measured and shaped the whole screen again,
/// 80 texts in 35 ms on an old phone, and none came out different. Red
/// too when it drops none: a label shown as boxes stays boxes.
#[test]
fn a_font_added_later_leaves_alone_the_texts_their_own_font_drew_whole() {
    let mut e = engine();
    let own: &str = "Save changes?";
    let with_a_missing_letter: &str = "Hi \u{0633}";
    assert!(!e.has_glyph('\u{0633}'), "the precondition: Roboto has no Arabic");
    assert_eq!(work_of_showing(&mut e, own).0, 2, "the precondition: a new text is measured twice, as a size and as its narrowest width");
    work_of_showing(&mut e, with_a_missing_letter);

    // Another Roboto file as a fallback: it has no Arabic either.
    assert!(e.add_fallback_font_file(Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../../assets/Fonts/Roboto Bold.ttf"))));
    assert_eq!(work_of_showing(&mut e, own), (0, 0, 0), "a text of one font is kept over a font added");
    assert_eq!(work_of_showing(&mut e, with_a_missing_letter), (2, 1, 0), "a text with a missing letter is measured and shaped again; its letters are in the atlas");
    assert_eq!(work_of_showing(&mut e, with_a_missing_letter), (0, 0, 0), "and kept from then on");

    e.load_system_fonts();
    assert_eq!(work_of_showing(&mut e, own), (0, 0, 0), "a text of one font is kept over the device's fonts");
    let (measured, shaped, _) = work_of_showing(&mut e, with_a_missing_letter);
    assert_eq!((measured, shaped), (2, 1), "a text with a missing letter is measured and shaped again after the device's fonts");
}

/// A text with a letter another font drew is measured and shaped again
/// when a font is added: the new font may be the one asked first for
/// that letter. Ignored unless asked for: Arial and Segoe UI Emoji are
/// Windows files (see `engine_with_emoji`).
#[test]
#[ignore = "needs Windows' Arial and Segoe UI Emoji fonts: run with --include-ignored on Windows"]
fn a_font_added_later_has_a_text_with_a_fallback_letter_shaped_again() {
    let mut e = engine();
    assert!(e.add_fallback_font_file(Path::new("C:/Windows/Fonts/arial.ttf")));
    let with_a_fallback_letter: &str = "Hi \u{0633}";
    assert!(e.has_glyph('\u{0633}'), "the precondition: Arial has the Arabic letter");
    work_of_showing(&mut e, "Hi");
    work_of_showing(&mut e, with_a_fallback_letter);
    assert!(e.add_fallback_font_file(Path::new("C:/Windows/Fonts/seguiemj.ttf")));
    assert_eq!(work_of_showing(&mut e, "Hi"), (0, 0, 0), "a text of one font is kept");
    assert_eq!(work_of_showing(&mut e, with_a_fallback_letter), (2, 1, 0), "a text with a letter of another font is measured and shaped again");
}

/// The texel of a big glyph at column `x` of row `y`: its bytes.
fn big_texel<'a>(glyph: &BigGlyph<'a>, x: u32, y: u32) -> &'a [u8] {
    let texel_bytes: usize = if glyph.is_color { 4 } else { 1 };
    &glyph.texels[(y * glyph.width + x) as usize * texel_bytes..][..texel_bytes]
}

/// Checks the raster of the big glyph `quad` names: the quad's size
/// plus one texel on every side, that border empty, ink inside it, and
/// the quad's uv over the ink alone.
///
/// Returns the glyph's id.
fn check_big_glyph(e: &TextEngine, quad: &GlyphQuad) -> BigGlyphId {
    let id: BigGlyphId = quad.big_glyph.expect("a big glyph");
    let glyph: BigGlyph = e.big_glyph(id).expect("the engine keeps the big glyph a quad names");
    assert_eq!(glyph.is_color, quad.color);
    assert_eq!((glyph.width, glyph.height), (quad.w as u32 + 2, quad.h as u32 + 2), "the ink and one texel on every side");
    assert_eq!(glyph.texels.len(), (glyph.width * glyph.height) as usize * if glyph.is_color { 4 } else { 1 });
    let is_empty = |x: u32, y: u32| big_texel(&glyph, x, y).iter().all(|byte| *byte == 0);
    assert!((0..glyph.width).all(|x| is_empty(x, 0) && is_empty(x, glyph.height - 1)), "the top and the bottom row are empty");
    assert!((0..glyph.height).all(|y| is_empty(0, y) && is_empty(glyph.width - 1, y)), "the left and the right column are empty");
    // The ink reaches the border on every side, give or take a texel.
    assert!((1..glyph.width - 1).any(|x| !is_empty(x, 1) || !is_empty(x, 2)) && (1..glyph.width - 1).any(|x| !is_empty(x, glyph.height - 2) || !is_empty(x, glyph.height - 3)), "ink at the top and at the bottom");
    assert!((1..glyph.height - 1).any(|y| !is_empty(1, y) || !is_empty(2, y)) && (1..glyph.height - 1).any(|y| !is_empty(glyph.width - 2, y) || !is_empty(glyph.width - 3, y)), "ink at the left and at the right");
    let (width, height): (f32, f32) = (glyph.width as f32, glyph.height as f32);
    assert_eq!(quad.uv, [1.0 / width, 1.0 / height, (width - 1.0) / width, (height - 1.0) / height], "the uv leaves the border out");
    id
}

/// A glyph drawn wider or taller than `ATLAS_GLYPH_MAX_PX` goes into no
/// atlas: the engine keeps its raster by itself, and the quad names it.
/// A smaller one goes into the atlas as before.
///
/// Red when a big glyph is packed into the atlas: one letter of 400 px
/// makes a shelf 400 rows tall, the atlas fills and doubles after a few
/// of them, and each new shelf sends those rows again.
#[test]
fn a_glyph_too_big_for_the_atlas_is_kept_by_itself() {
    let mut e = engine();
    e.take_work();
    let quads: Vec<GlyphQuad> = e.shape("Wg", px(400.0), None, None, 0.0, 0.0, 0.0, 0);
    let work: TextWork = e.take_work();
    assert_eq!((quads.len(), work.letters_drawn, work.pages_added), (2, 2, 0));
    assert_eq!(e.atlas().page_count(), 0, "nothing went into the letters' atlas");
    let ids: Vec<BigGlyphId> = quads.iter().map(|quad| check_big_glyph(&e, quad)).collect();
    assert_ne!(ids[0], ids[1], "each big glyph has its own id");

    // The same text again: the kept shape, the same glyphs.
    let again: Vec<GlyphQuad> = e.shape("Wg", px(400.0), None, None, 0.0, 0.0, 0.0, 0);
    assert_eq!(e.take_work(), TextWork::default());
    assert_eq!(again.iter().map(|quad| quad.big_glyph).collect::<Vec<_>>(), ids.iter().map(|id| Some(*id)).collect::<Vec<_>>());

    // 26 letters of 400 px would fill most of five pages.
    e.shape("ABCDEFGHIJKLMNOPQRSTUVWXYZ", px(400.0), None, None, 0.0, 0.0, 100_000.0, 0);
    assert_eq!((e.take_work().pages_added, e.atlas().page_count()), (0, 0), "big letters fill no atlas");

    // The limit is on the raster's sides, not on the font size.
    let (mut in_the_atlas, mut big): (u32, u32) = (0, 0);
    for font_size in [100.0, 130.0, 150.0, 170.0, 190.0, 220.0] {
        for quad in e.shape("Wi.", px(font_size), None, None, 0.0, 0.0, 0.0, 0) {
            let is_over: bool = quad.w.max(quad.h) > ATLAS_GLYPH_MAX_PX as f32;
            assert_eq!(quad.big_glyph.is_some(), is_over, "{font_size} px: {quad:?}");
            if is_over {
                check_big_glyph(&e, &quad);
                big += 1;
            } else {
                in_the_atlas += 1;
            }
        }
    }
    assert!(in_the_atlas >= 5 && big >= 5, "the precondition: glyphs on both sides of the limit, {in_the_atlas} and {big}");
    assert!(e.atlas().page_count() == 1 && !e.atlas().part_to_send(0, AtlasSent::default()).0.is_empty(), "the small ones went into the atlas");

    // A faux bold big glyph is kept at its grown size: 8 px more on
    // every side at 320 px.
    let key: &str = "assets/Fonts/Roboto Regular.ttf";
    let plain: GlyphQuad = e.glyph("H", 320.0, Some(key)).expect("ink");
    let bold: GlyphQuad = e.glyph("H", 320.0, Some(&styled_key(key, true, false))).expect("ink");
    assert_eq!((bold.w - plain.w, bold.h - plain.h), (16.0, 16.0));
    assert_ne!(check_big_glyph(&e, &plain), check_big_glyph(&e, &bold));
}

/// A big glyph and the kept shape of its text stay when a page of the
/// letters' atlas fills: the glyph is in no atlas. Red when a full page
/// drops the kept shapes: the text is shaped again in that frame.
#[test]
fn a_big_glyph_is_not_drawn_again_when_a_page_fills() {
    let mut e = engine();
    let before: Vec<GlyphQuad> = e.shape("W", px(400.0), None, None, 0.0, 0.0, 0.0, 0);
    let id: BigGlyphId = check_big_glyph(&e, &before[0]);
    run_that_fills_the_first_page(&mut e);
    e.take_work();
    let after: Vec<GlyphQuad> = e.shape("W", px(400.0), None, None, 0.0, 0.0, 0.0, 0);
    let work: TextWork = e.take_work();
    assert_eq!((work.shapes_to_draw, work.letters_drawn), (0, 0), "its kept shape, its glyph drawn");
    assert_eq!(after[0].big_glyph, Some(id));
    assert_eq!((after[0].uv, after[0].w, after[0].h), (before[0].uv, before[0].w, before[0].h));
    check_big_glyph(&e, &after[0]);
}

/// The bytes of the big glyphs of `ids` that `e` still keeps.
fn kept_bytes(e: &TextEngine, ids: &[BigGlyphId]) -> usize {
    ids.iter().filter_map(|id| e.big_glyph(*id)).map(|glyph| glyph.texels.len()).sum()
}

/// A box resized by a drag shapes its glyph at a new size every frame.
/// The engine drops the sizes of earlier frames once they hold more than
/// `BIG_GLYPH_KEPT_BYTES`, the oldest first, and keeps the glyph that
/// every frame shows.
///
/// Red when nothing is dropped: a drag of a few seconds leaves hundreds
/// of megabytes of rasters behind. Red too when the drop takes a glyph
/// still shown: it is drawn again in the next frame.
#[test]
fn big_glyphs_of_earlier_frames_are_dropped_over_the_kept_bytes() {
    let mut e = engine();
    let shown: BigGlyphId = e.shape("S", px(300.0), None, None, 0.0, 0.0, 0.0, 0)[0].big_glyph.expect("a big glyph");
    let shown_in_a_moving_box: BigGlyphId = e.shape("O", px(300.0), None, None, 0.0, 0.0, 0.0, 0)[0].big_glyph.expect("a big glyph");
    let mut resized: Vec<BigGlyphId> = Vec::new();
    for frame in 0..40 {
        e.start_frame();
        e.take_work();
        assert_eq!(e.shape("S", px(300.0), None, None, 0.0, 0.0, 0.0, 0)[0].big_glyph, Some(shown), "frame {frame}: the glyph shown every frame is kept");
        assert_eq!(e.take_work(), TextWork::default(), "frame {frame}: and costs nothing");
        // A box of another width each frame: the text is shaped anew,
        // and finds its glyph drawn.
        assert_eq!(e.shape("O", px(300.0), None, None, 0.0, 0.0, 1.0 + frame as f32, 0)[0].big_glyph, Some(shown_in_a_moving_box), "frame {frame}: so is the glyph of a text shaped anew every frame");
        let work: TextWork = e.take_work();
        assert_eq!((work.shapes_to_draw, work.letters_drawn), (1, 0), "frame {frame}: and it is not drawn again");
        let quads: Vec<GlyphQuad> = e.shape("W", px(600.0 + frame as f32), None, None, 0.0, 0.0, 0.0, 0);
        resized.push(quads[0].big_glyph.expect("a big glyph"));
        assert!(e.big_glyph(resized[frame]).is_some(), "frame {frame}: the glyph just shaped is kept");
        if frame > 0 {
            assert!(e.big_glyph(resized[frame - 1]).is_some(), "frame {frame}: so is the one the frame before showed");
        }
    }
    let newest: usize = e.big_glyph(resized[39]).expect("kept").texels.len();
    assert!(newest * 40 > 2 * BIG_GLYPH_KEPT_BYTES, "the precondition: the 40 sizes are more than twice the kept bytes");
    let kept: Vec<bool> = resized.iter().map(|id| e.big_glyph(*id).is_some()).collect();
    assert!(!kept[0] && kept[39], "the first size is dropped, the last is kept");
    assert!(kept.windows(2).all(|pair| pair[0] <= pair[1]), "the oldest are dropped first: {kept:?}");
    // The last drop left no more than the kept bytes; one size came
    // after it.
    assert!(kept_bytes(&e, &resized) + kept_bytes(&e, &[shown, shown_in_a_moving_box]) <= BIG_GLYPH_KEPT_BYTES + newest, "{} bytes kept", kept_bytes(&e, &resized));
    assert!(kept_bytes(&e, &resized) > BIG_GLYPH_KEPT_BYTES / 2, "and not everything was dropped: {} bytes kept", kept_bytes(&e, &resized));

    // A dropped size is drawn again when a text needs it, under a new id.
    e.take_work();
    let again: Vec<GlyphQuad> = e.shape("W", px(600.0), None, None, 0.0, 0.0, 0.0, 0);
    let work: TextWork = e.take_work();
    assert_eq!((work.shapes_to_draw, work.letters_drawn), (1, 1), "the kept shape went with its glyph");
    assert_ne!(again[0].big_glyph, Some(resized[0]));
    check_big_glyph(&e, &again[0]);
}

/// What the last frame showed is never dropped, however much it is: a
/// screen whose big glyphs hold more than `BIG_GLYPH_KEPT_BYTES` keeps
/// them all from frame to frame. Once no frame shapes them, they go.
///
/// Red when the drop looks at the bytes alone: every frame draws some
/// of the screen's big letters again, 10 ms or more each on an old
/// phone.
#[test]
fn the_big_glyphs_a_frame_shows_are_kept_however_many() {
    let mut e = engine();
    let screen = |e: &mut TextEngine| -> Vec<BigGlyphId> { e.shape("ABCDEFGHIJKLMNOPQRSTUVWXYZ", px(700.0), None, None, 0.0, 0.0, 100_000.0, 0).iter().map(|quad| quad.big_glyph.expect("a big glyph")).collect() };
    e.start_frame();
    let first: Vec<BigGlyphId> = screen(&mut e);
    assert!(kept_bytes(&e, &first) > BIG_GLYPH_KEPT_BYTES, "the precondition: one frame's big glyphs are more than the kept bytes, {}", kept_bytes(&e, &first));
    for frame in 1..4 {
        e.start_frame();
        e.take_work();
        assert_eq!(screen(&mut e), first, "frame {frame}: the same glyphs");
        assert_eq!(e.take_work(), TextWork::default(), "frame {frame}: nothing shaped, no letter drawn");
    }
    // Two frames that show something else: the first of them still
    // keeps what the frame before it showed.
    e.start_frame();
    e.shape("x", px(20.0), None, None, 0.0, 0.0, 0.0, 0);
    assert_eq!(first.iter().filter(|id| e.big_glyph(**id).is_some()).count(), first.len());
    e.start_frame();
    assert!(kept_bytes(&e, &first) <= BIG_GLYPH_KEPT_BYTES, "no frame shapes them: dropped down to the kept bytes, {}", kept_bytes(&e, &first));
}

/// An emoji too big for the color atlas is a big glyph too: straight
/// RGBA of its own, whether swash painted it or the host. A new host
/// painter drops the big glyphs the old one painted.
#[test]
#[ignore = "needs Windows' Segoe UI Emoji font: run with --include-ignored on Windows"]
fn a_big_emoji_is_kept_by_itself_in_color() {
    let mut e: TextEngine = engine_with_emoji();
    let quads: Vec<GlyphQuad> = e.shape("a\u{1F600}", px(300.0), None, None, 0.0, 0.0, 0.0, 0);
    assert!(quads.len() == 2 && !quads[0].color && quads[1].color, "{quads:?}");
    assert_eq!(e.color_atlas().page_count(), 0, "nothing went into the color atlas");
    check_big_glyph(&e, &quads[0]);
    let emoji: BigGlyph = e.big_glyph(check_big_glyph(&e, &quads[1])).expect("kept");
    assert!(emoji.texels.chunks_exact(4).any(|texel| texel[3] >= 250 && (texel[0] != texel[1] || texel[1] != texel[2])), "the emoji's own colors");
    // Straight color: a half-covered yellow texel still has a strong
    // red; premultiplied, no color is above the alpha.
    assert!(emoji.texels.chunks_exact(4).any(|texel| (1..200).contains(&texel[3]) && texel[..3].iter().any(|color| *color > texel[3])), "straight color, not premultiplied");

    e.set_host_color_glyph_painter(Some(Box::new(|_: &str, _: f32| Some(HostColorGlyph { width: 300, height: 200, left: 0, top: 200, rgba: [0, 255, 0, 255].repeat(300 * 200) }))));
    let host: Vec<GlyphQuad> = e.shape("a\u{1F600}", px(40.0), None, None, 0.0, 0.0, 0.0, 0);
    assert!(host[0].big_glyph.is_none(), "the letter of 40 px is in the atlas");
    assert_eq!((host[1].w, host[1].h, host[1].color), (300.0, 200.0, true));
    let id: BigGlyphId = check_big_glyph(&e, &host[1]);
    let painted: BigGlyph = e.big_glyph(id).expect("kept");
    assert_eq!((big_texel(&painted, 1, 1), big_texel(&painted, 300, 200)), (&[0u8, 255, 0, 255][..], &[0u8, 255, 0, 255][..]), "the host's pixels, inside the border");
    assert_eq!(e.color_atlas().page_count(), 0, "nothing went into the color atlas");

    e.set_host_color_glyph_painter(None);
    assert!(e.big_glyph(id).is_none(), "the old painter's big glyph went with it");
}
