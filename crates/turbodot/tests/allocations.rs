//! Count of heap allocations a page costs while it runs, by
//! `turbodot::alloc_count`. The page runs in a `pages::PageStack`, which a
//! `Program` and a host with its own pages both stand on.
//!
//! One test in this file, on purpose: counts are for the whole process,
//! and cargo runs the tests of one file at the same time.
//!
//! Not in the crates.io package (`exclude` in Cargo.toml): it reads a font
//! from `assets/Fonts` at the top of the repository, which a package does
//! not carry.

// Built with no feature (a build script's use), TurboDot has no pages to test.
#![cfg(feature = "ui")]

use gogo_text::TextEngine;
use gogo_ui::{Buttons, Event, EventKind, EventList, Images, MOUSE_ID, NodeId, Orientation, Phase, Rect, ScreenClass, Touch, compile, has_errors};
use turbodot::alloc_count::{self, CountingAlloc};
use turbodot::pages::{PageUi, ImageTable, PageId, PagePlatform, PageStack, UiAssets};

#[global_allocator]
static ALLOC: CountingAlloc = CountingAlloc;

const PAGE: &str = r#"
View page background=#202020 {
    Column menu halign=left, valign=top, spacing=10, width=300 {
        Label title text="Volume", font_size=20, font_color=#FFFFFF
        Button tapButton text="Tap me", width=200, height=50
        Slider volume width=200, value=20
        Button turned text="Turned", width=120, height=40, rotate=15
        Scroll box width=200, height=80 {
            Label long text="one two three four five six seven eight nine ten one two three four five six seven eight nine ten one two three four five six seven eight nine ten one two three four five six seven eight nine ten one two three four five six seven eight nine ten one two three four five six seven eight nine ten one two three four five six seven eight nine ten one two three four five six seven eight nine ten", wrap=true, width=fill, font_color=#FFFFFF
        }
    }
}
"#;

const SIZE: (u32, u32) = (400, 400);
const EPX: f32 = 1.0;
const FRAME_SECONDS: f32 = 0.016;

/// Held by a test for as long as it counts.
///
/// Needed because the count is of the whole process: a test that runs
/// on another thread at the same time adds its allocations to it.
static ONE_TEST_AT_A_TIME: std::sync::Mutex<()> = std::sync::Mutex::new(());

/// Allocations `work` makes.
fn allocations_of(work: impl FnOnce()) -> u64 {
    let before: u64 = alloc_count::allocations();
    work();
    alloc_count::allocations() - before
}

/// One page in a stack, with what it lays out and draws against.
struct Shown {
    stack: PageStack,
    assets: UiAssets,
    platform: PagePlatform,
    id: PageId,
    /// `on:hover=` events of the last mouse move.
    heard: Vec<(PageId, Event)>,
}

impl Shown {
    fn new() -> Shown {
        Shown::of(PAGE)
    }

    /// The page `text` describes, in a stack.
    fn of(text: &str) -> Shown {
        let compiled = compile(text, &|_| None);
        assert!(!has_errors(&compiled.diags), "{:?}", compiled.diags);
        let font: Vec<u8> = std::fs::read(concat!(env!("CARGO_MANIFEST_DIR"), "/../../assets/Fonts/Roboto Regular.ttf")).expect("the font");
        let assets: UiAssets = UiAssets::new(TextEngine::new(&[("Roboto Regular", font)]), ImageTable::from_keys(Images::from_names(std::iter::empty::<&str>())));
        let platform: PagePlatform = PagePlatform::new([0.0; 4], Orientation::from_view(SIZE.0 as f32, SIZE.1 as f32), ScreenClass::from_view(SIZE.0 as f32, SIZE.1 as f32), None);
        let mut stack: PageStack = PageStack::new();
        let id: PageId = stack.add(PageUi::new(compiled.page));
        Shown { stack, assets, platform, id, heard: Vec::new() }
    }

    fn frame(&mut self) {
        self.stack.frame(FRAME_SECONDS, SIZE, EPX, &self.platform, &self.assets);
    }

    fn page(&self) -> &PageUi {
        self.stack.get(self.id)
    }

    fn touch(&mut self, phase: Phase, x: f32, y: f32, t: f64) -> EventList {
        self.stack.touch_to(self.id, phase, x, y, t, MOUSE_ID, EPX, &self.assets)
    }

    fn hover(&mut self, x: f32, y: f32) {
        self.heard.clear();
        self.stack.hover(Some(self.id), Touch::mouse(Phase::Move, x, y, 0.0, Buttons::NONE), &mut self.heard);
    }

    /// One sample of a pointer that presses, with all a device says.
    fn pointer(&mut self, touch: Touch) -> EventList {
        self.stack.pointer_to(self.id, touch, EPX, &self.assets)
    }
}

/// Still page allocates nothing, and neither does a touch, a mouse move
/// or a frame of a moving one: a pan, a slider drag with an event per
/// step.
///
/// Needed because a page does these most of the time: an allocation in
/// one repeats every frame or every finger move, and user sees stutter on
/// a slow phone.
#[test]
fn a_still_page_and_a_moving_one_allocate_nothing() {
    let _alone = ONE_TEST_AT_A_TIME.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    let mut shown: Shown = Shown::new();
    for _ in 0..3 {
        shown.frame();
    }
    let (button, slider, scroll): (NodeId, NodeId, NodeId) = (shown.page().node("tapButton"), shown.page().node("volume"), shown.page().node("box"));

    assert_eq!(allocations_of(|| shown.frame()), 0, "a frame of a page where nothing changed");
    assert_eq!(allocations_of(|| { shown.page().node("tapButton"); }), 0, "finding a node by its name");

    // Mouse comes onto the button, then moves on it.
    let on_button: Rect = shown.page().rect(button);
    shown.hover(on_button.x + 10.0, on_button.y + 10.0);
    shown.frame();
    shown.frame();
    assert_eq!(allocations_of(|| shown.hover(on_button.x + 20.0, on_button.y + 12.0)), 0, "the mouse moves on the node it is over");
    assert_eq!(allocations_of(|| shown.frame()), 0, "the frame after that move");
    shown.stack.unhover(&mut shown.heard);

    // Mouse moves on the turned button. Page with a turned node finds
    // node under mouse through point turned back, for every move.
    let turned: NodeId = shown.page().node("turned");
    let on_turned: Rect = shown.page().rect(turned);
    let (x, y): (f32, f32) = (on_turned.x + on_turned.w / 2.0, on_turned.y + on_turned.h / 2.0);
    shown.hover(x, y);
    shown.frame();
    shown.frame();
    assert_eq!(gogo_ui::hit(&shown.page().page, shown.page().kept.frame(), x, y, false), Some(turned), "the mouse is over the turned button");
    assert_eq!(allocations_of(|| shown.hover(x + 3.0, y + 1.0)), 0, "the mouse moves on a turned node");
    assert_eq!(allocations_of(|| shown.frame()), 0, "the frame after that move");
    shown.stack.unhover(&mut shown.heard);

    // Finger pans the scrolling box.
    let in_box: Rect = shown.page().rect(scroll);
    let mut t: f64 = 1.0;
    shown.touch(Phase::Down, in_box.x + 20.0, in_box.y + 70.0, t);
    for step in 0..6 {
        t += FRAME_SECONDS as f64;
        let y: f32 = in_box.y + 60.0 - 8.0 * step as f32;
        let mut events: EventList = EventList::new();
        let for_the_touch: u64 = allocations_of(|| events = shown.touch(Phase::Move, in_box.x + 20.0, y, t));
        let for_the_frame: u64 = allocations_of(|| shown.frame());
        assert!(events.is_empty(), "a pan causes no event: {events:?}");
        if step >= 2 {
            assert!(for_the_touch == 0 && for_the_frame == 0, "step {step} of the pan: {for_the_touch} for the touch, {for_the_frame} for the frame after it");
        }
    }
    assert!(shown.page().scroll_of(scroll)[1] > 0.0, "the pan moved the text");
    shown.touch(Phase::Up, in_box.x + 20.0, in_box.y + 20.0, t + 0.02);
    for _ in 0..120 {
        shown.frame();
    }

    // Finger drags the slider.
    let on_slider: Rect = shown.page().rect(slider);
    t += 1.0;
    shown.touch(Phase::Down, on_slider.x + 20.0, on_slider.y + on_slider.h / 2.0, t);
    for step in 0..6 {
        t += FRAME_SECONDS as f64;
        let x: f32 = on_slider.x + 30.0 + 10.0 * step as f32;
        let mut events: EventList = EventList::new();
        let for_the_touch: u64 = allocations_of(|| events = shown.touch(Phase::Move, x, on_slider.y + on_slider.h / 2.0, t));
        let for_the_frame: u64 = allocations_of(|| shown.frame());
        assert_eq!(events.len(), 1, "each step of the drag changes the value: {events:?}");
        if step >= 3 {
            assert!(for_the_touch == 0 && for_the_frame == 0, "step {step} of the drag: {for_the_touch} for the touch, {for_the_frame} for the frame after it");
        }
    }
    shown.touch(Phase::Up, on_slider.x + 80.0, on_slider.y + on_slider.h / 2.0, t + 0.02);
    for _ in 0..60 {
        shown.frame();
    }

    // Finger taps the button: the tap comes back as an event.
    let (x, y): (f32, f32) = (on_button.x + 10.0, on_button.y + 10.0);
    for tap in 0..3 {
        t += 1.0;
        let for_the_down: u64 = allocations_of(|| { shown.touch(Phase::Down, x, y, t); });
        let mut events: EventList = EventList::new();
        let for_the_up: u64 = allocations_of(|| events = shown.touch(Phase::Up, x, y, t + 0.05));
        assert!(events.iter().any(|event| event.kind == EventKind::Activate), "a tap activates the button: {events:?}");
        if tap >= 1 {
            assert!(for_the_down == 0 && for_the_up == 0, "tap {tap}: {for_the_down} for the finger down, {for_the_up} for the finger up");
        }
        for _ in 0..60 {
            shown.frame();
        }
    }
}

/// A page with two boxes its code handles itself.
const PADS: &str = r#"
View page background=#202020 {
    Row halign=left, valign=top, spacing=10 {
        Panel left width=150, height=300, interactive=true, on:pointer=drawn, on:hover=aimed, on:wheel=zoomed
        Panel right width=150, height=300, interactive=true, on:pointer=drawn
    }
}
"#;

/// A sample of a pointer that presses, a mouse move over a box with
/// `on:hover=` and a wheel turn over one with `on:wheel=` allocate
/// nothing, each with the event it makes.
///
/// Needed because a phone hands several samples of a finger between two
/// frames, a brush reads every one, and a second finger doubles them: an
/// allocation in a sample repeats hundreds of times a second.
#[test]
fn a_pointer_sample_a_hover_and_a_wheel_turn_allocate_nothing() {
    let _alone = ONE_TEST_AT_A_TIME.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    let mut shown: Shown = Shown::of(PADS);
    for _ in 0..3 {
        shown.frame();
    }
    let (left, right): (Rect, Rect) = (shown.page().rect(shown.page().node("left")), shown.page().rect(shown.page().node("right")));
    let pen = |phase: Phase, x: f32, y: f32, t: f64, id: u64| Touch { phase, x, y, t, id, tool: gogo_ui::Tool::Pen, pressure: 0.3, tilt: 25.0, tilt_toward: 40.0, ..Touch::PLAIN };

    // Mouse over the box that hears it. First moves make the lists they
    // need, once.
    shown.hover(left.x + 20.0, left.y + 20.0);
    shown.hover(left.x + 22.0, left.y + 20.0);
    assert_eq!(allocations_of(|| shown.hover(left.x + 24.0, left.y + 21.0)), 0, "a mouse move over a box with on:hover");
    assert_eq!(shown.heard.len(), 1, "and the box heard it");

    // Wheel over the same box.
    drop(shown.stack.wheel_to(shown.id, left.x + 24.0, left.y + 21.0, 1.0, gogo_ui::Mods::NONE));
    assert_eq!(allocations_of(|| drop(shown.stack.wheel_to(shown.id, left.x + 24.0, left.y + 21.0, -1.0, gogo_ui::Mods::CTRL))), 0, "a wheel turn over a box with on:wheel");

    // A pen on the left box, then a finger on the right one: two presses.
    assert_eq!(shown.pointer(pen(Phase::Down, left.x + 30.0, left.y + 30.0, 1.0, 1)).len(), 1);
    drop(shown.pointer(pen(Phase::Move, left.x + 32.0, left.y + 30.0, 1.01, 1)));
    assert_eq!(allocations_of(|| drop(shown.pointer(pen(Phase::Move, left.x + 34.0, left.y + 31.0, 1.02, 1)))), 0, "a sample of the first pointer");
    assert_eq!(shown.pointer(Touch { phase: Phase::Down, x: right.x + 30.0, y: right.y + 30.0, t: 1.03, id: 2, ..Touch::PLAIN }).len(), 1, "a second finger presses the other box");
    drop(shown.pointer(Touch { phase: Phase::Move, x: right.x + 32.0, y: right.y + 30.0, t: 1.04, id: 2, ..Touch::PLAIN }));
    // Frames after the first samples take up what the mouse's leaving
    // and the two presses changed, once.
    shown.frame();
    shown.frame();
    assert_eq!(allocations_of(|| drop(shown.pointer(Touch { phase: Phase::Move, x: right.x + 34.0, y: right.y + 30.0, t: 1.05, id: 2, ..Touch::PLAIN }))), 0, "a sample of the second pointer");
    assert_eq!(allocations_of(|| drop(shown.pointer(pen(Phase::Move, left.x + 36.0, left.y + 32.0, 1.06, 1)))), 0, "a sample of the first, with two down");
    assert_eq!(allocations_of(|| shown.frame()), 0, "the frame after");
    assert_eq!(allocations_of(|| drop(shown.pointer(Touch { phase: Phase::Up, x: right.x + 34.0, y: right.y + 30.0, t: 1.07, id: 2, ..Touch::PLAIN }))), 0, "the second finger lifts");
    assert_eq!(allocations_of(|| drop(shown.pointer(pen(Phase::Up, left.x + 36.0, left.y + 32.0, 1.08, 1)))), 0, "the first pointer lifts");
}
