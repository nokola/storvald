//! The text engine on real fonts: it measures, wraps, shapes and
//! rasterizes Roboto, takes more faces by key, scans the OS fonts, and
//! puts emoji in the color atlas.
//!
//! Needed as a file of its own, outside `src`, because every test here
//! reads the Roboto files in `assets/Fonts` at the top of the repository,
//! and a crates.io package does not carry them. `Cargo.toml` leaves this
//! file out of the package (`exclude`), so `cargo test` on a downloaded
//! package runs only what the package can feed.

use gogo_text::{AtlasPart, AtlasSent, EpxMeasure, FaceMetrics, GlyphQuad, HostColorGlyph, TextEngine, TextPx, styled_key};
use gogo_ui::layout::TextMeasure;
use std::cell::RefCell;
use std::path::Path;

/// A run whose glyph grows the atlas mid-way comes out whole against
/// the grown atlas: its uvs equal a re-shape's. Red when the glyphs
/// shaped before the grow keep uvs into the atlas that was and the run
/// sits in the cache that way: the user sees a label without its first
/// letters ("ve changes?" for "Save changes?").
#[test]
fn a_run_that_grows_the_atlas_is_shaped_whole_against_the_grown_one() {
    let mut e = engine();
    let before: u32 = e.generation;
    // 400-px glyphs: nine fill the 1024 atlas, so the alphabet grows it
    // inside this one run.
    let text = "ABCDEFGHIJKLMNOPQRSTUVWXYZ";
    let shaped: Vec<GlyphQuad> = e.shape(text, px(400.0), None, None, 0.0, 0.0, 100_000.0, 0);
    assert!(e.generation > before, "the precondition: the run grew the atlas");
    // Shaped again past the cache: the box width is part of the cache
    // key and moves no glyph of a start-aligned line.
    let again: Vec<GlyphQuad> = e.shape(text, px(400.0), None, None, 0.0, 0.0, 100_001.0, 0);
    assert_eq!(shaped.len(), again.len());
    for (a, b) in shaped.iter().zip(&again) {
        assert_eq!(a.uv, b.uv, "a glyph of the run that grew the atlas, shaped whole: {a:?} vs {b:?}");
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
    let (atlas, side) = e.color_atlas();
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
    assert!(e.color_atlas_part_to_send(AtlasSent::default()).0.is_empty(), "nothing packed yet");
    let (_, letters_before): (AtlasPart, AtlasSent) = e.atlas_part_to_send(AtlasSent::default());
    let quads: Vec<GlyphQuad> = e.shape("a😀", px(40.0), None, None, 0.0, 0.0, 0.0, 0);
    assert_eq!(quads.len(), 2, "{quads:?}");
    assert!(!quads[0].color, "the letter: coverage atlas");
    assert!(quads[1].color, "the emoji: color atlas");
    assert!(quads[1].w > 20.0 && quads[1].h > 20.0, "an emoji at 40 px is a real box: {:?}", quads[1]);
    assert!(!e.atlas_part_to_send(letters_before).0.is_empty() && !e.color_atlas_part_to_send(AtlasSent::default()).0.is_empty());
    // Somewhere in the emoji's slot a texel is opaque and colored.
    let (atlas, side) = e.color_atlas();
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

/// A host raster that fills the color atlas grows it, and the run is
/// shaped whole against the grown one (the coverage atlas's law,
/// `a_run_that_grows_the_atlas_is_shaped_whole_against_the_grown_one`).
#[test]
#[ignore = "needs Windows' Segoe UI Emoji font: run with --include-ignored on Windows"]
fn a_host_emoji_that_fills_the_color_atlas_grows_it() {
    let mut e: TextEngine = engine_with_emoji();
    e.set_host_color_glyph_painter(Some(Box::new(|_: &str, _: f32| Some(HostColorGlyph { width: 300, height: 300, left: 0, top: 300, rgba: [255, 0, 0, 255].repeat(300 * 300) }))));
    let before: u32 = e.generation;
    // Four 300-px squares do not fit a 512 atlas: it doubles mid-run.
    let quads: Vec<GlyphQuad> = e.shape("😀😁😂🤣", px(40.0), None, None, 0.0, 0.0, 0.0, 0);
    assert!(e.generation > before, "the precondition: the run grew the color atlas");
    assert_eq!(e.color_atlas().1, 1024);
    assert_eq!(quads.len(), 4, "{quads:?}");
    for q in &quads {
        assert!(q.color);
        assert_eq!(color_texel_of(&e, q), [255, 0, 0, 255], "every slot is into the grown atlas: {q:?}");
    }
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
    let (_, before): (AtlasPart, AtlasSent) = e.atlas_part_to_send(AtlasSent::default());
    let quads = e.shape("Hi", px(20.0), None, None, 10.0, 5.0, 0.0, 0);
    assert_eq!(quads.len(), 2);
    assert!(quads[0].x >= 10.0 && quads[1].x > quads[0].x);
    let (atlas, size) = e.atlas();
    assert!(atlas.iter().any(|b| *b > 128), "atlas has ink");
    assert_eq!(size, 1024);
    // The atlas has a part to send when glyphs are packed into it, none
    // when a text's glyphs are in it already, and the whole of it for a
    // renderer that sent another engine's atlas.
    let (part, packed): (AtlasPart, AtlasSent) = e.atlas_part_to_send(before);
    assert!(!part.is_empty(), "two glyphs were packed");
    e.shape("Hi", px(20.0), None, None, 40.0, 5.0, 0.0, 0);
    assert!(e.atlas_part_to_send(packed).0.is_empty(), "nothing new was packed");
    let mut other = engine();
    assert_eq!(other.atlas_part_to_send(before).0, whole(1024), "a second engine's blank atlas");
    other.shape("Hi", px(20.0), None, None, 10.0, 5.0, 0.0, 0);
    assert_eq!(other.atlas_part_to_send(packed).0, whole(1024), "a second engine with the same two glyphs");
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
    let counts = |work: gogo_text::TextWork| (work.shapes_to_measure, work.shapes_to_draw, work.letters_drawn, work.texture_grows);
    let mut e = engine();
    assert_eq!(e.take_work(), gogo_text::TextWork::default(), "a new engine did nothing");
    e.measure("Hello", px(24.0), None, None);
    let work = e.take_work();
    assert_eq!(counts(work), (1, 0, 0, 0), "a measure shapes the text and draws no letter");
    assert!(work.seconds_shaping_to_measure > 0.0);
    e.shape("Hello", px(24.0), None, None, 0.0, 0.0, 0.0, 0);
    let work = e.take_work();
    assert_eq!((work.shapes_to_measure, work.shapes_to_draw, work.texture_grows), (0, 1, 0), "the draw shapes the same text once more");
    assert!((4..=5).contains(&work.letters_drawn), "H, e, l, o, and the second l when it sits at another part of a pixel: {}", work.letters_drawn);
    e.measure("Hello", px(24.0), None, None);
    e.shape("Hello", px(24.0), None, None, 0.0, 0.0, 0.0, 0);
    assert_eq!(e.take_work(), gogo_text::TextWork::default(), "a text seen before costs two lookups");
    e.narrowest_wrap("Hello there", px(24.0), None);
    assert_eq!(counts(e.take_work()), (1, 0, 0, 0), "the narrowest width of a wrapping text is a shape of its own");
    // 26 letters of 400 px do not fit the letters' texture: it doubles
    // and keeps the letters drawn. The text is shaped again against the
    // grown one, and each letter is drawn once.
    e.shape("ABCDEFGHIJKLMNOPQRSTUVWXYZ", px(400.0), None, None, 0.0, 0.0, 100_000.0, 0);
    let work = e.take_work();
    assert_eq!((work.texture_grows, work.shapes_to_draw, work.letters_drawn), (1, 2, 26), "{work:?}");
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

/// The whole of an atlas `size` texels a side.
fn whole(size: u32) -> AtlasPart {
    AtlasPart { columns: 0..size, rows: 0..size }
}

/// What a renderer's texture holds of an atlas: it starts empty and
/// takes only the part the engine names.
///
/// Needed by the tests of `atlas_part_to_send`: the part is right when
/// such a copy equals the engine's atlas after every take.
#[derive(Clone, Default)]
struct AtlasCopy {
    sent: AtlasSent,
    bytes: Vec<u8>,
    size: u32,
}

impl AtlasCopy {
    /// Copies `part` of `atlas` (`size` a side, `texel_bytes` a texel)
    /// and keeps `sent`. An atlas of another size starts a new, empty
    /// copy, as a renderer makes a new texture, and must be sent whole.
    fn take(&mut self, atlas: &[u8], size: u32, texel_bytes: u32, part: &AtlasPart, sent: AtlasSent) {
        self.sent = sent;
        if size != self.size {
            assert_eq!(*part, whole(size), "an atlas of another size is sent whole");
            self.bytes = vec![0; (size * size * texel_bytes) as usize];
            self.size = size;
        }
        for row in part.rows.clone() {
            let (from, to) = (((row * size + part.columns.start) * texel_bytes) as usize, ((row * size + part.columns.end) * texel_bytes) as usize);
            self.bytes[from..to].copy_from_slice(&atlas[from..to]);
        }
    }

    /// Takes the part of `e`'s letters' atlas this copy lacks.
    ///
    /// Returns the part the engine named.
    fn take_letters(&mut self, e: &TextEngine) -> AtlasPart {
        let (part, sent) = e.atlas_part_to_send(self.sent);
        let (atlas, size) = e.atlas();
        self.take(atlas, size, 1, &part, sent);
        part
    }

    /// `take_letters`, for the emoji atlas.
    fn take_emoji(&mut self, e: &TextEngine) -> AtlasPart {
        let (part, sent) = e.color_atlas_part_to_send(self.sent);
        let (atlas, size) = e.color_atlas();
        self.take(atlas, size, 4, &part, sent);
        part
    }
}

/// A renderer that sends only the part `atlas_part_to_send` names holds
/// the engine's whole atlas after each send: nothing for nothing new, a
/// new letter's own columns of its shelf for a new letter, whole rows
/// when letters start new shelves, the whole atlas for one it has not
/// got. Red when a texel that changed is not named: the user sees a new
/// letter as an empty box, or cut off.
#[test]
fn a_copy_that_takes_only_the_part_to_send_holds_the_whole_atlas() {
    let mut e = engine();
    let mut copy = AtlasCopy::default();
    let shape = |e: &mut TextEngine, text: &str, font_size: f32| {
        e.shape(text, px(font_size), None, None, 0.0, 0.0, 100_000.0, 0);
    };
    shape(&mut e, "Hi", 20.0);
    assert_eq!(copy.take_letters(&e), whole(1024), "nothing was sent yet: the whole atlas");
    assert!(copy.bytes == e.atlas().0);

    shape(&mut e, "Hi", 20.0);
    assert!(copy.take_letters(&e).is_empty(), "nothing new was packed");

    shape(&mut e, "o", 20.0);
    let beside: AtlasPart = copy.take_letters(&e);
    assert!(beside.rows.start == 0 && beside.rows.end < 64, "a new letter beside the first two: rows of their shelf, {beside:?}");
    assert!(beside.columns.start > 0 && beside.columns.end - beside.columns.start < 32, "and its own columns only, {beside:?}");
    assert!(copy.bytes == e.atlas().0, "the copy after one new letter");

    // A taller letter in the same shelf makes that shelf taller.
    shape(&mut e, "Q", 48.0);
    let taller: AtlasPart = copy.take_letters(&e);
    assert!(taller.columns.start >= beside.columns.end && taller.rows.start == 0 && taller.rows.end > beside.rows.end, "a taller letter in the same shelf: its columns, down to its foot, {taller:?}");
    assert!(copy.bytes == e.atlas().0, "the copy after a taller letter");

    // 52 letters at 64 px are wider than the atlas: they fill the first
    // shelf and start others below it.
    shape(&mut e, "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz", 64.0);
    let over_shelves: AtlasPart = copy.take_letters(&e);
    assert!(over_shelves.columns == (0..1024) && over_shelves.rows.start == 0, "letters over several shelves: whole rows from the shelf that was being filled, {over_shelves:?}");
    assert!(copy.bytes == e.atlas().0, "the copy after letters over several shelves");

    shape(&mut e, "?", 64.0);
    let lower: AtlasPart = copy.take_letters(&e);
    assert!(lower.rows.start > 0 && lower.columns.start > 0 && lower.texel_count() < 128 * 128, "a letter in a shelf below the first: no row of the first shelf and no column sent before, {lower:?}");
    assert!(copy.bytes == e.atlas().0, "the copy after a letter in a lower shelf");

    let mut second_renderer = AtlasCopy::default();
    assert_eq!(second_renderer.take_letters(&e), whole(1024), "a second renderer over the same engine: the whole atlas");
    assert!(second_renderer.bytes == e.atlas().0);

    // Another engine's atlas has the same size and other letters in the
    // same places.
    let mut other = engine();
    shape(&mut other, "xyz", 20.0);
    let mut handed_another_engine: AtlasCopy = copy.clone();
    assert_eq!(handed_another_engine.take_letters(&other), whole(1024), "another engine's atlas: the whole of it");
    assert!(handed_another_engine.bytes == other.atlas().0);

    let generation: u32 = e.generation;
    shape(&mut e, "ABCDEFGHIJKLMNOPQRSTUVWXYZ", 400.0);
    assert!(e.generation > generation, "the precondition: the atlas grew");
    let size: u32 = e.atlas().1;
    assert_eq!(copy.take_letters(&e), whole(size), "the grown atlas: the whole of it");
    assert!(copy.bytes == e.atlas().0);

    shape(&mut e, "#", 20.0);
    let in_grown: AtlasPart = copy.take_letters(&e);
    assert!(!in_grown.is_empty() && in_grown != whole(size), "a new letter in the grown atlas: not the whole of it, {in_grown:?}");
    assert!(copy.bytes == e.atlas().0, "the copy after a letter in the grown atlas");
}

/// The emoji atlas is sent the same way: nothing before the first
/// emoji, the whole of it at the first, a new emoji's own columns after.
#[test]
#[ignore = "needs Windows' Segoe UI Emoji font: run with --include-ignored on Windows"]
fn a_copy_that_takes_only_the_part_to_send_holds_the_whole_emoji_atlas() {
    let mut e: TextEngine = engine_with_emoji();
    let mut copy = AtlasCopy::default();
    e.shape("a", px(40.0), None, None, 0.0, 0.0, 0.0, 0);
    assert!(e.color_atlas_part_to_send(copy.sent).0.is_empty(), "no emoji was packed: nothing to send");

    e.shape("\u{1F600}", px(40.0), None, None, 0.0, 0.0, 0.0, 0);
    let size: u32 = e.color_atlas().1;
    assert_eq!(copy.take_emoji(&e), whole(size), "the first emoji: the whole atlas");
    assert!(copy.bytes == e.color_atlas().0);

    e.shape("\u{1F388}", px(40.0), None, None, 0.0, 0.0, 0.0, 0);
    let beside: AtlasPart = copy.take_emoji(&e);
    assert!(beside.rows.start == 0 && beside.rows.end < 128 && beside.columns.start > 0, "a second emoji beside the first: its own columns of their shelf, {beside:?}");
    assert!(copy.bytes == e.color_atlas().0, "the copy after a second emoji");
}

/// The texels of `q`'s slot in its atlas (the color atlas for an emoji),
/// row by row.
fn texels_of(e: &TextEngine, q: &GlyphQuad) -> Vec<u8> {
    let ((atlas, side), texel_bytes) = if q.color { (e.color_atlas(), 4) } else { (e.atlas(), 1) };
    let (x, y) = ((q.uv[0] * side as f32).round() as usize, (q.uv[1] * side as f32).round() as usize);
    let (w, h) = (q.w as usize * texel_bytes, q.h as usize);
    (0..h).flat_map(|row| atlas[((y + row) * side as usize + x) * texel_bytes..][..w].iter().copied()).collect()
}

/// A text drawn before an atlas grew, asked for again after: kept
/// shaped with no letter drawn, at the same place on screen, each glyph
/// in the same texels of the grown atlas. Then the same text past the
/// kept shapes: shaped once, and still no letter drawn.
///
/// `grown` shapes what makes the atlas grow.
fn check_a_grow_keeps(e: &mut TextEngine, text: &str, font_size: f32, grown: impl FnOnce(&mut TextEngine)) {
    let before: Vec<GlyphQuad> = e.shape(text, px(font_size), None, None, 0.0, 0.0, 0.0, 0);
    let texels_before: Vec<Vec<u8>> = before.iter().map(|q| texels_of(e, q)).collect();
    assert!(!before.is_empty() && texels_before.iter().all(|texels| texels.iter().any(|byte| *byte > 128)), "the precondition: glyphs with ink");
    let is_color: bool = before[0].color;
    let texel_bytes: usize = if is_color { 4 } else { 1 };
    let atlas_of = |e: &TextEngine| -> (Vec<u8>, usize) {
        let (atlas, side) = if is_color { e.color_atlas() } else { e.atlas() };
        (atlas.to_vec(), side as usize)
    };
    let (atlas_before, side_before) = atlas_of(e);
    let generation: u32 = e.generation;
    e.take_work();
    grown(e);
    assert!(e.generation > generation, "the precondition: an atlas grew");
    let grows: u32 = e.take_work().texture_grows;
    // Every byte with ink before the grow is the same after it, in the
    // same row and column. (`grown` drew into what was empty of the
    // atlas that was, too.)
    let (atlas_after, side_after) = atlas_of(e);
    assert!(side_after > side_before, "the precondition: the atlas of the text's glyphs grew");
    let row_bytes: usize = side_before * texel_bytes;
    for row in 0..side_before {
        let (was, is) = (&atlas_before[row * row_bytes..][..row_bytes], &atlas_after[row * side_after * texel_bytes..][..row_bytes]);
        assert!(was.iter().zip(is).all(|(was, is)| *was == 0 || was == is), "row {row}: what was drawn before the grow is in the same place after it");
    }

    let after: Vec<GlyphQuad> = e.shape(text, px(font_size), None, None, 0.0, 0.0, 0.0, 0);
    assert_eq!(e.take_work(), gogo_text::TextWork::default(), "a text shaped before the grow is kept: nothing shaped, no letter drawn");
    assert_eq!(after.len(), before.len());
    for ((b, a), texels) in before.iter().zip(&after).zip(&texels_before) {
        assert_eq!((a.x, a.y, a.w, a.h, a.color), (b.x, b.y, b.w, b.h, b.color), "the glyph is where it was on screen");
        assert_eq!(a.uv, b.uv.map(|coordinate| coordinate * 0.5f32.powi(grows as i32)), "its uv is a half for each grow");
        assert!(texels_of(e, a) == *texels, "and the same texels are under it");
    }

    // The box width is part of the kept shapes' key and moves no glyph
    // of a start-aligned line.
    let past_the_kept_shapes: Vec<GlyphQuad> = e.shape(text, px(font_size), None, None, 0.0, 0.0, 7.0, 0);
    let work = e.take_work();
    assert_eq!((work.shapes_to_draw, work.letters_drawn), (1, 0), "shaped anew, its letters are found in the grown atlas: none is drawn again");
    for (a, b) in after.iter().zip(&past_the_kept_shapes) {
        assert_eq!(a.uv, b.uv);
    }
}

/// The letters' atlas grows and keeps what was drawn. Red when a grow
/// empties the atlas: every text on screen is shaped again and every
/// letter drawn again in the frame of the grow, 10 ms for 61 letters on
/// an old phone.
#[test]
fn a_grown_atlas_keeps_the_letters_drawn_and_the_texts_shaped() {
    let mut e = engine();
    // 52 letters of 32 px lie across the whole width of the atlas.
    check_a_grow_keeps(&mut e, "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz", 32.0, |e| {
        e.shape("ABCDEFGHIJKLMNOPQRSTUVWXYZ", px(400.0), None, None, 0.0, 0.0, 100_000.0, 0);
    });
}

/// The emoji atlas grows the same way.
#[test]
#[ignore = "needs Windows' Segoe UI Emoji font: run with --include-ignored on Windows"]
fn a_grown_color_atlas_keeps_the_emoji_drawn() {
    let mut e: TextEngine = engine_with_emoji();
    // Ten emoji of 40 px lie across the whole width of the atlas.
    check_a_grow_keeps(&mut e, "\u{1F600}\u{1F601}\u{1F602}\u{1F603}\u{1F604}\u{1F605}\u{1F606}\u{1F609}\u{1F60A}\u{1F60D}", 40.0, |e| {
        // Nine emoji of 200 px do not fit an atlas 512 a side.
        e.shape("\u{1F388}\u{1F34E}\u{1F680}\u{1F308}\u{1F381}\u{1F436}\u{1F33B}\u{1F3C0}\u{1F525}", px(200.0), None, None, 0.0, 0.0, 100_000.0, 0);
    });
}

/// A grow of the letters' atlas leaves the uv of a kept emoji alone:
/// the emoji is in the color atlas, which did not grow. Red when a grow
/// rewrites every kept quad: the user sees a piece of another emoji, at
/// half its size.
#[test]
#[ignore = "needs Windows' Segoe UI Emoji font: run with --include-ignored on Windows"]
fn a_grow_of_the_letters_atlas_leaves_a_kept_emoji_alone() {
    let mut e: TextEngine = engine_with_emoji();
    let before: Vec<GlyphQuad> = e.shape("a\u{1F600}", px(40.0), None, None, 0.0, 0.0, 0.0, 0);
    assert!(before.len() == 2 && !before[0].color && before[1].color, "the precondition: a letter and an emoji, {before:?}");
    let color_side: u32 = e.color_atlas().1;
    e.take_work();
    e.shape("ABCDEFGHIJKLMNOPQRSTUVWXYZ", px(400.0), None, None, 0.0, 0.0, 100_000.0, 0);
    assert_eq!(e.take_work().texture_grows, 1, "the precondition: one grow");
    assert_eq!(e.color_atlas().1, color_side, "the precondition: the letters' atlas grew, not the color atlas");
    let after: Vec<GlyphQuad> = e.shape("a\u{1F600}", px(40.0), None, None, 0.0, 0.0, 0.0, 0);
    assert_eq!(after[0].uv, before[0].uv.map(|coordinate| coordinate * 0.5), "the letter: a half");
    assert_eq!(after[1].uv, before[1].uv, "the emoji: as it was");
}

/// A copy stays whole while letters fill a grown atlas: its top right
/// quarter first, where a send is that quarter's columns only, then its
/// lower half. Red when a send in the quarter is whole rows (twice the
/// bytes: the quarter's shelves are half as wide), or when the send
/// that goes on to the lower half leaves out the atlas's left half.
#[test]
fn a_copy_stays_whole_while_letters_fill_a_grown_atlas() {
    let mut e = engine();
    let mut copy = AtlasCopy::default();
    let generation: u32 = e.generation;
    let (mut sends_in_the_quarter, mut sends_in_the_lower_half): (u32, u32) = (0, 0);
    // The alphabet at 45 sizes fills the atlas of 1024 a side more than
    // twice: it grows once, and the letters go on into the lower half.
    for font_size in (20..=64).rev() {
        e.shape("ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz", px(font_size as f32), None, None, 0.0, 0.0, 100_000.0, 0);
        let part: AtlasPart = copy.take_letters(&e);
        assert!(copy.bytes == e.atlas().0, "the copy after the letters of {font_size} px, sent {part:?}");
        if e.generation == generation || part == whole(2048) {
            continue;
        }
        if sends_in_the_lower_half == 0 && part.columns.start >= 1024 {
            sends_in_the_quarter += 1;
            assert!(part.rows.end <= 1024, "a send in the top right quarter: {part:?}");
        } else {
            sends_in_the_lower_half += 1;
            assert!(part.rows.end > 1024, "a send after the top right quarter is full: {part:?}");
        }
    }
    assert_eq!(e.generation, generation + 1, "the precondition: one grow");
    assert!(sends_in_the_quarter >= 2 && sends_in_the_lower_half >= 2, "the precondition: letters in both: {sends_in_the_quarter} sends in the quarter, {sends_in_the_lower_half} in the lower half");
}
