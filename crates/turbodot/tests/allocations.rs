//! How many heap allocations a page costs while it runs, counted by
//! `turbodot::alloc_count`.
//!
//! One test in this file, on purpose: the counts are for the whole process,
//! and cargo runs the tests of one file at the same time.
//!
//! Not in the crates.io package (`exclude` in Cargo.toml): it reads a font
//! from `assets/Fonts` at the top of the repository, which a package does
//! not carry.

use gogo_ui::{Event, NodeId, Phase, Rect};
use turbodot::alloc_count::{self, CountingAlloc};
use turbodot::{PageSource, Ui};

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

const WIDTH: f32 = 400.0;
const HEIGHT: f32 = 400.0;
const FRAME_SECONDS: f32 = 0.016;

fn font() -> Vec<u8> {
    std::fs::read(concat!(env!("CARGO_MANIFEST_DIR"), "/../../assets/Fonts/Roboto Regular.ttf")).expect("the font")
}

/// The allocations `work` makes.
fn allocations_of(work: impl FnOnce()) -> u64 {
    let before: u64 = alloc_count::allocations();
    work();
    alloc_count::allocations() - before
}

/// A page that stands still allocates nothing, and `Ui` adds no allocation
/// of its own to a touch, a mouse move or a frame.
///
/// Needed because these are what a page does most of the time, and an
/// allocation in any of them repeats at every frame or every move of a
/// finger: the user sees it as a stutter on a slow phone.
///
/// One thing allocates inside gogo today, and the check holds it at the
/// count it has, so one more shows: a touch that causes an event, once.
/// The event comes back in a new list (`gogo_ui::Input::touch`). A touch
/// that causes none returns an empty list, which allocates nothing.
#[test]
fn a_still_page_and_a_moving_one_allocate_nothing_and_an_event_costs_one() {
    let mut ui: Ui = Ui::new(PageSource::Text(PAGE), &[("Roboto Regular", font())]).expect("the page compiles");
    for _ in 0..3 {
        ui.frame(WIDTH, HEIGHT, FRAME_SECONDS);
    }
    let (button, slider, scroll): (NodeId, NodeId, NodeId) = (ui.node("tapButton"), ui.node("volume"), ui.node("box"));

    assert_eq!(allocations_of(|| { ui.frame(WIDTH, HEIGHT, FRAME_SECONDS); }), 0, "a frame of a page where nothing changed");
    assert_eq!(allocations_of(|| { ui.node("tapButton"); }), 0, "finding a node by its name");

    // The mouse comes onto the button, then moves on it.
    let on_button: Rect = ui.rect(button);
    ui.hover(on_button.x + 10.0, on_button.y + 10.0);
    ui.frame(WIDTH, HEIGHT, FRAME_SECONDS);
    ui.frame(WIDTH, HEIGHT, FRAME_SECONDS);
    assert_eq!(allocations_of(|| ui.hover(on_button.x + 20.0, on_button.y + 12.0)), 0, "the mouse moves on the node it is over");
    assert_eq!(allocations_of(|| { ui.frame(WIDTH, HEIGHT, FRAME_SECONDS); }), 0, "the frame after that move");
    ui.unhover();

    // A finger pans the scrolling box.
    let in_box: Rect = ui.rect(scroll);
    let mut t: f64 = 1.0;
    ui.touch(Phase::Down, in_box.x + 20.0, in_box.y + 70.0, t);
    for step in 0..6 {
        t += FRAME_SECONDS as f64;
        let y: f32 = in_box.y + 60.0 - 8.0 * step as f32;
        let mut events: Vec<Event> = Vec::new();
        let for_the_touch: u64 = allocations_of(|| events = ui.touch(Phase::Move, in_box.x + 20.0, y, t));
        let for_the_frame: u64 = allocations_of(|| { ui.frame(WIDTH, HEIGHT, FRAME_SECONDS); });
        assert!(events.is_empty(), "a pan causes no event: {events:?}");
        if step >= 2 {
            assert!(for_the_touch == 0 && for_the_frame == 0, "step {step} of the pan: {for_the_touch} for the touch, {for_the_frame} for the frame after it");
        }
    }
    assert!(ui.scroll_of(scroll)[1] > 0.0, "the pan moved the text");
    ui.touch(Phase::Up, in_box.x + 20.0, in_box.y + 20.0, t + 0.02);
    for _ in 0..120 {
        ui.frame(WIDTH, HEIGHT, FRAME_SECONDS);
    }

    // A finger drags the slider.
    let on_slider: Rect = ui.rect(slider);
    t += 1.0;
    ui.touch(Phase::Down, on_slider.x + 20.0, on_slider.y + on_slider.h / 2.0, t);
    for step in 0..6 {
        t += FRAME_SECONDS as f64;
        let x: f32 = on_slider.x + 30.0 + 10.0 * step as f32;
        let mut events: Vec<Event> = Vec::new();
        let for_the_touch: u64 = allocations_of(|| events = ui.touch(Phase::Move, x, on_slider.y + on_slider.h / 2.0, t));
        let for_the_frame: u64 = allocations_of(|| { ui.frame(WIDTH, HEIGHT, FRAME_SECONDS); });
        assert_eq!(events.len(), 1, "each step of the drag changes the value: {events:?}");
        if step >= 3 {
            assert!(for_the_touch <= 1 && for_the_frame == 0, "step {step} of the drag: {for_the_touch} for the touch, {for_the_frame} for the frame after it");
        }
    }
}
