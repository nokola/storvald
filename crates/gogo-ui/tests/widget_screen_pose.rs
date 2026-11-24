//! D14: a hostless widget poses its OWN body on the screen class —
//! `when screen.landscapeSmall { card.width = 420 }` beside the body — and
//! the pose compiles once per instance as a DEFAULT: a page's pose on the
//! same instance wins, and the two never clash as two writers.
use gogo_ui::layout::{FixedAdvance, Rect, layout};
use gogo_ui::runtime::{Runtime, resolve};
use gogo_ui::{Orientation, Page, ScreenClass, compile};

fn cards() -> Page {
    let src = "widget Card {
    Panel card width=fill, height=40 { Label l text=\"c\" }
    when screen.landscapeSmall {
        card.width = 420
        card.halign = left
    }
}
View v { Column { Card a; Card b { when screen.landscapeSmall width=300 } } }";
    let c = compile(src, &|_| None);
    assert!(!gogo_ui::has_errors(&c.diags), "{:#?}", c.diags);
    c.page
}

fn widths(page: &Page, rt: &Runtime, view: Rect) -> (f32, f32) {
    let res = resolve(page, rt);
    let rects = layout(page, &res, view, &FixedAdvance);
    // The body root IS the instance (`a/card` aliases `a`).
    (rects[page.find("a").unwrap()].w, rects[page.find("b").unwrap()].w)
}

#[test]
fn a_widgets_screen_pose_lands_per_instance_and_the_pages_pose_wins() {
    let page = cards();
    let mut rt = Runtime::new(&page);
    rt.set_screen(&page, Orientation::Portrait, ScreenClass::PortraitSmall);
    assert_eq!(widths(&page, &rt, Rect::new(0.0, 0.0, 360.0, 800.0)), (360.0, 360.0), "portrait: the body's own width=fill");
    rt.set_screen(&page, Orientation::Landscape, ScreenClass::LandscapeSmall);
    let (a, b) = widths(&page, &rt, Rect::new(0.0, 0.0, 800.0, 360.0));
    assert_eq!(a, 420.0, "the widget's pose, per instance");
    assert_eq!(b, 300.0, "the page's instance-line pose wins over the widget's default");
}

/// Any other state word at widget scope still names nothing to answer.
#[test]
fn a_hostless_widgets_state_when_is_still_refused() {
    let c = compile("widget Card { Panel card width=fill { Label l text=\"c\" }
 when held { card.width = 1 } }
View v { Card a }", &|_| None);
    assert!(c.diags.iter().any(|d| d.code == "G5001"), "{:#?}", c.diags);
}

/// A widget's pose may move its items into its own seats: the seat's
/// name is the body's, one per instance, like the node's.
#[test]
fn a_widgets_screen_pose_moves_an_item_into_the_widgets_own_seat() {
    let src = "widget Strip {
    Panel plate width=hug, height=hug {
        Column { Rectangle first 10x10; Rectangle second 10x10 }
        Row { seat firstSeat; seat secondSeat }
    }
    when screen.landscapeSmall {
        first.seat = firstSeat
        second.seat = secondSeat
    }
}
View v { Column { Strip a; Strip b } }";
    let c = compile(src, &|_| None);
    assert!(!gogo_ui::has_errors(&c.diags), "{:#?}", c.diags);
    let page: Page = c.page;
    let mut rt = Runtime::new(&page);
    let places = |rt: &Runtime, view: Rect, instance: &str| -> (Rect, Rect) {
        let rects = layout(&page, &resolve(&page, rt), view, &FixedAdvance);
        (rects[page.find(&format!("{instance}/first")).unwrap()], rects[page.find(&format!("{instance}/second")).unwrap()])
    };
    rt.set_screen(&page, Orientation::Portrait, ScreenClass::PortraitSmall);
    for instance in ["a", "b"] {
        let (first, second) = places(&rt, Rect::new(0.0, 0.0, 360.0, 800.0), instance);
        assert_eq!((second.x, second.y), (first.x, first.y + 10.0), "{instance}, portrait: one under the other, at home");
    }
    rt.set_screen(&page, Orientation::Landscape, ScreenClass::LandscapeSmall);
    let (a_first, a_second) = places(&rt, Rect::new(0.0, 0.0, 800.0, 360.0), "a");
    let (b_first, _) = places(&rt, Rect::new(0.0, 0.0, 800.0, 360.0), "b");
    assert_eq!((a_second.x, a_second.y), (a_first.x + 10.0, a_first.y), "landscape: side by side, in the seats");
    assert!(b_first.y > a_first.y, "each instance in its own seats: {a_first:?} {b_first:?}");
}

/// A seat of the page is no seat of a widget: its body cannot know the
/// page it is placed in.
#[test]
fn a_widgets_screen_pose_cannot_name_a_seat_of_the_page() {
    let c = compile("widget Strip {
    Panel plate { Rectangle first 10x10 }
    when screen.landscapeSmall { first.seat = pageSeat }
}
View v { Row { seat pageSeat }
 Strip a }", &|_| None);
    assert!(c.diags.iter().any(|d| d.code == "G1016"), "{:#?}", c.diags);
}
