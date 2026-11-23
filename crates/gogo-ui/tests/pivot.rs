//! `pivot=` — the 0..1 box point `scale` and `rotate` transform about
//! (2026-09-02, the Godot 4.7 offset-transforms comparison round). Unset
//! = the center, exactly the old behavior; `0x0` holds the top-left
//! still. One fixed point serves the node's own rect, its subtree's
//! shift, the rotation center AND the hit walk's moved rects — look and
//! touch agree by construction.

use gogo_ui::compile;
use gogo_ui::KeptFrame;
use gogo_ui::draw::{Draw, draw, hit};
use gogo_ui::layout::{FixedAdvance, Rect};
use gogo_ui::page::{Color_Straight_Srgb_4xU8_RGBA, PropKey, Value};
use gogo_ui::runtime::{Runtime, resolve};
use gogo_ui::{ArrangementId, NodeId, Page};

fn fails(src: &str) -> bool {
    gogo_ui::has_errors(&compile(src, &|_| None).diags)
}

/// The plate quad drawn for the node filled `color` (the fixture's way to
/// find one box in the draw list).
fn quad_of(draws: &[Draw], color: Color_Straight_Srgb_4xU8_RGBA) -> Rect {
    draws
        .iter()
        .find_map(|d| match d {
            Draw::Quad { rect, color: c, .. } if *c == color => Some(*rect),
            _ => None,
        })
        .expect("the box's quad")
}

/// `pivot=0x0` under `scale=2`: the top-left corner holds still — the
/// box grows right and down, and a child in the corner scales in place.
/// Without a `pivot=`, the same scale spreads about the center (the old
/// behavior, unchanged).
#[test]
fn scale_holds_the_pivot_point_still() {
    let c = compile("View v { Panel p width=100, height=50, halign=left, valign=top, background=#FF0000FF, pivot=0x0 { Rectangle c 10x10, halign=left, valign=top, color=#00FF00FF } }", &|_| None);
    assert!(!gogo_ui::has_errors(&c.diags), "{:#?}", c.diags);
    let page = c.page;
    let p = page.find("p").unwrap();
    let mut rt = Runtime::new(&page);
    rt.set_prop(p, PropKey::Scale, Value::VNum(2.0));
    let f = KeptFrame::fresh(&page, resolve(&page, &rt), Rect::new(0.0, 0.0, 400.0, 800.0), &FixedAdvance);
    let draws = draw(&page, f.frame());
    assert_eq!(quad_of(&draws, Color_Straight_Srgb_4xU8_RGBA([255, 0, 0, 255])), Rect::new(0.0, 0.0, 200.0, 100.0), "the top-left corner held");
    assert_eq!(quad_of(&draws, Color_Straight_Srgb_4xU8_RGBA([0, 255, 0, 255])), Rect::new(0.0, 0.0, 20.0, 20.0), "the corner child scales in place");

    // The same page WITHOUT pivot: the center holds (the old law).
    let c = compile("View v { Panel p width=100, height=50, halign=left, valign=top, background=#FF0000FF }", &|_| None);
    assert!(!gogo_ui::has_errors(&c.diags), "{:#?}", c.diags);
    let page = c.page;
    let p = page.find("p").unwrap();
    let mut rt = Runtime::new(&page);
    rt.set_prop(p, PropKey::Scale, Value::VNum(2.0));
    let f = KeptFrame::fresh(&page, resolve(&page, &rt), Rect::new(0.0, 0.0, 400.0, 800.0), &FixedAdvance);
    let draws = draw(&page, f.frame());
    assert_eq!(quad_of(&draws, Color_Straight_Srgb_4xU8_RGBA([255, 0, 0, 255])), Rect::new(-50.0, -25.0, 200.0, 100.0), "no pivot = about the center");
}

/// `rotate` turns about the pivot: `1x1` puts the rotation center on the
/// bottom-right corner (a door swinging on its hinge).
#[test]
fn rotate_turns_about_the_pivot() {
    let c = compile("View v { Panel p width=100, height=50, halign=left, valign=top, background=#FF0000FF, pivot=1x1 }", &|_| None);
    assert!(!gogo_ui::has_errors(&c.diags), "{:#?}", c.diags);
    let page = c.page;
    let p = page.find("p").unwrap();
    let mut rt = Runtime::new(&page);
    rt.set_prop(p, PropKey::Rotate, Value::VNum(30.0));
    let f = KeptFrame::fresh(&page, resolve(&page, &rt), Rect::new(0.0, 0.0, 400.0, 800.0), &FixedAdvance);
    let draws = draw(&page, f.frame());
    let (cx, cy) = draws
        .iter()
        .find_map(|d| match d {
            Draw::RotatePush { cx, cy, .. } => Some((*cx, *cy)),
            _ => None,
        })
        .expect("a rotate bracket");
    assert_eq!((cx, cy), (100.0, 50.0), "the hinge is the bottom-right corner");
}

/// The finger lands on the pivoted picture: a button scaled 2x about its
/// top-left takes a tap in the grown half the un-pivoted box never
/// covered — `hit` reads the same fixed point the draw walk holds.
#[test]
fn hit_follows_the_pivot() {
    let c = compile("View v { Button b text=\"x\", width=100, height=50, halign=left, valign=top, pivot=0x0 }", &|_| None);
    assert!(!gogo_ui::has_errors(&c.diags), "{:#?}", c.diags);
    let page = c.page;
    let b = page.find("b").unwrap();
    let mut rt = Runtime::new(&page);
    rt.set_prop(b, PropKey::Scale, Value::VNum(2.0));
    let f = KeptFrame::fresh(&page, resolve(&page, &rt), Rect::new(0.0, 0.0, 400.0, 800.0), &FixedAdvance);
    assert_eq!(hit(&page, f.frame(), 150.0, 75.0, false), Some(b), "the grown half takes the tap");
}

/// The finger lands on the ROTATED picture too: a button turned 90
/// degrees about its center takes a tap in the space its turned box
/// covers and the laid-out box never did — and refuses one inside the
/// laid-out footprint the turned box has left. The renderer applies the
/// innermost rotate bracket per quad; `hit` un-rotates the finger about
/// the same point, so look and touch agree.
#[test]
fn hit_follows_rotation() {
    let c = compile("View v { Button b text=\"x\", width=100, height=50, halign=left, valign=top }", &|_| None);
    assert!(!gogo_ui::has_errors(&c.diags), "{:#?}", c.diags);
    let page = c.page;
    let b = page.find("b").unwrap();
    let mut rt = Runtime::new(&page);
    rt.set_prop(b, PropKey::Rotate, Value::VNum(90.0));
    let f = KeptFrame::fresh(&page, resolve(&page, &rt), Rect::new(0.0, 0.0, 400.0, 800.0), &FixedAdvance);
    // Turned 90 deg about the center (50, 25): the picture stands tall,
    // x in 25..75, y in -25..75.
    assert_eq!(hit(&page, f.frame(), 60.0, 65.0, false), Some(b), "inside the turned box, below the laid-out one");
    assert_eq!(hit(&page, f.frame(), 95.0, 45.0, false), None, "the laid-out corner the turned box left");

    // A pivoted rotation: the hinge is the top-left, the box swings so
    // its former top edge runs DOWN the y axis.
    let c = compile("View v { Button b text=\"x\", width=100, height=50, halign=left, valign=top, pivot=0x0 }", &|_| None);
    assert!(!gogo_ui::has_errors(&c.diags), "{:#?}", c.diags);
    let page = c.page;
    let b = page.find("b").unwrap();
    let mut rt = Runtime::new(&page);
    rt.set_prop(b, PropKey::Rotate, Value::VNum(90.0));
    let f = KeptFrame::fresh(&page, resolve(&page, &rt), Rect::new(0.0, 0.0, 400.0, 800.0), &FixedAdvance);
    assert_eq!(hit(&page, f.frame(), -25.0, 50.0, false), Some(b), "swung across the hinge");
    assert_eq!(hit(&page, f.frame(), 60.0, 25.0, false), None, "the old footprint is empty");
}

/// Screen point where a quarter turn of `turned` (90 degrees about its
/// center) draws the point 9/10 along and half way down `node`'s
/// laid-out rectangle. Panics when that screen point is inside `node`'s
/// laid-out rectangle too: a hit test that forgot the turn would pass.
fn drawn_point_after_quarter_turn(f: &KeptFrame, node: NodeId, turned: NodeId) -> (f32, f32) {
    let (own, about): (Rect, Rect) = (f.frame().rects()[node], f.frame().rects()[turned]);
    let (cx, cy): (f32, f32) = (about.x + about.w / 2.0, about.y + about.h / 2.0);
    let (lx, ly): (f32, f32) = (own.x + own.w * 0.9, own.y + own.h / 2.0);
    let point: (f32, f32) = (cx - (ly - cy), cy + (lx - cx));
    assert!(!own.contains(point.0, point.1), "{point:?} is inside the laid-out {own:?}");
    point
}

/// Button inside a turned Panel takes a tap where the turned picture
/// shows it, and none in the laid-out corner the picture left.
///
/// Needed because renderer turns a node's whole subtree with it: without
/// it, user taps the button they see and nothing happens.
#[test]
fn a_node_inside_a_turned_node_takes_the_tap_where_it_is_drawn() {
    let c = compile("View v { Panel p width=100, height=50, halign=left, valign=top { Button b text=\"x\", width=100, height=50 } }", &|_| None);
    assert!(!gogo_ui::has_errors(&c.diags), "{:#?}", c.diags);
    let page: Page = c.page;
    let (p, b): (NodeId, NodeId) = (page.find("p").unwrap(), page.find("b").unwrap());
    let mut rt = Runtime::new(&page);
    rt.set_prop(p, PropKey::Rotate, Value::VNum(90.0));
    let f = KeptFrame::fresh(&page, resolve(&page, &rt), Rect::new(0.0, 0.0, 400.0, 800.0), &FixedAdvance);
    let (x, y): (f32, f32) = drawn_point_after_quarter_turn(&f, b, p);
    assert_eq!(hit(&page, f.frame(), x, y, false), Some(b), "inside the turned picture of the button");
    assert_eq!(hit(&page, f.frame(), 95.0, 45.0, false), None, "the laid-out corner the turned picture left");
}

/// Button seated under a turned Panel takes a tap where the turned
/// picture shows it. Button is written BEFORE the Panel, so its number
/// is lower than its seat's.
///
/// Needed because a seated node turns with the node it sits in, not with
/// the one that wrote it, and a hit test that goes by node numbers reads
/// the seat's point before working it out: without it, user taps the
/// seated button they see and nothing happens.
#[test]
fn a_node_seated_under_a_turned_node_takes_the_tap_where_it_is_drawn() {
    let c = compile("arrangement moved\nwhen moved { b.seat = socket }\nView v { Button b text=\"x\", width=100, height=50, halign=right, valign=bottom\n Panel p width=100, height=50, halign=left, valign=top { seat socket } }", &|_| None);
    assert!(!gogo_ui::has_errors(&c.diags), "{:#?}", c.diags);
    let page: Page = c.page;
    let (p, b, socket): (NodeId, NodeId, NodeId) = (page.find("p").unwrap(), page.find("b").unwrap(), page.find("socket").unwrap());
    assert!(b.index() < socket.index(), "the button's number is lower than its seat's");
    let mut rt = Runtime::new(&page);
    rt.go_instant_id(ArrangementId::new(0));
    rt.set_prop(p, PropKey::Rotate, Value::VNum(90.0));
    let f = KeptFrame::fresh(&page, resolve(&page, &rt), Rect::new(0.0, 0.0, 400.0, 800.0), &FixedAdvance);
    assert!(f.frame().rects()[p].contains(f.frame().rects()[b].x + 1.0, f.frame().rects()[b].y + 1.0), "the button sits in the Panel: {:?} in {:?}", f.frame().rects()[b], f.frame().rects()[p]);
    let (x, y): (f32, f32) = drawn_point_after_quarter_turn(&f, b, p);
    assert_eq!(hit(&page, f.frame(), x, y, false), Some(b), "inside the turned picture of the seated button");
}

/// The fences: a full 0..1 pair or nothing — a bare number, a half pair
/// or an out-of-box point is refused (G2005), and `pivot` is a look
/// word, not an anim lane.
#[test]
fn pivot_fences() {
    assert!(fails("View v { Panel p pivot=2x0.5 }"), "out of the box");
    assert!(fails("View v { Panel p pivot=0.5 }"), "a bare number");
    assert!(fails("View v { Panel p pivot=0.5x }"), "half a pair");
    assert!(!fails("View v { Panel p pivot=0x1 }"), "corners are legal");
    assert!(fails("anim a { p.pivot to=1, duration=1 }\nView v { Panel p }"), "not an anim lane");
}
