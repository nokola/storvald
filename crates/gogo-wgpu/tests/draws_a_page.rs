//! End to end on a real GPU: a `.gogo` page compiled, laid out with
//! real text measures, drawn into a texture by `UiRenderer`, read back.
//! One test draws a plate and its text. Another uploads pictures the
//! way a host does, several times over, and reads them back. A third
//! draws the wedge of a dial and checks it against the hit test. Two
//! more draw one text engine's letters with a second renderer, and a
//! second engine's letters with the same renderer. A sixth draws letters
//! new to the atlas and counts the texels of it the renderer sent. Three
//! more draw letters too big for the atlas, each from a texture of its
//! own.
//!
//! Needed because nothing else in this repository runs the shader: the
//! unit tests cover the gradient tables only, and a renderer whose quads
//! are right but whose pipeline draws nothing would pass them all.
//!
//! Ignored unless asked for: all of them need a GPU adapter, which a hosted
//! test machine may not have, and a test that returned early there would
//! count as passed without having run. On a machine with a GPU:
//!
//!     cargo test -p gogo-wgpu -- --include-ignored

use std::cell::RefCell;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use gogo_text::{BIG_GLYPH_KEPT_BYTES, BigGlyph, EpxMeasure, GlyphQuad, TextEngine, TextPx};
use gogo_ui::layout::Rect;
use gogo_ui::runtime::{Runtime, resolve};
use gogo_ui::{Compiled, Draw, ImageKey, KeptFrame, Page, compile, draw, has_errors};
use gogo_wgpu::{PrepareStage, PrepareTimings, Tex_Tile_Premul_Srgb_4xU8_RGBA, UiRenderer};

const PAGE: &str = r#"
View card {
    Panel plate width=120, height=60, halign=left, valign=top, background=#E51400 {
        Label hello text="Hi", font_size=32, font_color=#FFFFFF, halign=center, valign=center
    }
}
"#;

const TILE: (u32, u32) = (160, 100);
const FONT_KEY: &str = "assets/Fonts/Roboto Regular.ttf";

/// A device on any backend built in (`WGPU_BACKEND` picks one, as in
/// wgpu's own tools).
fn device() -> (wgpu::Device, wgpu::Queue) {
    let mut desc = wgpu::InstanceDescriptor::new_without_display_handle();
    if let Some(backends) = wgpu::Backends::from_env() {
        desc.backends = backends;
    }
    let instance = wgpu::Instance::new(desc);
    let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions::default())).expect("this test needs a GPU adapter");
    pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor::default())).expect("a device of that adapter")
}

/// The tile's pixels, RGBA, row by row.
fn read_back(device: &wgpu::Device, queue: &wgpu::Queue, tile: &Tex_Tile_Premul_Srgb_4xU8_RGBA) -> Vec<u8> {
    let (w, h) = (tile.width(), tile.height());
    let padded: u32 = (w * 4).div_ceil(wgpu::COPY_BYTES_PER_ROW_ALIGNMENT) * wgpu::COPY_BYTES_PER_ROW_ALIGNMENT;
    let buffer = device.create_buffer(&wgpu::BufferDescriptor { label: Some("tile readback"), size: (padded * h) as u64, usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ, mapped_at_creation: false });
    let mut encoder = device.create_command_encoder(&Default::default());
    encoder.copy_texture_to_buffer(
        wgpu::TexelCopyTextureInfo { texture: &tile.RGBA, mip_level: 0, origin: wgpu::Origin3d::ZERO, aspect: wgpu::TextureAspect::All },
        wgpu::TexelCopyBufferInfo { buffer: &buffer, layout: wgpu::TexelCopyBufferLayout { offset: 0, bytes_per_row: Some(padded), rows_per_image: Some(h) } },
        wgpu::Extent3d { width: w, height: h, depth_or_array_layers: 1 },
    );
    queue.submit([encoder.finish()]);
    let slice = buffer.slice(..);
    slice.map_async(wgpu::MapMode::Read, |_| {});
    device.poll(wgpu::PollType::Wait { submission_index: None, timeout: Some(Duration::from_secs(10)) }).expect("the GPU answers within ten seconds");
    let mut pixels: Vec<u8> = Vec::with_capacity((w * h * 4) as usize);
    {
        let data = slice.get_mapped_range().expect("the read is mapped");
        for y in 0..h as usize {
            let start: usize = y * padded as usize;
            pixels.extend_from_slice(&data[start..start + (w * 4) as usize]);
        }
    }
    buffer.unmap();
    pixels
}

/// Writes down what a host's timings are told, to pin the order of the
/// calls.
#[derive(Default)]
struct Told {
    log: Arc<Mutex<Vec<String>>>,
}

impl PrepareTimings for Told {
    fn stage_begins(&mut self, stage: PrepareStage) {
        self.log.lock().expect("the log").push(format!("begin {stage:?}"));
    }

    fn stage_ends(&mut self, stage: PrepareStage) {
        self.log.lock().expect("the log").push(format!("end {stage:?}"));
    }

    fn text_shaped(&mut self, seconds: f32) {
        assert!(seconds >= 0.0);
        self.log.lock().expect("the log").push("text".to_string());
    }
}

#[test]
#[ignore = "needs a GPU adapter: run with --include-ignored on a machine that has one"]
fn a_page_drawn_to_a_texture_shows_its_plate_and_its_text() {
    let compiled: Compiled = compile(PAGE, &|_| None);
    assert!(!has_errors(&compiled.diags), "{:?}", compiled.diags);
    let page: Page = compiled.page;

    let font: Vec<u8> = std::fs::read(concat!(env!("CARGO_MANIFEST_DIR"), "/../../assets/Fonts/Roboto Regular.ttf")).expect("the font");
    let text: RefCell<TextEngine> = RefCell::new(TextEngine::new(&[(FONT_KEY, font)]));
    let runtime: Runtime = Runtime::new(&page);
    let screen: Rect = Rect::new(0.0, 0.0, TILE.0 as f32, TILE.1 as f32);
    let measure = EpxMeasure { engine: &text, epx: 1.0, font: None, images: &[] };
    let kept: KeptFrame = KeptFrame::fresh(&page, resolve(&page, &runtime), screen, &measure);
    let plate: Rect = kept.frame().rects()[page.find("plate").expect("the plate").index()];
    let label: Rect = kept.frame().rects()[page.find("hello").expect("the label").index()];
    assert_eq!((plate.x, plate.y, plate.w, plate.h), (0.0, 0.0, 120.0, 60.0));
    let draws: Vec<Draw> = draw(&page, kept.frame());

    let (device, queue) = device();
    let mut ui: UiRenderer = UiRenderer::new(&device, wgpu::TextureFormat::Rgba8Unorm, None);
    // A renderer can be built on one thread and used on another, with
    // timings installed or without.
    fn crosses_threads<T: Send + Sync>(_: &T) {}
    crosses_threads(&ui);
    let told = Told::default();
    let log = told.log.clone();
    ui.set_prepare_timings(Some(Box::new(told)));
    let tile: Tex_Tile_Premul_Srgb_4xU8_RGBA = ui.render_to_texture(&device, &queue, &draws, &page, &mut text.borrow_mut(), None, 1.0, (0.0, 0.0), TILE);
    assert_eq!((tile.width(), tile.height()), TILE);
    let pixels: Vec<u8> = read_back(&device, &queue, &tile);
    let at = |x: u32, y: u32| -> [u8; 4] {
        let i: usize = ((y * TILE.0 + x) * 4) as usize;
        [pixels[i], pixels[i + 1], pixels[i + 2], pixels[i + 3]]
    };

    // The plate's own color, away from the letters.
    assert_eq!(at(4, 4), [0xE5, 0x14, 0x00, 0xFF], "the plate's corner");
    // Nothing outside the plate: the clear shows, transparent.
    assert_eq!(at(140, 80), [0, 0, 0, 0], "outside the plate");
    // The letters: white ink somewhere inside the label's rect.
    let (x0, y0, x1, y1) = (label.x as u32, label.y as u32, (label.x + label.w) as u32, (label.y + label.h) as u32);
    let white: usize = (y0..y1).flat_map(|y| (x0..x1).map(move |x| (x, y))).filter(|&(x, y)| at(x, y) == [0xFF, 0xFF, 0xFF, 0xFF]).count();
    assert!(white > 20, "the label's letters are drawn: {white} white pixels in {label:?}");

    // The timings heard every stage once, in order, with the text time
    // inside the walk.
    assert_eq!(log.lock().expect("the log").as_slice(), ["begin Pass", "text", "end Pass", "begin Atlas", "end Atlas", "begin Upload", "end Upload"]);
}

/// Two pictures side by side, each in a rect of 40 x 40 epx. An Image
/// letterboxes its picture unless it says otherwise.
const ALBUM: &str = r#"
View album {
    Row shelf halign=left, valign=top, spacing=20 {
        Image photo 40x40, source="res://photo.png"
        Image other 40x40, source="res://other.png"
    }
}
"#;

/// `width` x `height` pixels of one color, as straight sRGB RGBA rows.
fn solid(color: [u8; 4], width: u32, height: u32) -> Vec<u8> {
    color.repeat((width * height) as usize)
}

/// A picture uploaded again keeps its ONE texture slot: at the same size,
/// at a new size, and after a release. Each time the tile shows the new
/// pixels, placed by the new size, and the picture beside it keeps its
/// own.
///
/// Needed because a new size used to move the picture to a fresh slot and
/// leave the old texture in the list until the renderer was dropped: a
/// host that uploaded one picture at changing sizes held more GPU memory
/// with every change, and nothing showed it.
#[test]
#[ignore = "needs a GPU adapter: run with --include-ignored on a machine that has one"]
fn a_picture_uploaded_again_keeps_its_one_texture_slot() {
    const RED: [u8; 4] = [255, 0, 0, 255];
    const GREEN: [u8; 4] = [0, 255, 0, 255];
    const BLUE: [u8; 4] = [0, 0, 255, 255];
    const YELLOW: [u8; 4] = [255, 255, 0, 255];
    const CLEAR: [u8; 4] = [0, 0, 0, 0];

    let compiled: Compiled = compile(ALBUM, &|_| None);
    assert!(!has_errors(&compiled.diags), "{:?}", compiled.diags);
    let photo_key: ImageKey = compiled.images.get("res://photo.png").expect("the photo's key");
    let other_key: ImageKey = compiled.images.get("res://other.png").expect("the other picture's key");
    let page: Page = compiled.page;

    let font: Vec<u8> = std::fs::read(concat!(env!("CARGO_MANIFEST_DIR"), "/../../assets/Fonts/Roboto Regular.ttf")).expect("the font");
    let text: RefCell<TextEngine> = RefCell::new(TextEngine::new(&[(FONT_KEY, font)]));
    let runtime: Runtime = Runtime::new(&page);
    let screen: Rect = Rect::new(0.0, 0.0, TILE.0 as f32, TILE.1 as f32);
    let measure = EpxMeasure { engine: &text, epx: 1.0, font: None, images: &[] };
    let kept: KeptFrame = KeptFrame::fresh(&page, resolve(&page, &runtime), screen, &measure);
    let photo: Rect = kept.frame().rects()[page.find("photo").expect("the photo").index()];
    let other: Rect = kept.frame().rects()[page.find("other").expect("the other picture").index()];
    assert_eq!((photo.x, photo.y, photo.w, photo.h), (0.0, 0.0, 40.0, 40.0));
    assert_eq!((other.x, other.y, other.w, other.h), (60.0, 0.0, 40.0, 40.0));
    let draws: Vec<Draw> = draw(&page, kept.frame());

    let (device, queue) = device();
    let mut ui: UiRenderer = UiRenderer::new(&device, wgpu::TextureFormat::Rgba8Unorm, None);
    // What the tile shows in the middle and near the top edge of the
    // photo's rect, then of the other picture's rect.
    let shown = |ui: &mut UiRenderer| -> [[u8; 4]; 4] {
        let tile: Tex_Tile_Premul_Srgb_4xU8_RGBA = ui.render_to_texture(&device, &queue, &draws, &page, &mut text.borrow_mut(), None, 1.0, (0.0, 0.0), TILE);
        let pixels: Vec<u8> = read_back(&device, &queue, &tile);
        let at = |x: u32, y: u32| -> [u8; 4] {
            let i: usize = ((y * TILE.0 + x) * 4) as usize;
            [pixels[i], pixels[i + 1], pixels[i + 2], pixels[i + 3]]
        };
        [at(20, 20), at(20, 3), at(80, 20), at(80, 3)]
    };

    // First uploads: each picture gets a slot of its own.
    let before: usize = ui.debug_texture_slot_count();
    ui.write_image_texture_only(&device, &queue, photo_key, &solid(RED, 2, 2), 2, 2);
    ui.write_image_texture_only(&device, &queue, other_key, &solid(YELLOW, 2, 2), 2, 2);
    let slots: usize = ui.debug_texture_slot_count();
    assert_eq!(slots, before + 2, "one slot per picture");
    assert_eq!(shown(&mut ui), [RED, RED, YELLOW, YELLOW], "two squares, each filling its rect");

    // The same size again: written in place.
    ui.write_image_texture_only(&device, &queue, photo_key, &solid(BLUE, 2, 2), 2, 2);
    assert_eq!(ui.debug_texture_slot_count(), slots, "the same size makes no slot");
    assert_eq!(shown(&mut ui), [BLUE, BLUE, YELLOW, YELLOW], "the photo's new pixels");

    // A new size, twice as wide as tall: no new slot, and the photo is
    // letterboxed by its NEW shape, 40 x 20 in the middle of its rect.
    ui.write_image_texture_only(&device, &queue, photo_key, &solid(GREEN, 4, 2), 4, 2);
    assert_eq!(ui.debug_texture_slot_count(), slots, "a new size makes no slot either");
    assert_eq!(shown(&mut ui), [GREEN, CLEAR, YELLOW, YELLOW], "the photo by its new shape, the other picture untouched");

    // Released, the photo draws nothing; uploaded again, it takes the
    // emptied slot back.
    assert!(ui.release_image(&device, photo_key), "the photo's texture is freed");
    assert!(!ui.has_image(photo_key));
    assert_eq!(shown(&mut ui), [CLEAR, CLEAR, YELLOW, YELLOW], "a released picture draws nothing");
    ui.write_image_texture_only(&device, &queue, photo_key, &solid(RED, 2, 2), 2, 2);
    assert_eq!(ui.debug_texture_slot_count(), slots, "the emptied slot is taken again");
    assert_eq!(shown(&mut ui), [RED, RED, YELLOW, YELLOW], "the photo is back, a square again");
}

/// A dial 80 x 80 at the tile's top left. Its wedge opens from 0 for a
/// quarter turn, and at `value=1` it is fully open over a green square
/// that fills the dial.
const DIAL: &str = r#"
control Dial $value=0, $min=0, $max=1, $step=0, min_size=80x80 {
    state_builtin hovered, focused, disabled
    Layers body {
        slot track width=fill, height=fill {
            slot wedge automatic=arc(from=0, sweep=90) by $value in $min..$max {
                Rectangle green color=#00FF00, width=fill, height=fill
            }
        }
    }
}
View card {
    Dial d 80x80, value=1, halign=left, valign=top
}
"#;

/// The shader cuts a wedge where the hit test says it is: `from=0` is 12
/// o'clock and the sweep runs clockwise, so a quarter wedge from 0 shows
/// the upper right quarter of its box and nothing of the other three.
///
/// Needed because the wedge is worked out twice, in `gogo-ui` for
/// touches and in the shader for pixels, and only a drawn frame shows
/// that the two agree.
#[test]
#[ignore = "needs a GPU adapter: run with --include-ignored on a machine that has one"]
fn a_wedge_is_drawn_where_it_takes_touches() {
    const GREEN: [u8; 4] = [0, 255, 0, 255];
    const CLEAR: [u8; 4] = [0, 0, 0, 0];
    // The middle of each quarter of the dial: upper right, lower right,
    // lower left, upper left.
    const QUARTERS: [(u32, u32); 4] = [(60, 20), (60, 60), (20, 60), (20, 20)];

    let compiled: Compiled = compile(DIAL, &|_| None);
    assert!(!has_errors(&compiled.diags), "{:?}", compiled.diags);
    let page: Page = compiled.page;

    let font: Vec<u8> = std::fs::read(concat!(env!("CARGO_MANIFEST_DIR"), "/../../assets/Fonts/Roboto Regular.ttf")).expect("the font");
    let text: RefCell<TextEngine> = RefCell::new(TextEngine::new(&[(FONT_KEY, font)]));
    let runtime: Runtime = Runtime::new(&page);
    let screen: Rect = Rect::new(0.0, 0.0, TILE.0 as f32, TILE.1 as f32);
    let measure = EpxMeasure { engine: &text, epx: 1.0, font: None, images: &[] };
    let kept: KeptFrame = KeptFrame::fresh(&page, resolve(&page, &runtime), screen, &measure);
    let wedge = page.find("d/wedge").expect("the wedge");
    let dial: Rect = kept.frame().rects()[wedge.index()];
    assert_eq!((dial.x, dial.y, dial.w, dial.h), (0.0, 0.0, 80.0, 80.0));
    let draws: Vec<Draw> = draw(&page, kept.frame());

    let (device, queue) = device();
    let mut ui: UiRenderer = UiRenderer::new(&device, wgpu::TextureFormat::Rgba8Unorm, None);
    let tile: Tex_Tile_Premul_Srgb_4xU8_RGBA = ui.render_to_texture(&device, &queue, &draws, &page, &mut text.borrow_mut(), None, 1.0, (0.0, 0.0), TILE);
    let pixels: Vec<u8> = read_back(&device, &queue, &tile);
    let at = |x: u32, y: u32| -> [u8; 4] {
        let i: usize = ((y * TILE.0 + x) * 4) as usize;
        [pixels[i], pixels[i + 1], pixels[i + 2], pixels[i + 3]]
    };

    assert_eq!(QUARTERS.map(|(x, y)| at(x, y)), [GREEN, CLEAR, CLEAR, CLEAR], "only the upper right quarter is drawn");
    let touched: [bool; 4] = QUARTERS.map(|(x, y)| gogo_ui::hit(&page, kept.frame(), x as f32, y as f32, true).is_some_and(|n| kept.look().placed_is_under(&page, n, wedge)));
    assert_eq!(touched, [true, false, false, false], "and only that quarter takes a touch");
}

/// The plate of `PAGE` with its two letters the other way round: an
/// engine packs this text's glyphs into its atlas in the other order.
const PAGE_LETTERS_SWAPPED: &str = r#"
View card {
    Panel plate width=120, height=60, halign=left, valign=top, background=#E51400 {
        Label hello text="iH", font_size=32, font_color=#FFFFFF, halign=center, valign=center
    }
}
"#;

/// A text engine with the one font these tests draw with.
fn engine() -> RefCell<TextEngine> {
    let font: Vec<u8> = std::fs::read(concat!(env!("CARGO_MANIFEST_DIR"), "/../../assets/Fonts/Roboto Regular.ttf")).expect("the font");
    RefCell::new(TextEngine::new(&[(FONT_KEY, font)]))
}

/// `source` laid out on a tile and drawn by `ui` with the letters of
/// `text`.
///
/// Returns the tile's pixels and the rect of the Label named `hello`.
fn drawn_tile(source: &str, ui: &mut UiRenderer, device: &wgpu::Device, queue: &wgpu::Queue, text: &RefCell<TextEngine>) -> (Vec<u8>, Rect) {
    drawn_tile_of(source, TILE, ui, device, queue, text)
}

/// `drawn_tile`, on a tile of `tile` pixels.
fn drawn_tile_of(source: &str, tile: (u32, u32), ui: &mut UiRenderer, device: &wgpu::Device, queue: &wgpu::Queue, text: &RefCell<TextEngine>) -> (Vec<u8>, Rect) {
    let compiled: Compiled = compile(source, &|_| None);
    assert!(!has_errors(&compiled.diags), "{:?}", compiled.diags);
    let page: Page = compiled.page;
    let runtime: Runtime = Runtime::new(&page);
    let screen: Rect = Rect::new(0.0, 0.0, tile.0 as f32, tile.1 as f32);
    let measure = EpxMeasure { engine: text, epx: 1.0, font: None, images: &[] };
    let kept: KeptFrame = KeptFrame::fresh(&page, resolve(&page, &runtime), screen, &measure);
    let label: Rect = kept.frame().rects()[page.find("hello").expect("the label").index()];
    let draws: Vec<Draw> = draw(&page, kept.frame());
    let tile: Tex_Tile_Premul_Srgb_4xU8_RGBA = ui.render_to_texture(device, queue, &draws, &page, &mut text.borrow_mut(), None, 1.0, (0.0, 0.0), tile);
    (read_back(device, queue, &tile), label)
}

/// How many pixels of a tile inside `rect` are opaque white.
fn white_pixels(pixels: &[u8], rect: Rect) -> usize {
    let (x0, y0, x1, y1) = (rect.x as u32, rect.y as u32, (rect.x + rect.w) as u32, (rect.y + rect.h) as u32);
    let is_white = |x: u32, y: u32| -> bool {
        let i: usize = ((y * TILE.0 + x) * 4) as usize;
        pixels[i..i + 4] == [0xFF; 4]
    };
    (y0..y1).flat_map(|y| (x0..x1).map(move |x| (x, y))).filter(|&(x, y)| is_white(x, y)).count()
}

/// A renderer made after another has drawn with the same text engine
/// draws that engine's letters too.
///
/// Needed because one text engine may meet several renderers over its
/// life: one per picture a host takes, a new one after the GPU was lost.
/// Each must upload the glyph atlas by itself, or its picture has every
/// plate and no letters.
#[test]
#[ignore = "needs a GPU adapter: run with --include-ignored on a machine that has one"]
fn a_second_renderer_over_one_text_engine_draws_its_letters() {
    let text: RefCell<TextEngine> = engine();
    let (device, queue) = device();
    let mut first: UiRenderer = UiRenderer::new(&device, wgpu::TextureFormat::Rgba8Unorm, None);
    let (pixels, label) = drawn_tile(PAGE, &mut first, &device, &queue, &text);
    let first_white: usize = white_pixels(&pixels, label);
    assert!(first_white > 20, "the precondition: the first renderer draws the letters: {first_white} white pixels");

    let mut second: UiRenderer = UiRenderer::new(&device, wgpu::TextureFormat::Rgba8Unorm, None);
    let (pixels, label) = drawn_tile(PAGE, &mut second, &device, &queue, &text);
    assert_eq!(white_pixels(&pixels, label), first_white, "the second renderer draws the same letters");
}

/// A renderer handed another text engine draws that engine's letters.
///
/// Needed because a renderer keeps the version of the atlas it uploaded
/// (`TextEngine::atlas_part_to_send`). Two engines that each packed two glyphs
/// would share a count kept per engine: the renderer would keep the first
/// engine's atlas and draw the second engine's letters from the wrong
/// places in it.
#[test]
#[ignore = "needs a GPU adapter: run with --include-ignored on a machine that has one"]
fn a_renderer_handed_another_text_engine_draws_that_engines_letters() {
    let (device, queue) = device();
    let mut ui: UiRenderer = UiRenderer::new(&device, wgpu::TextureFormat::Rgba8Unorm, None);
    let first_engine: RefCell<TextEngine> = engine();
    drawn_tile(PAGE, &mut ui, &device, &queue, &first_engine);

    let second_engine: RefCell<TextEngine> = engine();
    let (pixels, label) = drawn_tile(PAGE_LETTERS_SWAPPED, &mut ui, &device, &queue, &second_engine);
    assert!(white_pixels(&pixels, label) > 20, "the letters are drawn");
    let mut fresh: UiRenderer = UiRenderer::new(&device, wgpu::TextureFormat::Rgba8Unorm, None);
    let (want, _) = drawn_tile(PAGE_LETTERS_SWAPPED, &mut fresh, &device, &queue, &second_engine);
    assert!(pixels == want, "the picture of a renderer that only ever saw the second engine");
}

const PAGE_OTHER_LETTERS: &str = r#"
View card {
    Panel plate width=120, height=60, halign=left, valign=top, background=#E51400 {
        Label hello text="ok", font_size=32, font_color=#FFFFFF, halign=center, valign=center
    }
}
"#;

/// A renderer that drew one text and then draws letters new to the
/// atlas sends those letters' rectangle only, and draws them as a
/// renderer that sent the whole atlas does.
///
/// Needed because a frame with one new letter otherwise sends the whole
/// atlas to the GPU, megabytes for a few hundred texels. Red on the
/// texel count when every send is the whole atlas; red on the picture
/// when the rectangle sent is not the one that changed.
#[test]
#[ignore = "needs a GPU adapter: run with --include-ignored on a machine that has one"]
fn a_new_letter_sends_only_its_part_of_the_atlas() {
    let (device, queue) = device();
    let mut ui: UiRenderer = UiRenderer::new(&device, wgpu::TextureFormat::Rgba8Unorm, None);
    let text: RefCell<TextEngine> = engine();
    // 52 letters at 64 px are wider than the atlas: they fill its first
    // shelf, so the letters drawn below land under it and right of its
    // left edge. Needed because a rectangle written at a wrong place,
    // or read from a wrong place, is still right at the atlas's top
    // left corner.
    text.borrow_mut().warm("ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz", 64.0, None);
    drawn_tile(PAGE, &mut ui, &device, &queue, &text);
    let atlas_side: u64 = u64::from(text.borrow().atlas().1);
    let atlas_texels: u64 = atlas_side * atlas_side;
    assert_eq!(ui.debug_atlas_texels_written(), atlas_texels, "the first frame sends the whole atlas");

    let (pixels, label) = drawn_tile(PAGE_OTHER_LETTERS, &mut ui, &device, &queue, &text);
    let texels_of_new_letters: u64 = ui.debug_atlas_texels_written() - atlas_texels;
    assert!(texels_of_new_letters > 0 && texels_of_new_letters < 100 * 100, "two new letters at 32 px: {texels_of_new_letters} texels sent");
    assert!(white_pixels(&pixels, label) > 20, "the new letters are drawn");
    let new_letter: GlyphQuad = text.borrow_mut().glyph("o", 32.0, None).expect("the letter drawn above");
    assert!(new_letter.uv[0] > 0.0 && new_letter.uv[1] > 0.0, "the precondition: the new letters are below the atlas's first shelf and right of its left edge");
    let mut fresh: UiRenderer = UiRenderer::new(&device, wgpu::TextureFormat::Rgba8Unorm, None);
    let (want, _) = drawn_tile(PAGE_OTHER_LETTERS, &mut fresh, &device, &queue, &text);
    assert!(pixels == want, "the picture of a renderer that sent the whole atlas");

    drawn_tile(PAGE_OTHER_LETTERS, &mut ui, &device, &queue, &text);
    assert_eq!(ui.debug_atlas_texels_written(), atlas_texels + texels_of_new_letters, "a frame with no new letter sends nothing");
}

/// The tile the big letters are drawn on.
const BIG_TILE: (u32, u32) = (256, 256);

/// One letter of 200 px: its raster is wider and taller than
/// `gogo_text::ATLAS_GLYPH_MAX_PX`, so it is a big glyph.
const PAGE_BIG_LETTER: &str = r#"
View card {
    Label hello text="W", font_size=200, font_color=#FFFFFF, halign=left, valign=top
}
"#;

/// A letter too big for the atlas is drawn from a texture of its own,
/// texel for texel where the text engine's raster has it; the renderer
/// sends that raster once, and a second renderer and a second engine
/// each get the letter too.
///
/// Needed because a big glyph is in no atlas (`gogo_text::BigGlyph`).
/// Red when the renderer draws its quad from the atlas: the user sees
/// nothing, or a piece of other letters, where the big letter is. Red
/// when the uv or the texture is off by a texel: the letter is cut at
/// one edge and has a line of nothing at the other.
#[test]
#[ignore = "needs a GPU adapter: run with --include-ignored on a machine that has one"]
fn a_letter_too_big_for_the_atlas_is_drawn_from_a_texture_of_its_own() {
    let (device, queue) = device();
    let mut ui: UiRenderer = UiRenderer::new(&device, wgpu::TextureFormat::Rgba8Unorm, None);
    let text: RefCell<TextEngine> = engine();
    let (pixels, label) = drawn_tile_of(PAGE_BIG_LETTER, BIG_TILE, &mut ui, &device, &queue, &text);
    let texels_sent: u64 = ui.debug_atlas_texels_written();
    {
        // Where the renderer put the letter: the label's corner.
        let quad: GlyphQuad = text.borrow_mut().shape("W", TextPx::from_epx(200.0, 1.0), None, None, label.x, label.y, label.w, 0)[0];
        let engine = text.borrow();
        let glyph: BigGlyph = engine.big_glyph(quad.big_glyph.expect("the precondition: a big glyph")).expect("the engine keeps it");
        assert_eq!(texels_sent, 1024 * 1024 + u64::from(glyph.width * glyph.height), "the first frame sends the atlas, with no letter in it, and the big letter's raster");
        // The texel one right and one down of the raster's corner is
        // the quad's first pixel: the raster has an empty border.
        let (mut covered, mut empty): (u32, u32) = (0, 0);
        for y in 0..BIG_TILE.1 {
            for x in 0..BIG_TILE.0 {
                let (column, row): (i32, i32) = (x as i32 - quad.x as i32 + 1, y as i32 - quad.y as i32 + 1);
                let is_in_raster: bool = (0..glyph.width as i32).contains(&column) && (0..glyph.height as i32).contains(&row);
                let coverage: u8 = if is_in_raster { glyph.texels[(row as u32 * glyph.width + column as u32) as usize] } else { 0 };
                let pixel: &[u8] = &pixels[((y * BIG_TILE.0 + x) * 4) as usize..][..4];
                match coverage {
                    0 => {
                        assert_eq!(pixel, [0; 4], "nothing at {x},{y}");
                        empty += 1;
                    }
                    255 => {
                        assert_eq!(pixel, [255; 4], "the letter's white at {x},{y}");
                        covered += 1;
                    }
                    _ => assert!(pixel[3] > 0, "the letter's edge at {x},{y}: coverage {coverage}"),
                }
            }
        }
        assert!(covered > 4000 && empty > 20_000, "the precondition: a letter with an inside and an outside, {covered} and {empty} pixels");
    }

    let (again, _) = drawn_tile_of(PAGE_BIG_LETTER, BIG_TILE, &mut ui, &device, &queue, &text);
    assert!(again == pixels);
    assert_eq!(ui.debug_atlas_texels_written(), texels_sent, "a frame with no new letter sends nothing");

    let mut second: UiRenderer = UiRenderer::new(&device, wgpu::TextureFormat::Rgba8Unorm, None);
    let (pixels_of_second, _) = drawn_tile_of(PAGE_BIG_LETTER, BIG_TILE, &mut second, &device, &queue, &text);
    assert!(pixels_of_second == pixels, "a second renderer over the same engine draws the big letter");

    // Another engine's big letter has another id: the renderer makes its
    // texture in the slot of the first engine's letter, which it frees.
    let slots: usize = ui.debug_texture_slot_count();
    let other_engine: RefCell<TextEngine> = engine();
    let (pixels_of_other, _) = drawn_tile_of(PAGE_BIG_LETTER, BIG_TILE, &mut ui, &device, &queue, &other_engine);
    assert!(pixels_of_other == pixels, "a renderer handed another engine draws that engine's big letter");
    assert_eq!(ui.debug_texture_slot_count(), slots, "in the slot of the first engine's letter");
}

/// One letter stretched over a box 200 by 150: the renderer asks for
/// the glyph at 200 px, a big glyph.
const PAGE_STRETCHED_LETTER: &str = r#"
View card {
    Label hello text="W", text_fit=stretch, font_color=#FFFFFF, width=200, height=150, halign=left, valign=top
}
"#;

/// A big glyph stretched over a box (`text_fit=stretch`) covers the box
/// from edge to edge and draws nothing outside it.
///
/// Red when the stretched quad is drawn from the atlas, where a big
/// glyph is not: the box stays empty.
#[test]
#[ignore = "needs a GPU adapter: run with --include-ignored on a machine that has one"]
fn a_big_glyph_stretched_over_a_box_covers_the_box() {
    let (device, queue) = device();
    let mut ui: UiRenderer = UiRenderer::new(&device, wgpu::TextureFormat::Rgba8Unorm, None);
    let text: RefCell<TextEngine> = engine();
    let (pixels, label) = drawn_tile_of(PAGE_STRETCHED_LETTER, BIG_TILE, &mut ui, &device, &queue, &text);
    assert_eq!((label.x, label.y, label.w, label.h), (0.0, 0.0, 200.0, 150.0));
    assert!(text.borrow_mut().glyph("W", 200.0, None).expect("ink").big_glyph.is_some(), "the precondition: a big glyph");
    let inked: Vec<(u32, u32)> = (0..BIG_TILE.1).flat_map(|y| (0..BIG_TILE.0).map(move |x| (x, y))).filter(|&(x, y)| pixels[((y * BIG_TILE.0 + x) * 4 + 3) as usize] > 0).collect();
    assert!(inked.len() > 4000, "the letter is drawn: {} pixels", inked.len());
    let (left, right): (u32, u32) = (inked.iter().map(|p| p.0).min().expect("ink"), inked.iter().map(|p| p.0).max().expect("ink"));
    let (top, bottom): (u32, u32) = (inked.iter().map(|p| p.1).min().expect("ink"), inked.iter().map(|p| p.1).max().expect("ink"));
    assert!(left <= 1 && top <= 1 && (198..200).contains(&right) && (148..150).contains(&bottom), "the ink reaches the box's edges and stays inside: {left}..{right} x {top}..{bottom}");
}

/// The texture of a big glyph is freed once the text engine dropped the
/// glyph, and its slot serves the next one: a letter shown at a new
/// size every frame, as in a box resized by a drag, ends with a steady
/// number of textures.
///
/// Red when the renderer keeps every texture it made: half a megabyte
/// of GPU memory a frame, for the whole session.
#[test]
#[ignore = "needs a GPU adapter: run with --include-ignored on a machine that has one"]
fn the_texture_of_a_big_glyph_the_engine_dropped_is_freed() {
    let (device, queue) = device();
    let mut ui: UiRenderer = UiRenderer::new(&device, wgpu::TextureFormat::Rgba8Unorm, None);
    let text: RefCell<TextEngine> = engine();
    // A letter of 900 px is half a megabyte: the engine keeps eight.
    let page = |frame: u32| -> String { format!("View card {{\n    Label hello text=\"W\", font_size={}, font_color=#FFFFFF, halign=left, valign=top\n}}\n", 900 + frame) };
    let mut slots: Vec<usize> = Vec::new();
    let mut last: Vec<u8> = Vec::new();
    for frame in 0..24 {
        (last, _) = drawn_tile_of(&page(frame), BIG_TILE, &mut ui, &device, &queue, &text);
        slots.push(ui.debug_texture_slot_count());
    }
    let raster_bytes: usize = {
        let quad: GlyphQuad = text.borrow_mut().glyph("W", 923.0, None).expect("ink");
        text.borrow().big_glyph(quad.big_glyph.expect("a big glyph")).expect("kept").texels.len()
    };
    assert!(raster_bytes * 24 > 2 * BIG_GLYPH_KEPT_BYTES, "the precondition: the 24 sizes are more than twice what the engine keeps");
    assert!(slots[2] > slots[1] && slots[1] > slots[0], "the precondition: each new size takes a slot at first: {slots:?}");
    assert_eq!(slots[23], slots[16], "then a freed slot serves each new size: {slots:?}");
    assert!((slots[23] - slots[0]) * raster_bytes <= BIG_GLYPH_KEPT_BYTES + 2 * raster_bytes, "the renderer holds textures for what the engine keeps: {slots:?}");

    let mut fresh: UiRenderer = UiRenderer::new(&device, wgpu::TextureFormat::Rgba8Unorm, None);
    let (want, _) = drawn_tile_of(&page(23), BIG_TILE, &mut fresh, &device, &queue, &text);
    assert!(last == want, "a slot used again holds the new size's letter");
    assert!(last.chunks_exact(4).filter(|pixel| *pixel == [255; 4]).count() > 4000, "and the letter is drawn");
}
