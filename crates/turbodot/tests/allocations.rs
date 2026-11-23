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
use gogo_ui::{Event, Images, MOUSE_ID, NodeId, Orientation, Phase, Rect, ScreenClass, compile, has_errors};
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
        Scroll box width=200, height=80 {
            Label long text="one two three four five six seven eight nine ten one two three four five six seven eight nine ten one two three four five six seven eight nine ten one two three four five six seven eight nine ten one two three four five six seven eight nine ten one two three four five six seven eight nine ten one two three four five six seven eight nine ten one two three four five six seven eight nine ten", wrap=true, width=fill, font_color=#FFFFFF
        }
    }
}
"#;

const SIZE: (u32, u32) = (400, 400);
const EPX: f32 = 1.0;
const FRAME_SECONDS: f32 = 0.016;

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
}

impl Shown {
    fn new() -> Shown {
        let compiled = compile(PAGE, &|_| None);
        assert!(!has_errors(&compiled.diags), "{:?}", compiled.diags);
        let font: Vec<u8> = std::fs::read(concat!(env!("CARGO_MANIFEST_DIR"), "/../../assets/Fonts/Roboto Regular.ttf")).expect("the font");
        let assets: UiAssets = UiAssets::new(TextEngine::new(&[("Roboto Regular", font)]), ImageTable::from_keys(Images::from_names(std::iter::empty::<&str>())));
        let platform: PagePlatform = PagePlatform::new([0.0; 4], Orientation::from_view(SIZE.0 as f32, SIZE.1 as f32), ScreenClass::from_view(SIZE.0 as f32, SIZE.1 as f32), None);
        let mut stack: PageStack = PageStack::new();
        let id: PageId = stack.add(PageUi::new(compiled.page));
        Shown { stack, assets, platform, id }
    }

    fn frame(&mut self) {
        self.stack.frame(FRAME_SECONDS, SIZE, EPX, &self.platform, &self.assets);
    }

    fn page(&self) -> &PageUi {
        self.stack.get(self.id)
    }

    fn touch(&mut self, phase: Phase, x: f32, y: f32, t: f64) -> Vec<Event> {
        self.stack.touch_to(self.id, phase, x, y, t, MOUSE_ID, EPX, &self.assets)
    }

    fn hover(&mut self, x: f32, y: f32) {
        self.stack.hover(Some(self.id), x, y);
    }
}

/// Still page allocates nothing, and the page stack adds no allocation of
/// its own to a touch, a mouse move or a frame.
///
/// Needed because a page does these most of the time: an allocation in
/// one repeats every frame or every finger move, and user sees stutter on
/// a slow phone.
///
/// One thing allocates inside gogo today; check holds its count, so one
/// more shows: a touch that causes an event, once. Event comes back in a
/// new list (`gogo_ui::Input::touch`). Touch that causes none returns an
/// empty list, which allocates nothing.
#[test]
fn a_still_page_and_a_moving_one_allocate_nothing_and_an_event_costs_one() {
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
    shown.stack.unhover();

    // Finger pans the scrolling box.
    let in_box: Rect = shown.page().rect(scroll);
    let mut t: f64 = 1.0;
    shown.touch(Phase::Down, in_box.x + 20.0, in_box.y + 70.0, t);
    for step in 0..6 {
        t += FRAME_SECONDS as f64;
        let y: f32 = in_box.y + 60.0 - 8.0 * step as f32;
        let mut events: Vec<Event> = Vec::new();
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
        let mut events: Vec<Event> = Vec::new();
        let for_the_touch: u64 = allocations_of(|| events = shown.touch(Phase::Move, x, on_slider.y + on_slider.h / 2.0, t));
        let for_the_frame: u64 = allocations_of(|| shown.frame());
        assert_eq!(events.len(), 1, "each step of the drag changes the value: {events:?}");
        if step >= 3 {
            assert!(for_the_touch <= 1 && for_the_frame == 0, "step {step} of the drag: {for_the_touch} for the touch, {for_the_frame} for the frame after it");
        }
    }
}
