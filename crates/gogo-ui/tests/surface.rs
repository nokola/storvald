//! `Surface`: a box of a page that the program's code draws.
//!
//! The file says how code fills it (`draw=picture`, `draw=commands`,
//! `draw=gpu`) and there is no default. A Surface has no size of its own and holds no
//! child. Its name in a build has a type by its `draw=`. A picture from
//! code belongs in a Surface: an Image names a file.
use gogo_ui::draw::{Draw, Fit, Sampling};
use gogo_ui::input::{EventKind, EventList, Input, Phase, Touch};
use gogo_ui::layout::{FixedAdvance, Rect, TextMeasure};
use gogo_ui::page::{ImageKey, Kind, NO_KEY, NodeId, PropKey, StrId, Value};
use gogo_ui::runtime::Runtime;
use gogo_ui::{Color_Straight_Srgb_4xU8_RGBA, CommandSurface, GpuSurface, KeptFrame, Page, PictureSurface, SurfaceAsk, SurfaceDraw, TextAlign, compile};

/// Codes of every diagnostic `src` gets.
fn codes(src: &str) -> Vec<String> {
    compile(src, &|_| None).diags.iter().map(|d| d.code.clone()).collect()
}

/// The message of the first diagnostic of `src` with `code`.
fn message(src: &str, code: &str) -> String {
    let c = compile(src, &|_| None);
    c.diags.iter().find(|d| d.code == code).unwrap_or_else(|| panic!("no {code} in {:#?}", c.diags)).message.clone()
}

/// A page, its runtime, and the frame road a host runs.
struct Fx {
    page: Page,
    rt: Runtime,
    kept: KeptFrame,
    draws: Vec<Draw>,
}

impl Fx {
    fn new(src: &str) -> Fx {
        let c = compile(src, &|_| None);
        assert!(!gogo_ui::has_errors(&c.diags), "{:#?}", c.diags);
        let rt = Runtime::new(&c.page);
        Fx { page: c.page, rt, kept: KeptFrame::default(), draws: Vec::new() }
    }

    /// One frame of a window `width` x `height` epx.
    fn frame(&mut self, width: f32, height: f32) {
        self.rt.tick(&mut self.page, 0.0);
        self.kept.frame_retained(&mut self.rt, &self.page, Rect::new(0.0, 0.0, width, height), &FixedAdvance, &mut self.draws, None);
    }

    fn id(&self, name: &str) -> NodeId {
        self.page.find(name).unwrap()
    }

    fn rect(&self, name: &str) -> Rect {
        self.kept.rects()[self.id(name).index()]
    }

    /// Writes one red rectangle over the whole of each Surface asked
    /// for, as a host does after its frame. Returns the asks answered.
    fn answer_asks(&mut self) -> Vec<SurfaceAsk> {
        let mut asks: Vec<SurfaceAsk> = Vec::new();
        while let Some(ask) = self.rt.next_surface_to_draw() {
            let mut out = self.page.write_surface(ask);
            let rect: Rect = Rect::new(0.0, 0.0, out.width(), out.height());
            out.draw(Draw::Quad { rect, color: RED, corner: 0.0, border: 0.0, border_color: Color_Straight_Srgb_4xU8_RGBA::TRANSPARENT });
            asks.push(ask);
        }
        asks
    }
}

const RED: Color_Straight_Srgb_4xU8_RGBA = Color_Straight_Srgb_4xU8_RGBA([0xFF, 0, 0, 0xFF]);

// ---------------------------------------------------------------- the file

/// A Surface says how code fills it. Without `draw=` it does not compile:
/// nobody gets one kind of drawing by forgetting to ask for another.
#[test]
fn a_surface_without_draw_does_not_compile() {
    assert!(codes("View v { Surface s width=10, height=10 }").contains(&"G7015".to_string()));
    assert!(message("View v { Surface s width=10, height=10 }", "G7015").contains("there is no default"));
    assert!(codes("View v { Surface s draw=picture, width=10, height=10 }").is_empty());
    assert!(codes("View v { Surface s draw=commands, width=10, height=10 }").is_empty());
    assert!(codes("View v { Surface s draw=gpu, width=10, height=10 }").is_empty());
    // A word that is none of the set is one error, at the word.
    assert_eq!(codes("View v { Surface s draw=shader, width=10, height=10 }"), ["G0057"]);
}

/// How a Surface is filled is fixed when the page is built: its name in
/// the code has a type by it. A `when` cannot change it.
#[test]
fn draw_is_written_on_the_surface_s_own_line() {
    let src = "View v { Surface s draw=commands, width=10, height=10 {\n when screen.landscape draw=picture\n} }";
    assert!(codes(src).contains(&"G7015".to_string()), "{:?}", codes(src));
}

/// `fit=`, `window=` and `sampling=` place a picture: they belong to a
/// `draw=picture` Surface. `clip_contents=` cuts draw commands at the
/// Surface's edge: it belongs to a `draw=commands` one.
#[test]
fn a_surface_takes_the_words_of_its_own_draw() {
    assert!(codes("View v { Surface s draw=picture, fit=cover, sampling=pixel, window=[0, 0, 4, 4], width=10, height=10 }").is_empty());
    assert!(codes("View v { Surface s draw=commands, clip_contents=true, width=10, height=10 }").is_empty());
    for word in ["fit=cover", "sampling=pixel", "window=[0, 0, 4, 4]"] {
        let src = format!("View v {{ Surface s draw=commands, {word}, width=10, height=10 }}");
        assert_eq!(codes(&src), ["G7016"], "{src}");
    }
    let said: String = message("View v { Surface s draw=picture, clip_contents=true, width=10, height=10 }", "G7016");
    assert!(said.contains("`clip_contents=` belongs to a `draw=commands` Surface") && said.contains("`draw=picture`"), "{said}");
}

/// A Surface holds nothing: what goes over it is a node beside it in a
/// `Layers`.
#[test]
fn a_surface_holds_no_child() {
    let src = "View v { Surface s draw=commands, width=10, height=10 {\n Label l text=\"x\"\n} }";
    assert!(codes(src).contains(&"G7014".to_string()), "{:?}", codes(src));
}

/// An Image names a picture file. A name that is no file made an Image
/// show what code draws, with nothing on the line to say so: it is an
/// error that names the Surface to write.
#[test]
fn an_image_names_a_file_and_a_picture_from_code_goes_in_a_surface() {
    let said: String = message("View v { Image i source=\"preview://brush\", width=10, height=10 }", "G2023");
    assert!(said.contains("`source=\"preview://brush\"` names no picture file") && said.contains("Surface <name> draw=picture"), "{said}");
    // In a `when` too: every `source=` passes the same check.
    let in_when = "View v { Image i source=\"res://a.png\", width=10, height=10 {\n when screen.landscape source=\"live://b\"\n} }";
    assert!(codes(in_when).contains(&"G2023".to_string()), "{:?}", codes(in_when));
    // A file, and an Image with no picture yet, stay as they were.
    assert!(!codes("View v { Image i source=\"res://a.png\", width=10, height=10 }").contains(&"G2023".to_string()));
    assert!(codes("View v { Image i source=\"\", width=10, height=10 }").is_empty());
}

// ---------------------------------------------------------------- layout

/// A Surface has no size of its own: gogo does not know what code draws.
/// In a parent that asks its children for their size it answers 0, and
/// it takes the size its words give it.
#[test]
fn a_surface_has_no_size_of_its_own() {
    let mut fx = Fx::new("View v { Layers l width=fill, height=fill {\n Surface bare draw=picture, halign=left, valign=top\n Column c halign=left, valign=top {\n  Surface sized draw=picture, width=120, height=40\n  Surface wide draw=commands, width=fill, height=30\n }\n} }");
    fx.frame(400.0, 800.0);
    assert_eq!((fx.rect("bare").w, fx.rect("bare").h), (0.0, 0.0));
    assert_eq!((fx.rect("sized").w, fx.rect("sized").h), (120.0, 40.0));
    assert_eq!((fx.rect("wide").w, fx.rect("wide").h), (120.0, 30.0), "`fill` takes the column's width, which the sized Surface gives");
    assert_eq!((fx.rect("c").w, fx.rect("c").h), (120.0, 70.0), "the column is as large as its two sized Surfaces");
}

/// A measure that knows the size of one picture, key 7: 300 x 200.
struct KnowsOnePicture;

impl TextMeasure for KnowsOnePicture {
    fn measure(&self, text: &str, font_size: f32, max_width: Option<f32>, font: Option<&str>) -> (f32, f32) {
        FixedAdvance.measure(text, font_size, max_width, font)
    }
    fn image_size(&self, key: ImageKey) -> Option<(f32, f32)> {
        (key == ImageKey::new(7)).then_some((300.0, 200.0))
    }
}

/// A Surface does not take the size of the picture code handed it, where
/// an Image with the same picture does: the Surface's file sizes it, and
/// `size=hug` on it is 0 by 0.
#[test]
fn a_surface_does_not_take_its_picture_s_size() {
    let mut fx = Fx::new("View v { Layers l width=fill, height=fill {\n Image file source=\"\", size=hug, halign=left, valign=top\n Surface made draw=picture, size=hug, halign=left, valign=top\n} }");
    let (file, made): (NodeId, PictureSurface) = (fx.id("file"), PictureSurface::find(&fx.page, "made").unwrap());
    fx.rt.set_prop(file, PropKey::Source, Value::VImage(ImageKey::new(7)));
    fx.rt.set_picture(made, ImageKey::new(7));
    fx.rt.tick(&mut fx.page, 0.0);
    fx.kept.frame_retained(&mut fx.rt, &fx.page, Rect::new(0.0, 0.0, 400.0, 800.0), &KnowsOnePicture, &mut fx.draws, None);
    assert_eq!((fx.rect("file").w, fx.rect("file").h), (300.0, 200.0), "the Image hugs its picture");
    assert_eq!((fx.rect("made").w, fx.rect("made").h), (0.0, 0.0), "the Surface has no size of its own");
}

// ---------------------------------------------------------------- draw=picture

/// A `draw=picture` Surface shows the picture code handed it as an Image
/// shows a file: its own `fit=`, `window=` and `sampling=`. Before code
/// hands one over it draws nothing.
#[test]
fn a_picture_surface_shows_the_picture_code_set() {
    let mut fx = Fx::new("View v { Layers l width=fill, height=fill {\n Surface plain draw=picture, width=100, height=50, halign=left, valign=top\n Surface tile draw=picture, fit=distort, sampling=pixel, window=[2, 2, 8, 8], width=40, height=40, halign=right, valign=bottom\n} }");
    fx.frame(400.0, 800.0);
    assert!(!fx.draws.iter().any(|d| matches!(d, Draw::Image { .. })), "no picture yet: {:?}", fx.draws);

    let plain: PictureSurface = PictureSurface::find(&fx.page, "plain").unwrap();
    let tile: PictureSurface = PictureSurface::find(&fx.page, "tile").unwrap();
    fx.rt.set_picture(plain, ImageKey::new(7));
    fx.rt.set_picture(tile, ImageKey::new(9));
    fx.frame(400.0, 800.0);
    let images: Vec<&Draw> = fx.draws.iter().filter(|d| matches!(d, Draw::Image { .. })).collect();
    assert_eq!(images.len(), 2, "{:?}", fx.draws);
    let white = Color_Straight_Srgb_4xU8_RGBA::opaque(0xFFFFFF);
    assert_eq!(*images[0], Draw::Image { rect: Rect::new(0.0, 0.0, 100.0, 50.0), image: ImageKey::new(7), tint: white, fit: Fit::Contain, window: None, sampling: Sampling::Smooth });
    assert_eq!(*images[1], Draw::Image { rect: Rect::new(360.0, 760.0, 40.0, 40.0), image: ImageKey::new(9), tint: white, fit: Fit::Distort, window: Some([2.0, 2.0, 8.0, 8.0]), sampling: Sampling::Pixel });

    // Code takes the picture away again.
    fx.rt.set_picture(plain, gogo_ui::NO_IMAGE);
    fx.frame(400.0, 800.0);
    assert_eq!(fx.draws.iter().filter(|d| matches!(d, Draw::Image { .. })).count(), 1);
}

/// A name's type follows the file's `draw=`: a `draw=commands` Surface
/// is no `PictureSurface`, and a node that is no Surface is neither.
#[test]
fn a_surface_s_name_has_the_type_of_its_draw() {
    let fx = Fx::new("View v { Layers l {\n Surface photo draw=picture, width=10, height=10\n Surface board draw=commands, width=10, height=10\n Panel plate width=10, height=10\n} }");
    assert!(PictureSurface::find(&fx.page, "photo").is_some());
    assert!(PictureSurface::find(&fx.page, "board").is_none());
    assert!(CommandSurface::find(&fx.page, "board").is_some());
    assert!(CommandSurface::find(&fx.page, "photo").is_none());
    assert!(PictureSurface::find(&fx.page, "plate").is_none() && CommandSurface::find(&fx.page, "plate").is_none());
    assert_eq!(SurfaceDraw::of(&fx.page, fx.id("photo")), Some(SurfaceDraw::Picture));
    assert_eq!(SurfaceDraw::of(&fx.page, fx.id("plate")), None);
    assert_eq!(fx.page.nodes[fx.id("board").index()].kind, Kind::Surface);
}

/// The build writes each Surface's name with its type, in `NAMES`, in a
/// repeat's row (the row itself when it is a Surface) and in a widget's
/// names.
#[test]
fn the_build_writes_a_surface_s_name_with_its_type() {
    let src = "widget Card {\n Layers width=40, height=40 {\n  Surface face draw=picture, width=fill, height=fill\n }\n}\nView v { Column c {\n Surface board draw=commands, width=100, height=100\n Card first\n repeat tiles capacity=2 {\n  Surface tile draw=picture, width=10, height=10\n }\n repeat rows capacity=2 {\n  Layers row width=20, height=20 {\n   Surface thumb draw=picture, width=fill, height=fill\n  }\n }\n} }";
    let c = compile(src, &|_| None);
    assert!(!gogo_ui::has_errors(&c.diags), "{:#?}", c.diags);
    let code: String = gogo_ui::emit::emit_rust(&c.page, "m").unwrap();
    let board: NodeId = c.page.find("board").unwrap();
    assert!(code.contains("pub board: gogo_ui::CommandSurface,"), "{code}");
    assert!(code.contains(&format!("board: gogo_ui::CommandSurface::from_build(gogo_ui::{board:?}),")), "{code}");
    // A row that is a Surface: `root` has the type, and `at` takes the
    // row's node as `Runtime::items` hands it out.
    assert!(code.contains("pub struct TilesSlot {\n        pub root: gogo_ui::PictureSurface,\n    }"), "{code}");
    assert!(code.contains("pub const fn at(slot_root: NodeId) -> TilesSlot {\n            TilesSlot::TEMPLATE.rebase(TilesSlot::TEMPLATE.root.node(), slot_root)"), "{code}");
    // A Surface inside a row.
    assert!(code.contains("pub struct RowsSlot {\n        pub root: NodeId,\n        pub thumb: gogo_ui::PictureSurface,\n    }"), "{code}");
    // A Surface inside a widget: the widget's names, and its `bind`
    // finds it by name and by `draw=`.
    let names: String = gogo_ui::emit::emit_widget_names(&[("m", &c.page)]).unwrap();
    assert!(names.contains("pub struct CardNames {\n    pub root: NodeId,\n    pub face: gogo_ui::PictureSurface,\n}"), "{names}");
    assert!(names.contains("face: gogo_ui::PictureSurface::find(page, &format!(\"{instance}/face\"))?,"), "{names}");
}

// ---------------------------------------------------------------- draw=commands

/// gogo asks for a Surface's commands when it shows for the first time,
/// when its size changed, and when code said its content changed. In
/// every other frame it asks for nothing and draws the commands it kept.
#[test]
fn gogo_asks_for_commands_in_three_cases_and_keeps_them() {
    let mut fx = Fx::new("View v { Layers l width=fill, height=fill {\n Surface board draw=commands, width=fill, height=100, valign=top\n} }");
    let board: CommandSurface = CommandSurface::find(&fx.page, "board").unwrap();
    fx.frame(400.0, 800.0);
    assert_eq!(fx.answer_asks(), [SurfaceAsk { surface: board, width: 400.0, height: 100.0 }], "it shows for the first time");
    assert_eq!(fx.page.surface_commands(board.node()), [Draw::Quad { rect: Rect::new(0.0, 0.0, 400.0, 100.0), color: RED, corner: 0.0, border: 0.0, border_color: Color_Straight_Srgb_4xU8_RGBA::TRANSPARENT }]);

    // An ask is taken once: a host that draws again with no layout in
    // between is not asked again.
    assert_eq!(fx.rt.next_surface_to_draw(), None);
    for _ in 0..3 {
        fx.frame(400.0, 800.0);
        assert_eq!(fx.rt.next_surface_to_draw(), None, "nothing changed");
    }

    fx.frame(300.0, 800.0);
    assert_eq!(fx.answer_asks(), [SurfaceAsk { surface: board, width: 300.0, height: 100.0 }], "its size changed");
    fx.frame(300.0, 800.0);
    assert_eq!(fx.rt.next_surface_to_draw(), None);

    fx.rt.surface_changed(&mut fx.page, board);
    assert!(fx.rt.dirty, "a host that draws only when something changed draws the next frame");
    fx.frame(300.0, 800.0);
    assert_eq!(fx.answer_asks(), [SurfaceAsk { surface: board, width: 300.0, height: 100.0 }], "code said its content changed");
    fx.frame(300.0, 800.0);
    assert_eq!(fx.rt.next_surface_to_draw(), None);
}

/// The page's own draw list holds a mark where the Surface is, and the
/// commands stay with the page. So code that writes its commands again
/// makes gogo build nothing of the page's list, and a page that changes
/// does not ask code for the commands again.
#[test]
fn new_commands_do_not_build_the_page_s_draw_list_again() {
    let mut fx = Fx::new("View v { Layers l width=fill, height=fill {\n Rectangle under width=fill, height=fill, color=#000000FF\n Surface board draw=commands, width=200, height=100, halign=left, valign=top, margin_left=20, margin_top=30, opacity=0.5\n Label over text=\"x\"/_, halign=right, valign=bottom\n} }");
    let board: CommandSurface = CommandSurface::find(&fx.page, "board").unwrap();
    fx.frame(400.0, 800.0);
    fx.answer_asks();
    // Paint order: the rectangle, the Surface's mark, the label.
    let mark_at: usize = fx.draws.iter().position(|d| matches!(d, Draw::Surface { .. })).expect("a mark");
    assert_eq!(fx.draws[mark_at], Draw::Surface { rect: Rect::new(20.0, 30.0, 200.0, 100.0), surface: board.node(), alpha: 0.5 });
    assert!(matches!(fx.draws[mark_at - 1], Draw::Quad { .. }) && matches!(fx.draws[mark_at + 1], Draw::Text { .. }), "{:?}", fx.draws);

    // Code writes the commands again: the page's list is not walked.
    fx.frame(400.0, 800.0);
    let list_before: Vec<Draw> = fx.draws.clone();
    fx.rt.surface_changed(&mut fx.page, board);
    fx.frame(400.0, 800.0);
    assert_eq!(fx.answer_asks().len(), 1);
    assert_eq!(fx.rt.debug_draw_stats().emitted, 0, "{:?}", fx.rt.debug_draw_stats());
    assert_eq!(fx.draws, list_before);

    // The page changes beside the Surface: no ask.
    let over: NodeId = fx.id("over");
    fx.rt.set_text(&mut fx.page, over, "y");
    fx.frame(400.0, 800.0);
    assert_eq!(fx.rt.next_surface_to_draw(), None);
}

/// A Surface that is not shown, or has no area, is not asked for: its
/// commands would be for a picture nobody sees. It is asked for when it
/// shows.
#[test]
fn a_surface_that_does_not_show_is_not_asked_for() {
    let mut fx = Fx::new("View v { Layers l width=fill, height=fill {\n Surface gone draw=commands, width=50, height=50, visibility=gone\n Surface kept draw=commands, width=50, height=50, visibility=space_only\n Surface flat draw=commands, width=50, height=0\n} }");
    fx.frame(400.0, 800.0);
    assert_eq!(fx.rt.next_surface_to_draw(), None);
    assert!(!fx.draws.iter().any(|d| matches!(d, Draw::Surface { .. })), "{:?}", fx.draws);
    let gone: NodeId = fx.id("gone");
    fx.rt.set_visible(&mut fx.page, gone, true);
    fx.frame(400.0, 800.0);
    let asked: Vec<NodeId> = fx.answer_asks().iter().map(|ask| ask.surface.node()).collect();
    assert_eq!(asked, [gone]);
}

/// A text of a Surface's commands that changes with every write goes
/// into the string the write before used: a score that counts up adds
/// no string to the page.
#[test]
fn a_text_written_again_uses_the_string_it_had() {
    let mut fx = Fx::new("View v { Surface board draw=commands, width=200, height=100 }");
    let board: CommandSurface = CommandSurface::find(&fx.page, "board").unwrap();
    fx.frame(400.0, 800.0);
    let mut strings_after_first: usize = 0;
    let mut id_of_first: Option<StrId> = None;
    for score in 0..50 {
        fx.rt.surface_changed(&mut fx.page, board);
        fx.frame(400.0, 800.0);
        let ask: SurfaceAsk = fx.rt.next_surface_to_draw().expect("an ask");
        let mut out = fx.page.write_surface(ask);
        let text: StrId = out.text(&format!("score {score}"));
        let label: StrId = out.text("points");
        out.draw(Draw::Text { rect: Rect::new(0.0, 0.0, 100.0, 20.0), text, key: NO_KEY, font_size: 14.0, color: RED, halign: TextAlign::Start, valign: TextAlign::Start, wrap: false, font: gogo_ui::NO_STR });
        assert_ne!(text, label);
        if score == 0 {
            strings_after_first = fx.page.strings.len();
            id_of_first = Some(text);
        }
        assert_eq!(Some(text), id_of_first);
        assert_eq!(fx.page.str(text), format!("score {score}"));
        assert_eq!(fx.page.str(label), "points");
    }
    assert_eq!(fx.page.strings.len(), strings_after_first);
}

/// A row of a `repeat` that is a Surface is asked for like any other:
/// rows the list gains later are found.
#[test]
fn a_surface_in_a_row_added_later_is_asked_for() {
    let mut fx = Fx::new("View v { Column c halign=left, valign=top {\n repeat cells capacity=1 {\n  Surface cell draw=commands, width=30, height=30\n }\n} }");
    let cells: NodeId = fx.id("cells");
    fx.frame(400.0, 800.0);
    assert_eq!(fx.rt.next_surface_to_draw(), None, "the hidden row the others are made from is not asked for");
    fx.rt.set_count(&mut fx.page, cells, 3);
    fx.frame(400.0, 800.0);
    let rows: Vec<NodeId> = fx.rt.items(&fx.page, cells);
    let asked: Vec<NodeId> = fx.answer_asks().iter().map(|ask| ask.surface.node()).collect();
    assert_eq!(asked, rows);
    fx.frame(400.0, 800.0);
    assert_eq!(fx.rt.next_surface_to_draw(), None);
}

// ---------------------------------------------------------------- fingers

fn pointer_downs(events: &EventList) -> Vec<u64> {
    events
        .iter()
        .filter_map(|e| match e.kind {
            EventKind::Pointer(Phase::Down, sample) => Some(sample.finger),
            _ => None,
        })
        .collect()
}

/// A Surface takes every finger that lands on it, each with a press of
/// its own: the program's code tells one finger (a stroke) from two (a
/// zoom). Any other box holds one finger at a time.
#[test]
fn a_surface_takes_every_finger_that_lands_on_it() {
    let src = "View v { Layers l width=fill, height=fill {\n Surface world draw=commands, width=fill, height=300, valign=top, interactive=true, on:pointer=world_pointer\n Panel pad width=fill, height=300, valign=bottom, interactive=true, on:pointer=pad_pointer\n} }";
    let mut fx = Fx::new(src);
    fx.frame(400.0, 800.0);
    let mut input = Input::new();
    let mut touch = |fx: &mut Fx, phase: Phase, x: f32, y: f32, id: u64| -> EventList { input.touch(&fx.page, &mut fx.rt, fx.kept.frame(), &FixedAdvance, Touch { phase, x, y, t: 0.0, id, ..Touch::PLAIN }) };
    let world: NodeId = fx.id("world");

    assert_eq!(pointer_downs(&touch(&mut fx, Phase::Down, 100.0, 100.0, 1)), [1]);
    assert_eq!(pointer_downs(&touch(&mut fx, Phase::Down, 300.0, 200.0, 2)), [2], "the second finger on the Surface has a press of its own");
    // Each finger's moves and lift arrive under its own id, in the
    // Surface's own space.
    let moved: EventList = touch(&mut fx, Phase::Move, 310.0, 210.0, 2);
    let sample = moved.iter().find_map(|e| match e.kind {
        EventKind::Pointer(Phase::Move, sample) if e.node == world => Some(sample),
        _ => None,
    });
    assert_eq!(sample.map(|s| (s.finger, s.x, s.y)), Some((2, 310.0, 210.0)));
    let lifted: EventList = touch(&mut fx, Phase::Up, 100.0, 100.0, 1);
    assert!(lifted.iter().any(|e| matches!(e.kind, EventKind::Pointer(Phase::Up, sample) if sample.finger == 1)));
    let lifted: EventList = touch(&mut fx, Phase::Up, 310.0, 210.0, 2);
    assert!(lifted.iter().any(|e| matches!(e.kind, EventKind::Pointer(Phase::Up, sample) if sample.finger == 2)));

    // A box that is no Surface: the second finger does nothing.
    assert_eq!(pointer_downs(&touch(&mut fx, Phase::Down, 100.0, 600.0, 3)), [3]);
    assert!(touch(&mut fx, Phase::Down, 300.0, 700.0, 4).is_empty());
}

// ---------------------------------------------------------------- hot reload

/// A saved file that changes a Surface's `draw=` is not shown: the
/// program's code names the Surface with a type by its `draw=`, fixed
/// when the program was built. Any other word of a Surface reloads.
#[test]
fn a_reload_refuses_a_surface_with_another_draw() {
    use gogo_ui::reload::{ShapeChange, same_shape};
    let page = |src: &str| -> Page {
        let c = compile(src, &|_| None);
        assert!(!gogo_ui::has_errors(&c.diags), "{:#?}", c.diags);
        c.page
    };
    let built: Page = page("View v { Surface board draw=commands, width=100, height=50 }");
    assert_eq!(same_shape(&built, &page("View v { Surface board draw=commands, width=200, height=80, opacity=0.5 }")), Ok(()));
    let changed: ShapeChange = same_shape(&built, &page("View v { Surface board draw=picture, width=100, height=50 }")).unwrap_err();
    assert_eq!(changed, ShapeChange::Draw { name: "board".to_string(), old: SurfaceDraw::Commands, new: SurfaceDraw::Picture });
    assert!(changed.to_string().contains("Surface `board` changed its `draw=` (commands -> picture)"), "{changed}");
}

// ---------------------------------------------------------------- draw=gpu

/// A `draw=gpu` Surface takes none of the words of the other two: gogo
/// places no picture in it and cuts no commands of it.
#[test]
fn a_gpu_surface_takes_no_word_of_another_draw() {
    for word in ["fit=cover", "sampling=pixel", "window=[0, 0, 4, 4]", "clip_contents=true"] {
        let src = format!("View v {{ Surface world draw=gpu, {word}, width=10, height=10 }}");
        assert_eq!(codes(&src), ["G7016"], "{src}");
    }
}

/// A `draw=gpu` Surface cannot be turned: the program's own draw calls
/// run in a viewport, which is an upright rectangle. A turn written on
/// the Surface, or on a node above it, is an error at the Surface's line.
/// A turn of 0, and a turn beside the Surface, are none.
#[test]
fn a_turn_on_a_gpu_surface_or_above_it_does_not_compile() {
    let on_it: String = message("View v { Surface world draw=gpu, rotate=10, width=10, height=10 }", "G7017");
    assert!(on_it.contains("`rotate=` is written on it"), "{on_it}");
    let above: String = message("View v { Layers box rotate=45, width=fill, height=fill {\n Panel inner width=fill, height=fill {\n  Surface world draw=gpu, width=fill, height=fill\n }\n} }", "G7017");
    assert!(above.contains("on `Layers`, a node above it") && above.contains("draw=picture"), "{above}");
    assert!(codes("View v { Layers box rotate=0, width=fill, height=fill {\n Surface world draw=gpu, width=fill, height=fill\n} }").is_empty(), "a turn of 0 turns nothing");
    assert!(codes("View v { Layers l width=fill, height=fill {\n Rectangle beside 10x10, rotate=45, color=#FF0000FF\n Surface world draw=gpu, width=fill, height=fill\n} }").is_empty(), "a turned node beside the Surface");
    // The other two kinds turn: gogo draws them.
    assert!(codes("View v { Surface s draw=picture, rotate=10, width=10, height=10 }").is_empty());
    assert!(codes("View v { Surface s draw=commands, rotate=10, width=10, height=10 }").is_empty());
}

/// The page's draw list holds a `draw=gpu` Surface's place, in paint
/// order, with how see-through it is: the program's own draw calls run
/// there when the frame is drawn. Its name is a `GpuSurface`.
#[test]
fn a_gpu_surface_puts_its_place_in_the_draw_list() {
    let mut fx = Fx::new("View v { Layers l width=fill, height=fill {\n Rectangle under width=fill, height=fill, color=#000000FF\n Surface world draw=gpu, width=200, height=100, halign=left, valign=top, margin_left=20, margin_top=30, opacity=0.5\n Label over text=\"x\"/_, halign=right, valign=bottom\n Surface unseen draw=gpu, width=50, height=50, visibility=gone\n} }");
    let world: GpuSurface = GpuSurface::find(&fx.page, "world").unwrap();
    assert!(PictureSurface::find(&fx.page, "world").is_none() && CommandSurface::find(&fx.page, "world").is_none());
    assert_eq!(SurfaceDraw::of(&fx.page, world.node()), Some(SurfaceDraw::Gpu));
    fx.frame(400.0, 800.0);
    let marks: Vec<&Draw> = fx.draws.iter().filter(|d| matches!(d, Draw::GpuSurface { .. })).collect();
    assert_eq!(marks, [&Draw::GpuSurface { rect: Rect::new(20.0, 30.0, 200.0, 100.0), surface: world.node(), alpha: 0.5, has_depth: false }], "one place: a Surface that does not show has none");
    let at: usize = fx.draws.iter().position(|d| matches!(d, Draw::GpuSurface { .. })).unwrap();
    assert!(matches!(fx.draws[at - 1], Draw::Quad { .. }) && matches!(fx.draws[at + 1], Draw::Text { .. }), "over what comes before it, under what comes after: {:?}", fx.draws);
    // gogo asks for no commands of it: the code draws when the frame is drawn.
    assert_eq!(fx.rt.next_surface_to_draw(), None);
    // The place moves and fades with a page that slides.
    assert_eq!(fx.draws[at].posed(10.0, -5.0, 0.5), Draw::GpuSurface { rect: Rect::new(30.0, 25.0, 200.0, 100.0), surface: world.node(), alpha: 0.25, has_depth: false });
}

/// The build writes a `draw=gpu` Surface's name as a `GpuSurface`.
#[test]
fn the_build_writes_a_gpu_surface_s_name_with_its_type() {
    let c = compile("View v { Surface world draw=gpu, width=100, height=100 }", &|_| None);
    assert!(!gogo_ui::has_errors(&c.diags), "{:#?}", c.diags);
    let code: String = gogo_ui::emit::emit_rust(&c.page, "m").unwrap();
    assert!(code.contains("pub world: gogo_ui::GpuSurface,"), "{code}");
}

// ---------------------------------------------------------------- depth=true

/// `depth=true` goes with `draw=gpu` alone: the program's own draw calls
/// test and write depth, and gogo's drawing never does. On a Surface of
/// another `draw=` it is G7016, on any other node G2002, with a message
/// that says whose word it is.
#[test]
fn depth_belongs_to_a_gpu_surface() {
    assert!(codes("View v { Surface world draw=gpu, depth=true, width=10, height=10 }").is_empty());
    assert!(codes("View v { Surface world draw=gpu, depth=false, width=10, height=10 }").is_empty());
    for draw in ["picture", "commands"] {
        let src = format!("View v {{ Surface s draw={draw}, depth=true, width=10, height=10 }}");
        assert_eq!(codes(&src), ["G7016"], "{src}");
        let said: String = message(&src, "G7016");
        assert!(said.contains("`depth=` belongs to a `draw=gpu` Surface") && said.contains(&format!("`draw={draw}`")), "{said}");
    }
    let on_a_shape: String = message("View v { Rectangle r 10x10, depth=true, color=#FF0000FF }", "G2002");
    assert!(on_a_shape.contains("`depth=` gives the program's own draw calls a depth buffer") && on_a_shape.contains("`Surface draw=gpu`"), "{on_a_shape}");
    // True or false, and no other word.
    assert_eq!(codes("View v { Surface world draw=gpu, depth=maybe, width=10, height=10 }"), ["G0078"]);
}

/// Whether the program has a depth buffer is fixed when the program is
/// built: one Surface that says `depth=true` gives every frame one. A
/// `when` cannot change it.
#[test]
fn depth_is_written_on_the_surface_s_own_line() {
    let src = "View v { Surface world draw=gpu, width=10, height=10 {\n when screen.landscape depth=true\n} }";
    assert_eq!(codes(src), ["G7018"], "{:?}", codes(src));
    assert!(message(src, "G7018").contains("fixed when the program is built"));
}

/// The place of a `draw=gpu` Surface in the draw list says whether the
/// Surface has depth, so a renderer knows without reading the page.
#[test]
fn the_place_in_the_draw_list_says_whether_the_surface_has_depth() {
    let mut fx = Fx::new("View v { Layers l width=fill, height=fill {\n Surface world draw=gpu, depth=true, width=fill, height=fill\n Surface flat draw=gpu, width=50, height=50, halign=left, valign=top\n Surface off draw=gpu, depth=false, width=50, height=50, halign=right, valign=top\n} }");
    fx.frame(400.0, 800.0);
    let depths: Vec<(NodeId, bool)> = fx
        .draws
        .iter()
        .filter_map(|d| match d {
            Draw::GpuSurface { surface, has_depth, .. } => Some((*surface, *has_depth)),
            _ => None,
        })
        .collect();
    assert_eq!(depths, [(fx.id("world"), true), (fx.id("flat"), false), (fx.id("off"), false)]);
    // A page that slides keeps it.
    let world: usize = fx.draws.iter().position(|d| matches!(d, Draw::GpuSurface { has_depth: true, .. })).unwrap();
    assert!(matches!(fx.draws[world].posed(10.0, 0.0, 0.5), Draw::GpuSurface { has_depth: true, .. }));
}

/// A page says whether any of its Surfaces has depth: what a build asks
/// each page to know if the program needs a depth buffer.
#[test]
fn a_page_says_whether_a_surface_of_it_has_depth() {
    let page = |src: &str| -> Page {
        let c = compile(src, &|_| None);
        assert!(!gogo_ui::has_errors(&c.diags), "{:#?}", c.diags);
        c.page
    };
    assert!(page("View v { Panel p width=fill, height=fill {\n Surface world draw=gpu, depth=true, width=fill, height=fill\n} }").has_depth_surface());
    assert!(!page("View v { Surface world draw=gpu, width=fill, height=fill }").has_depth_surface());
    assert!(!page("View v { Surface world draw=gpu, depth=false, width=fill, height=fill }").has_depth_surface());
    assert!(!page("View v { Surface s draw=picture, width=10, height=10 }").has_depth_surface());
}

/// The build writes `HAS_DEPTH_SURFACE`: true when a Surface of any page
/// says `depth=true`, false for a build with none. A host makes a depth
/// buffer by it, so a program with no such Surface never makes one.
#[test]
fn a_build_says_whether_a_surface_of_any_page_has_depth() {
    let build = |tag: &str, pages: &[(&str, &str)]| -> String {
        let dir = std::env::temp_dir().join(format!("gogo_depth_{tag}_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let ui = dir.join("ui");
        std::fs::create_dir_all(&ui).unwrap();
        for (file, text) in pages {
            std::fs::write(ui.join(file), text).unwrap();
        }
        let out = dir.join("pages.rs");
        gogo_ui::build_pages(&ui, &out, gogo_ui::BuildOptions::new(&dir)).unwrap();
        let source: String = std::fs::read_to_string(&out).unwrap();
        let _ = std::fs::remove_dir_all(&dir);
        source
    };
    let flat: &str = "View root { Surface stamp draw=gpu, width=48, height=48 }\n";
    let deep: &str = "View root { Surface world draw=gpu, depth=true, width=fill, height=fill }\n";
    assert!(build("none", &[("a.gogo", flat)]).contains("pub const HAS_DEPTH_SURFACE: bool = false;"));
    // One page of two is enough.
    assert!(build("one", &[("a.gogo", flat), ("b.gogo", deep)]).contains("pub const HAS_DEPTH_SURFACE: bool = true;"));
}

/// A saved file that changes a Surface's `depth=` is not shown: whether
/// the program has a depth buffer was fixed when it was built.
#[test]
fn a_reload_refuses_a_surface_with_another_depth() {
    use gogo_ui::reload::{ShapeChange, same_shape};
    let page = |src: &str| -> Page {
        let c = compile(src, &|_| None);
        assert!(!gogo_ui::has_errors(&c.diags), "{:#?}", c.diags);
        c.page
    };
    let built: Page = page("View v { Surface world draw=gpu, width=100, height=50 }");
    assert_eq!(same_shape(&built, &page("View v { Surface world draw=gpu, depth=false, width=200, height=80 }")), Ok(()), "`depth=false` is what no word says");
    let changed: ShapeChange = same_shape(&built, &page("View v { Surface world draw=gpu, depth=true, width=100, height=50 }")).unwrap_err();
    assert_eq!(changed, ShapeChange::Depth { name: "world".to_string() });
    assert!(changed.to_string().contains("Surface `world` changed its `depth=`") && changed.to_string().contains("needs a rebuild"), "{changed}");
    let deep: Page = page("View v { Surface world draw=gpu, depth=true, width=100, height=50 }");
    assert_eq!(same_shape(&deep, &built), Err(ShapeChange::Depth { name: "world".to_string() }), "and the other way");
}
