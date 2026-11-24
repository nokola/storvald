//! Several pointers at once, and what a pointer's sample carries.
//!
//! Each finger has its own press. A node holds one finger at a time: a
//! second finger on a held node does nothing, and a Scroll a finger pans
//! is held by that finger. Focus and the text caret stay single. A mouse
//! is one pointer with a set of buttons. `on:pointer=` hears a press
//! with the whole sample; `on:hover=` hears a pointer that presses
//! nothing; `on:wheel=` hears the wheel.
use gogo_ui::KeptFrame;
use gogo_ui::input::{Buttons, EventKind, EventList, HoverPhase, Input, Mods, Phase, PointerSample, Tool, Touch};
use gogo_ui::layout::{FixedAdvance, Rect};
use gogo_ui::page::{NodeId, Value};
use gogo_ui::runtime::{HELD, HOVERED, Runtime, resolve};
use gogo_ui::{MOUSE_ID, Page, compile};

struct Fx {
    page: Page,
    rt: Runtime,
    input: Input,
}

impl Fx {
    fn new(src: &str) -> Fx {
        let c = compile(src, &|_| None);
        assert!(!gogo_ui::has_errors(&c.diags), "{:#?}", c.diags);
        let page = c.page;
        let rt = Runtime::new(&page);
        Fx { page, rt, input: Input::new() }
    }
    fn lay(&self) -> KeptFrame {
        KeptFrame::fresh(&self.page, resolve(&self.page, &self.rt), Rect::new(0.0, 0.0, 400.0, 800.0), &FixedAdvance)
    }
    fn id(&self, name: &str) -> NodeId {
        self.page.find(name).unwrap()
    }
    fn rect(&self, name: &str) -> Rect {
        self.lay().rects()[self.id(name).index()]
    }
    fn sample(&mut self, t: Touch) -> EventList {
        let f = self.lay();
        self.input.touch(&self.page, &mut self.rt, f.frame(), &FixedAdvance, t)
    }
    fn finger(&mut self, phase: Phase, x: f32, y: f32, t: f64, id: u64) -> EventList {
        self.sample(Touch { phase, x, y, t, id, ..Touch::PLAIN })
    }
    fn mouse(&mut self, phase: Phase, x: f32, y: f32, t: f64, buttons: Buttons) -> EventList {
        self.sample(Touch::mouse(phase, x, y, t, buttons))
    }
    /// Mouse moves to (`x`, `y`) with no button down.
    fn hover(&mut self, x: f32, y: f32) -> EventList {
        let f = self.lay();
        self.input.hover(&self.page, &mut self.rt, f.frame(), x, y)
    }
    /// Mouse leaves the window.
    fn unhover(&mut self) -> EventList {
        self.input.unhover(&self.page, &mut self.rt)
    }
    fn tick(&mut self, dt: f32) -> bool {
        let f = self.lay();
        self.input.tick(&self.page, &mut self.rt, f.frame(), dt)
    }
}

fn kinds(events: &EventList) -> Vec<String> {
    events
        .iter()
        .map(|e| match e.kind {
            EventKind::Activate => "activate".to_string(),
            EventKind::Press => "press".to_string(),
            EventKind::Release => "release".to_string(),
            EventKind::Change(_) => "change".to_string(),
            EventKind::Edit(_) => "edit".to_string(),
            EventKind::Pointer(phase, _) => format!("pointer:{phase:?}"),
            EventKind::Hover(phase, _) => format!("hover:{phase:?}"),
            EventKind::Wheel { notches, .. } => format!("wheel:{notches}"),
            EventKind::Key { .. } => "key".to_string(),
            EventKind::Signal => "signal".to_string(),
        })
        .collect()
}

const TWO_BUTTONS: &str = "View v { Button b width=100, height=40, halign=left, valign=top; Button c width=100, height=40, halign=right, valign=top }";

/// Two fingers on two buttons: both are held, and each button activates
/// when its own finger lifts.
#[test]
fn two_fingers_press_two_buttons_at_once() {
    let mut fx = Fx::new(TWO_BUTTONS);
    let (b, c) = (fx.id("b"), fx.id("c"));
    fx.finger(Phase::Down, 10.0, 10.0, 0.0, 1);
    fx.finger(Phase::Down, 390.0, 10.0, 0.1, 2);
    assert!(fx.rt.has(b, HELD) && fx.rt.has(c, HELD), "each finger holds its own button");
    let lifted_c = fx.finger(Phase::Up, 390.0, 10.0, 0.2, 2);
    assert_eq!(kinds(&lifted_c), ["activate"]);
    assert_eq!(lifted_c[0].node, c);
    assert!(fx.rt.has(b, HELD) && !fx.rt.has(c, HELD), "the other finger still holds its button");
    let lifted_b = fx.finger(Phase::Up, 10.0, 10.0, 0.3, 1);
    assert_eq!(kinds(&lifted_b), ["activate"]);
    assert_eq!(lifted_b[0].node, b);
    assert!(!fx.input.is_pressed());
}

/// A finger drags a slider while another taps a button: the slider keeps
/// its drag, the button activates.
#[test]
fn a_slider_drag_goes_on_while_another_finger_taps() {
    let mut fx = Fx::new("View v { Column col halign=left, valign=top { Slider s width=300, height=40, min=0, max=100, value=0; Button b width=100, height=40 } }");
    let slider: Rect = fx.rect("s");
    let button: Rect = fx.rect("b");
    let y: f32 = slider.y + slider.h / 2.0;
    fx.finger(Phase::Down, slider.x + 10.0, y, 0.0, 1);
    fx.finger(Phase::Move, slider.x + 100.0, y, 0.1, 1);
    let tapped = {
        let mut events = fx.finger(Phase::Down, button.x + 10.0, button.y + 10.0, 0.2, 2);
        events.extend(fx.finger(Phase::Up, button.x + 10.0, button.y + 10.0, 0.3, 2));
        events
    };
    assert_eq!(kinds(&tapped), ["activate"]);
    let moved = fx.finger(Phase::Move, slider.x + 200.0, y, 0.4, 1);
    assert_eq!(kinds(&moved), ["edit"], "the drag goes on after the other finger's tap");
    let lifted = fx.finger(Phase::Up, slider.x + 200.0, y, 0.5, 1);
    assert_eq!(kinds(&lifted), ["change"]);
}

const LIST: &str = "View v { Scroll s width=200, height=200, halign=left, valign=top { Column c { Button a width=200, height=100; Button b width=200, height=100; Button d width=200, height=100; Button e width=200, height=100 } } }";

/// A Scroll that a finger pans is held by that finger: a second finger on
/// a button inside it presses that button and cannot take the Scroll.
#[test]
fn a_panned_scroll_is_not_taken_by_a_second_finger() {
    let mut fx = Fx::new(LIST);
    let (s, b) = (fx.id("s"), fx.id("b"));
    fx.finger(Phase::Down, 50.0, 50.0, 0.0, 1);
    fx.finger(Phase::Move, 50.0, 20.0, 0.02, 1);
    let panned: f32 = fx.rt.scroll[s][1];
    assert!(panned > 0.0, "the first finger pans");
    // Button `b` is at 100 to 200 of the content, so at 100 - panned on screen.
    let on_b: f32 = 150.0 - panned;
    fx.finger(Phase::Down, 150.0, on_b, 0.03, 2);
    assert!(fx.rt.has(b, HELD), "the second finger presses the button under it");
    fx.finger(Phase::Move, 150.0, on_b - 40.0, 0.04, 2);
    assert_eq!(fx.rt.scroll[s][1], panned, "its drag does not move the Scroll the first finger pans");
    fx.finger(Phase::Move, 50.0, 10.0, 0.05, 1);
    assert_eq!(fx.rt.scroll[s][1], panned + 10.0, "the first finger still pans");
}

/// Two lists flicked by two fingers both glide: one glide per Scroll.
#[test]
fn two_scrolls_glide_at_once() {
    let rows = "Button width=150, height=100; Button width=150, height=100; Button width=150, height=100; Button width=150, height=100; Button width=150, height=100; Button width=150, height=100";
    let mut fx = Fx::new(&format!("View v {{ Scroll left width=150, height=200, halign=left, valign=top {{ Column {{ {rows} }} }}; Scroll right width=150, height=200, halign=right, valign=top {{ Column {{ {rows} }} }} }}"));
    let (left, right) = (fx.id("left"), fx.id("right"));
    for (finger, x) in [(1u64, 50.0f32), (2, 350.0)] {
        fx.finger(Phase::Down, x, 150.0, 0.0, finger);
        fx.finger(Phase::Move, x, 120.0, 0.01, finger);
        fx.finger(Phase::Move, x, 80.0, 0.02, finger);
        fx.finger(Phase::Up, x, 60.0, 0.03, finger);
    }
    let before: ([f32; 2], [f32; 2]) = (fx.rt.scroll[left], fx.rt.scroll[right]);
    assert!(fx.tick(0.016), "something glides");
    assert!(fx.rt.scroll[left][1] > before.0[1], "the first list glides on");
    assert!(fx.rt.scroll[right][1] > before.1[1], "and so does the second");
    // A press on one stops that one only.
    fx.finger(Phase::Down, 50.0, 100.0, 0.1, 1);
    let held: f32 = fx.rt.scroll[left][1];
    let other: f32 = fx.rt.scroll[right][1];
    fx.tick(0.016);
    assert_eq!(fx.rt.scroll[left][1], held, "the caught list stands");
    assert!(fx.rt.scroll[right][1] > other, "the other still glides");
}

const PAD: &str = "View v { Layers l { Panel pad width=200, height=200, halign=left, valign=top, margin_left=20, margin_top=30, interactive=true, on:pointer=track, on:hover=over, on:wheel=turned; Button b width=100, height=40, halign=right, valign=bottom } }";

/// Phase and sample of the first `Pointer` event.
fn pointer_of(events: &EventList) -> (Phase, PointerSample) {
    events.iter().find_map(|e| if let EventKind::Pointer(phase, sample) = e.kind { Some((phase, sample)) } else { None }).unwrap_or_else(|| panic!("no pointer event in {events:?}"))
}

/// Phase and sample of the first `Hover` event.
fn hover_of(events: &EventList) -> (HoverPhase, PointerSample) {
    events.iter().find_map(|e| if let EventKind::Hover(phase, sample) = e.kind { Some((phase, sample)) } else { None }).unwrap_or_else(|| panic!("no hover event in {events:?}"))
}

/// `on:pointer=` hands on everything the host said about the sample, the
/// place in the box's own space, and the move since the last sample.
#[test]
fn a_pointer_sample_carries_what_the_device_said() {
    let mut fx = Fx::new(PAD);
    let pen = Touch { phase: Phase::Down, x: 70.0, y: 90.0, t: 2.5, id: 7, tool: Tool::Pen, buttons: Buttons::PRIMARY.with(Buttons::SECONDARY), pressure: 0.25, tilt: 30.0, tilt_toward: 135.0, mods: Mods::SHIFT };
    let (phase, down) = pointer_of(&fx.sample(pen));
    assert_eq!((phase, down.x, down.y, down.dx, down.dy), (Phase::Down, 50.0, 60.0, 0.0, 0.0), "place in the box's own space");
    assert_eq!((down.finger, down.tool, down.pressure, down.tilt, down.tilt_toward, down.time), (7, Tool::Pen, 0.25, 30.0, 135.0, 2.5));
    assert!(down.buttons.has(Buttons::SECONDARY) && down.buttons.has(Buttons::PRIMARY) && !down.buttons.has(Buttons::MIDDLE));
    assert_eq!(down.mods, Mods::SHIFT);
    let (phase, moved) = pointer_of(&fx.sample(Touch { phase: Phase::Move, x: 75.0, y: 88.0, t: 2.6, pressure: 0.5, ..pen }));
    assert_eq!((phase, moved.dx, moved.dy, moved.pressure, moved.time), (Phase::Move, 5.0, -2.0, 0.5, 2.6));
    let (phase, lifted) = pointer_of(&fx.sample(Touch { phase: Phase::Up, x: 300.0, y: 500.0, t: 2.7, ..pen }));
    assert_eq!((phase, lifted.x, lifted.y), (Phase::Up, 280.0, 470.0), "the pointer stays with the box wherever it lifts");
}

/// A second finger on an `on:pointer=` box that is held does nothing.
#[test]
fn a_second_finger_on_a_held_pointer_box_does_nothing() {
    let mut fx = Fx::new(PAD);
    assert_eq!(kinds(&fx.finger(Phase::Down, 70.0, 90.0, 0.0, 1)), ["pointer:Down"]);
    assert!(fx.finger(Phase::Down, 100.0, 100.0, 0.1, 2).is_empty());
    assert!(fx.finger(Phase::Move, 110.0, 100.0, 0.2, 2).is_empty());
    assert!(fx.finger(Phase::Up, 110.0, 100.0, 0.3, 2).is_empty());
    assert_eq!(kinds(&fx.finger(Phase::Move, 80.0, 90.0, 0.4, 1)), ["pointer:Move"]);
    assert_eq!(kinds(&fx.finger(Phase::Up, 80.0, 90.0, 0.5, 1)), ["pointer:Up"]);
}

/// The mouse is one pointer with a set of buttons: its press starts with
/// the first button and ends with the last; a button in between is a move
/// with another set. A button other than the primary one presses only an
/// `on:pointer=` box.
#[test]
fn a_mouse_is_one_pointer_with_buttons() {
    let mut fx = Fx::new(PAD);
    let b = fx.id("b");
    let button: Rect = fx.rect("b");
    // Middle button on a Button: nothing.
    assert!(fx.mouse(Phase::Down, button.x + 10.0, button.y + 10.0, 0.0, Buttons::MIDDLE).is_empty());
    assert!(!fx.rt.has(b, HELD) && !fx.input.is_pressed(), "a Button reacts to the primary button alone");
    assert!(fx.mouse(Phase::Up, button.x + 10.0, button.y + 10.0, 0.1, Buttons::NONE).is_empty());
    // Middle button on the pad: its press starts.
    let (phase, down) = pointer_of(&fx.mouse(Phase::Down, 70.0, 90.0, 1.0, Buttons::MIDDLE));
    assert_eq!((phase, down.finger, down.tool), (Phase::Down, MOUSE_ID, Tool::Mouse));
    assert!(down.buttons.has(Buttons::MIDDLE) && !down.buttons.has(Buttons::PRIMARY));
    // The left button goes down too: a move with both, not a second press.
    let (phase, both) = pointer_of(&fx.mouse(Phase::Down, 72.0, 90.0, 1.1, Buttons::MIDDLE.with(Buttons::PRIMARY)));
    assert_eq!(phase, Phase::Move, "a second button is a move of the same press");
    assert!(both.buttons.has(Buttons::MIDDLE.with(Buttons::PRIMARY)));
    let (_, one) = pointer_of(&fx.mouse(Phase::Move, 74.0, 90.0, 1.2, Buttons::PRIMARY));
    assert!(!one.buttons.has(Buttons::MIDDLE) && one.buttons.has(Buttons::PRIMARY), "the middle button lifted");
    assert_eq!(kinds(&fx.mouse(Phase::Up, 74.0, 90.0, 1.3, Buttons::NONE)), ["pointer:Up"]);
    assert!(!fx.input.is_pressed());
}

/// `on:hover=` hears the mouse over its box with no button down, with the
/// move since the last sample, and `Leave` when the mouse is over another
/// box or gone. A press freezes it. A box without the word hears nothing.
#[test]
fn hover_reaches_the_box_that_asks_for_it() {
    let mut fx = Fx::new(PAD);
    let pad = fx.id("pad");
    let button: Rect = fx.rect("b");
    let (phase, first) = hover_of(&fx.hover(70.0, 90.0));
    assert_eq!((phase, first.x, first.y, first.dx, first.dy), (HoverPhase::Over, 50.0, 60.0, 0.0, 0.0));
    assert!(first.buttons.is_none() && fx.rt.has(pad, HOVERED));
    let (_, second) = hover_of(&fx.hover(73.0, 95.0));
    assert_eq!((second.dx, second.dy), (3.0, 5.0));
    // A press freezes hover: the moves are the press's.
    fx.mouse(Phase::Down, 73.0, 95.0, 0.2, Buttons::PRIMARY);
    assert!(fx.hover(80.0, 95.0).is_empty(), "no hover while a pointer presses");
    fx.mouse(Phase::Up, 73.0, 95.0, 0.4, Buttons::NONE);
    // Over the Button: the pad hears that the mouse left, the Button hears nothing.
    let over_button = fx.hover(button.x + 10.0, button.y + 10.0);
    assert_eq!(kinds(&over_button), ["hover:Left"]);
    assert_eq!(over_button[0].node, pad);
    assert!(fx.hover(button.x + 12.0, button.y + 10.0).is_empty(), "a box without the word hears nothing");
    // Back over the pad, then off the window.
    assert_eq!(kinds(&fx.hover(70.0, 90.0)), ["hover:Over"]);
    let gone = fx.unhover();
    assert_eq!(kinds(&gone), ["hover:Left"]);
    assert!(!fx.rt.has(pad, HOVERED));
    assert!(fx.unhover().is_empty(), "it left once");
}

/// `on:wheel=` hears the wheel over its box, with the place, the notches
/// and the keys held. A Slider under the mouse still steps by the wheel.
#[test]
fn the_wheel_reaches_the_box_that_asks_for_it() {
    let mut fx = Fx::new("View v { Column col halign=left, valign=top { Panel pad width=200, height=100, interactive=true, on:wheel=turned { Label text=\"zoom here\" }; Slider s width=200, height=40, min=0, max=10, value=5, step=1; Panel plain width=200, height=100 } }");
    let (pad, s) = (fx.id("pad"), fx.id("s"));
    let f = fx.lay();
    let turned = fx.input.wheel(&fx.page, &mut fx.rt, f.frame(), 30.0, 40.0, -2.0, Mods::CTRL);
    assert_eq!(turned.len(), 1);
    assert_eq!(turned[0].node, pad);
    assert_eq!(turned[0].kind, EventKind::Wheel { x: 30.0, y: 40.0, notches: -2.0, mods: Mods::CTRL });
    let slider: Rect = fx.rect("s");
    let stepped = fx.input.wheel(&fx.page, &mut fx.rt, f.frame(), slider.x + 20.0, slider.y + 20.0, 1.0, Mods::NONE);
    assert_eq!(stepped.len(), 1);
    assert!(stepped[0].node == s && stepped[0].kind == EventKind::Change(Value::VNum(6.0)), "a Slider takes the wheel as before: {stepped:?}");
    let plain: Rect = fx.rect("plain");
    assert!(fx.input.wheel(&fx.page, &mut fx.rt, f.frame(), plain.x + 20.0, plain.y + 20.0, 1.0, Mods::NONE).is_empty(), "a box without the word takes no wheel");
}

/// Two words on one node may name the same handler: the event's kind
/// says which one it was.
#[test]
fn two_words_of_a_box_may_name_one_handler() {
    let mut fx = Fx::new("View v { Panel pad width=200, height=200, halign=left, valign=top, interactive=true, on:pointer=both, on:hover=both }");
    let hovered = fx.hover(50.0, 50.0);
    let pressed = fx.mouse(Phase::Down, 50.0, 50.0, 0.1, Buttons::PRIMARY);
    assert_eq!((kinds(&hovered), kinds(&pressed)), (vec!["hover:Over".to_string()], vec!["pointer:Down".to_string()]));
    assert_eq!(hovered[0].action, pressed[0].action, "one handler for both");
    assert!(hovered[0].action.is_some());
}

/// A finger that lifts while the mouse hovers over another box moves no
/// hover: a finger is not the pointer that hovers.
#[test]
fn a_finger_that_lifts_moves_no_hover() {
    let mut fx = Fx::new(TWO_BUTTONS);
    let (b, c) = (fx.id("b"), fx.id("c"));
    fx.hover(10.0, 10.0);
    assert!(fx.rt.has(b, HOVERED));
    fx.finger(Phase::Down, 390.0, 10.0, 0.1, 1);
    fx.finger(Phase::Up, 390.0, 10.0, 0.2, 1);
    assert!(fx.rt.has(b, HOVERED) && !fx.rt.has(c, HOVERED), "the mouse still hovers where it is");
    // The mouse's own lift does check where it is.
    fx.mouse(Phase::Down, 10.0, 10.0, 0.3, Buttons::PRIMARY);
    fx.mouse(Phase::Up, 390.0, 10.0, 0.4, Buttons::NONE);
    assert!(!fx.rt.has(b, HOVERED) && fx.rt.has(c, HOVERED), "lifted over the other button: hover follows the mouse");
}
