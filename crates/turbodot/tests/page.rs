//! A page run by `Ui` with no window: compiled, laid out, tapped, printed
//! as an outline, and drawn on a GPU.
//!
//! Not in the crates.io package (`exclude` in Cargo.toml): it reads a font
//! from `assets/Fonts` at the top of the repository, which a package does
//! not carry.

// Built with no feature (a build script's use), TurboDot has no pages to test.
#![cfg(feature = "ui")]

use gogo_ui::{Event, EventKind, NodeId};
use turbodot::{App, PageSource, Ui};
#[cfg(feature = "gpu")]
use turbodot::{Gpu, Picture};

const PAGE: &str = r#"
View counterPage background=#202020 {
    Column menu halign=left, valign=top, spacing=10 {
        Label counter text="0 taps", font_size=20, font_color=#FFFFFF
        Button tapButton text="Tap me", width=200, height=50
        Rectangle swatch color=#E51400, width=200, height=40
    }
}
"#;

fn font() -> Vec<u8> {
    std::fs::read(concat!(env!("CARGO_MANIFEST_DIR"), "/../../assets/Fonts/Roboto Regular.ttf")).expect("the font")
}

/// Counts the taps on the button and writes the count into the label.
#[derive(Default)]
struct Counter {
    taps: u32,
}

impl App for Counter {
    fn show(&mut self, ui: &mut Ui) {
        let counter: NodeId = ui.node("counter");
        ui.set_text(counter, &format!("{} taps", self.taps));
    }

    fn handle(&mut self, ui: &mut Ui, events: &[Event]) {
        let button: NodeId = ui.node("tapButton");
        for event in events {
            if event.kind == EventKind::Activate && event.node == button {
                self.taps += 1;
                self.show(ui);
            }
        }
    }
}

/// The page after its first frame of 400 x 300, with its app.
fn shown() -> (Ui, Counter) {
    let mut ui: Ui = Ui::new(PageSource::Text(PAGE), &[("Roboto Regular", font())]).expect("the page compiles");
    let mut app: Counter = Counter::default();
    app.show(&mut ui);
    ui.frame(400.0, 300.0, 0.0);
    (ui, app)
}

#[test]
fn a_tap_reaches_the_app_and_the_app_writes_into_the_page() {
    let (mut ui, mut app) = shown();
    for t in [1.0, 2.0] {
        let events: Vec<Event> = ui.tap(ui.node("tapButton"), t);
        app.handle(&mut ui, &events);
    }
    assert_eq!(app.taps, 2);
    assert_eq!(ui.text_of(ui.node("counter")), Some("2 taps"));
}

/// A touch before the first frame lands on nothing.
#[test]
fn a_touch_before_the_first_frame_causes_no_event() {
    let mut ui: Ui = Ui::new(PageSource::Text(PAGE), &[("Roboto Regular", font())]).expect("the page compiles");
    assert!(ui.touch(gogo_ui::Phase::Down, 50.0, 50.0, 0.0).is_empty());
    assert_eq!(ui.outline(), "");
}

/// The outline names every node that shows, with the rectangle a test
/// reads from the same frame.
#[test]
fn the_outline_gives_each_node_its_kind_its_name_and_its_rectangle() {
    let (ui, _) = shown();
    let outline: String = ui.outline();
    let lines: Vec<&str> = outline.lines().collect();
    assert_eq!(lines[0], "View counterPage x=0 y=0 w=400 h=300", "{outline}");
    assert!(lines[1].starts_with("  Column menu x=0 y=0 "), "{outline}");
    let button = ui.rect(ui.node("tapButton"));
    assert_eq!((button.w, button.h), (200.0, 50.0));
    assert!(lines.contains(&format!("    Button tapButton x={} y={} w=200 h=50", button.x, button.y).as_str()), "{outline}");
    let swatch = ui.rect(ui.node("swatch"));
    assert!(lines.contains(&format!("    Rectangle swatch x={} y={} w=200 h=40", swatch.x, swatch.y).as_str()), "{outline}");
}

/// A page that does not compile comes back as the compiler's lines, with
/// the code and the line of each mistake.
#[test]
fn a_page_with_a_mistake_gives_the_compilers_words() {
    let why: String = match Ui::new(PageSource::Text("View v {\n    Label a no_such_word=1\n}"), &[("Roboto Regular", font())]) {
        Ok(_) => panic!("the page has a mistake"),
        Err(why) => why,
    };
    assert!(why.starts_with("error G"), "{why}");
    assert!(why.contains("line 2"), "{why}");
}

#[test]
fn a_page_file_that_is_missing_says_which() {
    let dir = std::path::PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/tests"));
    let why: String = match Ui::new(PageSource::File { dir, name: "no_such_page.gogo" }, &[("Roboto Regular", font())]) {
        Ok(_) => panic!("there is no such file"),
        Err(why) => why,
    };
    assert!(why.contains("no_such_page.gogo"), "{why}");
}

/// The page drawn on a real GPU at two device pixels per epx: twice the
/// pixels, the page's own colors where the page says, and the same colors
/// in a PNG read back from disk.
///
/// Ignored unless asked for: it needs a GPU adapter, which a hosted test
/// machine may not have, and a test that returned early there would count
/// as passed without having run. On a machine with a GPU:
///
///     cargo test -p turbodot -- --include-ignored
#[cfg(feature = "gpu")]
#[test]
#[ignore = "needs a GPU adapter: run with --include-ignored on a machine that has one"]
fn a_page_is_drawn_to_a_picture_in_device_pixels() {
    const BACK: [u8; 4] = [0x20, 0x20, 0x20, 0xFF];
    const RED: [u8; 4] = [0xE5, 0x14, 0x00, 0xFF];

    let (mut ui, _) = shown();
    ui.set_epx(2.0);
    ui.frame(400.0, 300.0, 0.0);
    let gpu: Gpu = Gpu::new().expect("this test needs a GPU adapter");
    let picture: Picture = gpu.draw(&ui).expect("the page is drawn");
    assert_eq!((picture.width, picture.height), (800, 600));
    let swatch = ui.rect(ui.node("swatch"));
    let middle: (u32, u32) = (((swatch.x + swatch.w / 2.0) * 2.0) as u32, ((swatch.y + swatch.h / 2.0) * 2.0) as u32);
    assert_eq!(picture.at(middle.0, middle.1), RED, "the swatch");
    assert_eq!(picture.at(790, 590), BACK, "the page's background");

    let file = std::path::PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/../../target/pictures/turbodot_page.png"));
    picture.save_png(&file).expect("the picture is saved");
    let decoder = png::Decoder::new(std::io::BufReader::new(std::fs::File::open(&file).expect("the saved file")));
    let mut reader = decoder.read_info().expect("a PNG");
    let mut bytes: Vec<u8> = vec![0; reader.output_buffer_size().expect("a size")];
    let info = reader.next_frame(&mut bytes).expect("the pixels");
    assert_eq!((info.width, info.height, info.color_type), (800, 600, png::ColorType::Rgba));
    assert_eq!(&bytes[..info.buffer_size()], &picture.pixels.Bytes[..], "the file holds the picture's bytes");
}
