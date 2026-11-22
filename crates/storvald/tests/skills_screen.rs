//! The skills screen, run with no window: the page, the bundled skills and
//! the screen's code together.

use gogo_ui::runtime::ItemKey;
use gogo_ui::{Event, NodeId, Rect};
use storvald::screen::{self, SkillsScreen};
use storvald::skills::{self, Skill};
use turbodot::{App, Gpu, Picture, Ui};

/// The screen after its first frame, at the size the window opens at.
fn shown() -> (Ui, SkillsScreen) {
    let mut ui: Ui = screen::ui().expect("the page compiles");
    let mut app: SkillsScreen = SkillsScreen::new(skills::bundled(), &ui);
    app.show(&mut ui);
    ui.frame(screen::SIZE.0, screen::SIZE.1, 0.0);
    (ui, app)
}

/// The list row of the skill named `name`.
fn row(ui: &Ui, name: &str) -> NodeId {
    let repeat: NodeId = ui.node("skills");
    let rows: Vec<NodeId> = ui.rows(repeat);
    rows.into_iter().find(|row| ui.row_key(*row) == Some(&ItemKey::Str(name.to_string()))).unwrap_or_else(|| panic!("no row for `{name}`"))
}

#[test]
fn the_screen_opens_on_the_first_skill() {
    let (ui, app) = shown();
    let all: Vec<Skill> = skills::bundled();
    let first: &Skill = &all[0];
    assert_eq!(app.picked(), Some(first));
    assert_eq!(ui.rows(ui.node("skills")).len(), all.len(), "one row per skill");
    assert_eq!(ui.selected_keys(ui.node("skillList")), [ItemKey::Str(first.name.clone())]);
    assert_eq!(ui.text_of(ui.node("skillName")), Some(first.name.as_str()));
    assert_eq!(ui.text_of(ui.node("skillAbout")), Some(first.description.as_str()));
    assert_eq!(ui.text_of(ui.node("skillText")), Some(first.text.as_str()));
    assert_eq!(ui.text_of(ui.node("carried")), Some(format!("storvald {} carries {} skills", env!("CARGO_PKG_VERSION"), all.len()).as_str()));
}

/// A click on another row shows that skill on the right, from its top.
#[test]
fn a_click_on_a_row_shows_that_skill() {
    let (mut ui, mut app) = shown();
    let all: Vec<Skill> = skills::bundled();
    let last: &Skill = all.last().expect("a skill");
    // The text is scrolled down first, to see that a pick starts at the top.
    let scroll: NodeId = ui.node("skillScroll");
    ui.set_scroll(scroll, [0.0, 300.0]);
    ui.frame(screen::SIZE.0, screen::SIZE.1, 0.0);
    assert_eq!(ui.scroll_of(scroll), [0.0, 300.0], "the first skill's text is long enough to scroll");

    let events: Vec<Event> = ui.tap(row(&ui, &last.name), 1.0);
    app.handle(&mut ui, &events);
    ui.frame(screen::SIZE.0, screen::SIZE.1, 0.0);

    assert_eq!(app.picked(), Some(last));
    assert_eq!(ui.text_of(ui.node("skillName")), Some(last.name.as_str()));
    assert_eq!(ui.text_of(ui.node("skillText")), Some(last.text.as_str()));
    assert_eq!(ui.scroll_of(scroll), [0.0, 0.0]);
}

/// Every skill can be picked, one after the other and back to the first:
/// each shows its own text, and the frame after the pick lays it out.
///
/// Needed because a text is measured when the frame after a pick lays it
/// out, and a text can stop the program there: the window closed on a click
/// on `explain-comments`, the second row, while a test that clicked only
/// the last row passed.
#[test]
fn every_skill_can_be_picked() {
    let (mut ui, mut app) = shown();
    let all: Vec<Skill> = skills::bundled();
    let mut t: f64 = 1.0;
    for skill in all.iter().chain(all.first()) {
        let events: Vec<Event> = ui.tap(row(&ui, &skill.name), t);
        app.handle(&mut ui, &events);
        ui.frame(screen::SIZE.0, screen::SIZE.1, 0.016);
        t += 1.0;
        assert_eq!(app.picked(), Some(skill));
        assert_eq!(ui.text_of(ui.node("skillText")), Some(skill.text.as_str()));
        assert!(ui.rect(ui.node("skillText")).h > 100.0, "`{}` has a text of many lines", skill.name);
    }
}

/// The list is the left column and the picked skill fills the rest, by
/// the numbers of the layout.
#[test]
fn the_list_is_on_the_left_and_the_skill_fills_the_rest() {
    let (ui, _) = shown();
    let (side, main): (Rect, Rect) = (ui.rect(ui.node("side")), ui.rect(ui.node("main")));
    assert_eq!((side.x, side.y, side.w, side.h), (0.0, 0.0, 260.0, screen::SIZE.1));
    assert_eq!((main.x, main.y, main.w, main.h), (260.0, 0.0, screen::SIZE.0 - 260.0, screen::SIZE.1));
    let about: Rect = ui.rect(ui.node("skillAbout"));
    assert!(about.x + about.w <= screen::SIZE.0 - 24.0, "the description wraps inside the right side's padding: {about:?}");
    let outline: String = ui.outline();
    assert!(outline.contains("Panel side x=0 y=0 w=260 h=720"), "{outline}");
}

/// The mouse wheel over the skill's text moves the text, and over the
/// list it leaves the text alone.
#[test]
fn the_wheel_scrolls_the_text_under_the_mouse() {
    let (mut ui, _) = shown();
    let scroll: NodeId = ui.node("skillScroll");
    let text: Rect = ui.rect(scroll);
    ui.wheel(text.x + 40.0, text.y + 40.0, -2.0);
    ui.frame(screen::SIZE.0, screen::SIZE.1, 0.0);
    assert_eq!(ui.scroll_of(scroll), [0.0, 96.0], "two notches toward the user move the text up by 96");
    ui.wheel(text.x + 40.0, text.y + 40.0, 5.0);
    ui.frame(screen::SIZE.0, screen::SIZE.1, 0.0);
    assert_eq!(ui.scroll_of(scroll), [0.0, 0.0], "the top holds");
    // Far past the end: the text stops with its last line at the bottom
    // of the box.
    ui.wheel(text.x + 40.0, text.y + 40.0, -1000.0);
    ui.frame(screen::SIZE.0, screen::SIZE.1, 0.0);
    ui.frame(screen::SIZE.0, screen::SIZE.1, 0.0);
    let whole: Rect = ui.rect(ui.node("skillText"));
    assert_eq!(ui.scroll_of(scroll), [0.0, whole.h - text.h], "the end holds: the text is {} tall in a box of {}", whole.h, text.h);
    ui.wheel(text.x + 40.0, text.y + 40.0, 1000.0);
    ui.frame(screen::SIZE.0, screen::SIZE.1, 0.0);
    let side: Rect = ui.rect(ui.node("side"));
    ui.wheel(side.x + 40.0, side.y + 80.0, -2.0);
    ui.frame(screen::SIZE.0, screen::SIZE.1, 0.0);
    assert_eq!(ui.scroll_of(scroll), [0.0, 0.0]);
}

/// The screen drawn on a real GPU: the two sides have their colors, the
/// picked row has its own, and there are letters on both sides. Leaves
/// the picture at `target/pictures/skills_screen.png` to look at.
///
/// Needed because the tests above read the layout and the texts, and a
/// page whose numbers are right can still draw nothing.
///
/// Ignored unless asked for: it needs a GPU adapter, which a hosted test
/// machine may not have, and a test that returned early there would count
/// as passed without having run. On a machine with a GPU:
///
///     cargo test -p storvald -- --include-ignored
#[test]
#[ignore = "needs a GPU adapter: run with --include-ignored on a machine that has one"]
fn the_screen_is_drawn_as_the_page_says() {
    const SIDE: [u8; 4] = [0x1F, 0x21, 0x27, 0xFF];
    const BACK: [u8; 4] = [0x17, 0x18, 0x1C, 0xFF];
    const PICK: [u8; 4] = [0x2F, 0x4C, 0x7A, 0xFF];
    const INK: [u8; 4] = [0xE8, 0xE8, 0xEC, 0xFF];

    let (ui, app) = shown();
    let gpu: Gpu = Gpu::new().expect("this test needs a GPU adapter");
    let picture: Picture = gpu.draw(&ui).expect("the page is drawn");
    picture.save_png(std::path::Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../../target/pictures/skills_screen.png"))).expect("the picture is saved");
    assert_eq!((picture.width, picture.height), (screen::SIZE.0 as u32, screen::SIZE.1 as u32));

    // The colors of the two sides, at their bottom corners, away from text.
    assert_eq!(picture.at(4, picture.height - 4), SIDE, "the left side");
    assert_eq!(picture.at(picture.width - 4, picture.height - 4), BACK, "the right side");
    // The picked row's own color, at its right end, past its text.
    let picked: Rect = ui.rect(row(&ui, &app.picked().expect("a skill").name));
    assert_eq!(picture.at((picked.x + picked.w - 12.0) as u32, (picked.y + picked.h / 2.0) as u32), PICK, "the picked row");
    // Letters: text-colored pixels inside the skill's name and its text.
    for name in ["skillName", "skillText"] {
        let r: Rect = ui.rect(ui.node(name));
        let (x0, y0, x1, y1) = (r.x as u32, r.y as u32, ((r.x + r.w) as u32).min(picture.width), ((r.y + r.h) as u32).min(picture.height));
        let ink: usize = (y0..y1).flat_map(|y| (x0..x1).map(move |x| (x, y))).filter(|&(x, y)| picture.at(x, y) == INK).count();
        assert!(ink > 50, "`{name}` draws its letters: {ink} text-colored pixels in {r:?}");
    }
}
