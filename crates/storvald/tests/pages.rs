//! Storvald's pages, run with no window: the pages, the bundled skills and
//! the pages' code together.

use storvald::about_page::AboutPage;
use storvald::skills::{self, Skill};
use storvald::skills_page::SkillsPage;
use storvald::ui::{about_page, skills_page};
use storvald::{AppState, WINDOW_SIZE_EPX};
use turbodot::gogo::runtime::ItemKey;
use turbodot::gogo::{NodeId, Rect};
use turbodot::{Gpu, Picture, Program};

/// Nodes of skills page.
const SKILLS: skills_page::Names = skills_page::NAMES;
/// Nodes of about page.
const ABOUT: about_page::Names = about_page::NAMES;

/// One frame at size window opens at, `dt` seconds after last.
fn frame(program: &mut Program<AppState>, dt: f32) -> bool {
    program.frame(WINDOW_SIZE_EPX.0, WINDOW_SIZE_EPX.1, dt)
}

/// Frames until no page slides any more.
fn let_the_slides_end(program: &mut Program<AppState>) {
    for _ in 0..60 {
        frame(program, 0.016);
        if !program.is_sliding() {
            return;
        }
    }
    panic!("a slide is still running after a second");
}

/// Program after its first frame.
fn shown() -> Program<AppState> {
    let mut program: Program<AppState> = storvald::program(false).expect("the pages can be shown");
    frame(&mut program, 0.0);
    program
}

/// The list row of the skill named `name`.
fn row(program: &Program<AppState>, name: &str) -> NodeId {
    let rows: Vec<NodeId> = program.ui().rows(SKILLS.skills);
    rows.into_iter().find(|row| program.ui().row_key(*row) == Some(&ItemKey::Str(name.to_string()))).unwrap_or_else(|| panic!("no row for `{name}`"))
}

/// Skill the skills page has picked.
fn picked(program: &Program<AppState>) -> Option<&Skill> {
    program.page::<SkillsPage>().expect("the skills page is on screen").picked(program.app())
}

#[test]
fn the_program_opens_on_the_first_skill() {
    let program: Program<AppState> = shown();
    let all: Vec<Skill> = skills::bundled();
    let first: &Skill = &all[0];
    assert_eq!(program.depth(), 1);
    assert_eq!(picked(&program), Some(first));
    assert_eq!(program.ui().rows(SKILLS.skills).len(), all.len(), "one row per skill");
    assert_eq!(program.ui().selected_keys(SKILLS.skill_list), [ItemKey::Str(first.name.clone())]);
    assert_eq!(program.ui().text_of(SKILLS.skill_name), Some(first.name.as_str()));
    assert_eq!(program.ui().text_of(SKILLS.skill_about), Some(first.description.as_str()));
    assert_eq!(program.ui().text_of(SKILLS.skill_text), Some(first.text.as_str()));
    assert_eq!(program.ui().text_of(SKILLS.carried), Some(format!("storvald {} carries {} skills", env!("CARGO_PKG_VERSION"), all.len()).as_str()));
}

/// A click on another row shows that skill on the right, from its top.
#[test]
fn a_click_on_a_row_shows_that_skill() {
    let mut program: Program<AppState> = shown();
    let all: Vec<Skill> = skills::bundled();
    let last: &Skill = all.last().expect("a skill");
    // The text is scrolled down first, to see that a pick starts at the top.
    program.ui_mut().set_scroll(SKILLS.skill_scroll, [0.0, 300.0]);
    frame(&mut program, 0.0);
    assert_eq!(program.ui().scroll_of(SKILLS.skill_scroll), [0.0, 300.0], "the first skill's text is long enough to scroll");

    program.tap(row(&program, &last.name), 1.0);
    frame(&mut program, 0.0);

    assert_eq!(picked(&program), Some(last));
    assert_eq!(program.ui().text_of(SKILLS.skill_name), Some(last.name.as_str()));
    assert_eq!(program.ui().text_of(SKILLS.skill_text), Some(last.text.as_str()));
    assert_eq!(program.ui().scroll_of(SKILLS.skill_scroll), [0.0, 0.0]);
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
    let mut program: Program<AppState> = shown();
    let all: Vec<Skill> = skills::bundled();
    let mut t: f64 = 1.0;
    for skill in all.iter().chain(all.first()) {
        program.tap(row(&program, &skill.name), t);
        frame(&mut program, 0.016);
        t += 1.0;
        assert_eq!(picked(&program), Some(skill));
        assert_eq!(program.ui().text_of(SKILLS.skill_text), Some(skill.text.as_str()));
        assert!(program.ui().rect(SKILLS.skill_text).h > 100.0, "`{}` has a text of many lines", skill.name);
    }
}

/// The list is the left column and the picked skill fills the rest, by
/// the numbers of the layout.
#[test]
fn the_list_is_on_the_left_and_the_skill_fills_the_rest() {
    let program: Program<AppState> = shown();
    let (side, main): (Rect, Rect) = (program.ui().rect(SKILLS.side), program.ui().rect(SKILLS.main));
    assert_eq!((side.x, side.y, side.w, side.h), (0.0, 0.0, 260.0, WINDOW_SIZE_EPX.1));
    assert_eq!((main.x, main.y, main.w, main.h), (260.0, 0.0, WINDOW_SIZE_EPX.0 - 260.0, WINDOW_SIZE_EPX.1));
    let about: Rect = program.ui().rect(SKILLS.skill_about);
    assert!(about.x + about.w <= WINDOW_SIZE_EPX.0 - 24.0, "the description wraps inside the right side's padding: {about:?}");
    let outline: String = program.ui().outline();
    assert!(outline.contains("Panel side x=0 y=0 w=260 h=720"), "{outline}");
}

/// The mouse wheel over the skill's text moves the text, and over the
/// list it leaves the text alone.
#[test]
fn the_wheel_scrolls_the_text_under_the_mouse() {
    let mut program: Program<AppState> = shown();
    let scroll: NodeId = SKILLS.skill_scroll;
    let text: Rect = program.ui().rect(scroll);
    program.wheel(text.x + 40.0, text.y + 40.0, -2.0);
    frame(&mut program, 0.0);
    assert_eq!(program.ui().scroll_of(scroll), [0.0, 96.0], "two notches toward the user move the text up by 96");
    program.wheel(text.x + 40.0, text.y + 40.0, 5.0);
    frame(&mut program, 0.0);
    assert_eq!(program.ui().scroll_of(scroll), [0.0, 0.0], "the top holds");
    // Far past the end: the text stops with its last line at the bottom
    // of the box.
    program.wheel(text.x + 40.0, text.y + 40.0, -1000.0);
    frame(&mut program, 0.0);
    frame(&mut program, 0.0);
    let whole: Rect = program.ui().rect(SKILLS.skill_text);
    assert_eq!(program.ui().scroll_of(scroll), [0.0, whole.h - text.h], "the end holds: the text is {} tall in a box of {}", whole.h, text.h);
    program.wheel(text.x + 40.0, text.y + 40.0, 1000.0);
    frame(&mut program, 0.0);
    let side: Rect = program.ui().rect(SKILLS.side);
    program.wheel(side.x + 40.0, side.y + 80.0, -2.0);
    frame(&mut program, 0.0);
    assert_eq!(program.ui().scroll_of(scroll), [0.0, 0.0]);
}

/// About button opens about page over skills page. Back request returns
/// to skills page as left: same pick, text scrolled to same place.
#[test]
fn about_opens_over_the_skills_and_back_returns_to_them_as_they_were_left() {
    let mut program: Program<AppState> = shown();
    let all: Vec<Skill> = skills::bundled();
    let last: &Skill = all.last().expect("a skill");
    program.tap(row(&program, &last.name), 1.0);
    frame(&mut program, 0.016);
    program.ui_mut().set_scroll(SKILLS.skill_scroll, [0.0, 120.0]);
    frame(&mut program, 0.016);

    program.tap(SKILLS.about_button, 2.0);
    assert_eq!(program.depth(), 2);
    assert!(program.page::<AboutPage>().is_some(), "the about page is on screen");
    assert_eq!(program.ui().text_of(ABOUT.about_version), Some(program.app().carried_line().as_str()));
    assert!(program.is_sliding(), "the about page slides in");
    assert_eq!(program.drawn_page_count(), 2, "the skills page shows under the page that slides in");
    let_the_slides_end(&mut program);
    assert_eq!(program.drawn_page_count(), 1, "the about page covers the screen: the skills page is drawn no more");

    assert!(program.back(), "the about page leaves");
    assert_eq!(program.depth(), 1);
    assert_eq!(program.drawn_page_count(), 2, "the about page shows over the skills while it slides out");
    let_the_slides_end(&mut program);
    assert_eq!(program.drawn_page_count(), 1, "the about page is gone");
    assert_eq!(picked(&program), Some(last));
    assert_eq!(program.ui().scroll_of(SKILLS.skill_scroll), [0.0, 120.0]);
    assert_eq!(program.ui().text_of(SKILLS.skill_text), Some(last.text.as_str()));
}

/// Back request on first page is not used: nothing is before it.
#[test]
fn back_on_the_first_page_is_not_used() {
    let mut program: Program<AppState> = shown();
    assert!(!program.back());
    assert_eq!(program.depth(), 1);
    assert!(program.page::<SkillsPage>().is_some());
}

/// About page's own Back button goes the road of the back key.
#[test]
fn the_back_button_of_the_about_page_returns_to_the_skills() {
    let mut program: Program<AppState> = shown();
    program.tap(SKILLS.about_button, 1.0);
    let_the_slides_end(&mut program);
    program.tap(ABOUT.back_button, 2.0);
    assert_eq!(program.depth(), 1);
    assert!(program.page::<SkillsPage>().is_some());
    let_the_slides_end(&mut program);
    assert_eq!(program.drawn_page_count(), 1);
}

/// Pages drawn on a real GPU: on skills page the two sides have their
/// colors, picked row has its own, letters on both sides; about page has
/// its letters. Leaves pictures at `target/pictures/skills_page.png` and
/// `about_page.png` to look at.
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
fn the_pages_are_drawn_as_their_files_say() {
    const SIDE: [u8; 4] = [0x1F, 0x21, 0x27, 0xFF];
    const BACK: [u8; 4] = [0x17, 0x18, 0x1C, 0xFF];
    const PICK: [u8; 4] = [0x2F, 0x4C, 0x7A, 0xFF];
    const INK: [u8; 4] = [0xE8, 0xE8, 0xEC, 0xFF];

    /// How many text-colored pixels `picture` has inside `r`.
    fn ink_in(picture: &Picture, r: Rect) -> usize {
        let (x0, y0, x1, y1) = (r.x as u32, r.y as u32, ((r.x + r.w) as u32).min(picture.width), ((r.y + r.h) as u32).min(picture.height));
        (y0..y1).flat_map(|y| (x0..x1).map(move |x| (x, y))).filter(|&(x, y)| picture.at(x, y) == INK).count()
    }

    let mut program: Program<AppState> = shown();
    let gpu: Gpu = Gpu::new().expect("this test needs a GPU adapter");
    let pictures: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../target/pictures");
    let picture: Picture = gpu.draw_program(&mut program).expect("the skills page is drawn");
    picture.save_png(&std::path::Path::new(pictures).join("skills_page.png")).expect("the picture is saved");
    assert_eq!((picture.width, picture.height), (WINDOW_SIZE_EPX.0 as u32, WINDOW_SIZE_EPX.1 as u32));

    // The colors of the two sides, at their bottom corners, away from text.
    assert_eq!(picture.at(4, picture.height - 4), SIDE, "the left side");
    assert_eq!(picture.at(picture.width - 4, picture.height - 4), BACK, "the right side");
    // The picked row's own color, at its right end, past its text.
    let picked_row: Rect = program.ui().rect(row(&program, &picked(&program).expect("a skill").name));
    assert_eq!(picture.at((picked_row.x + picked_row.w - 12.0) as u32, (picked_row.y + picked_row.h / 2.0) as u32), PICK, "the picked row");
    // Letters: text-colored pixels inside the skill's name, its text, and
    // the button's one small word.
    for (name, node, least) in [("skillName", SKILLS.skill_name, 50), ("skillText", SKILLS.skill_text, 50), ("aboutButton", SKILLS.about_button, 5)] {
        let r: Rect = program.ui().rect(node);
        let ink: usize = ink_in(&picture, r);
        assert!(ink > least, "`{name}` draws its letters: {ink} text-colored pixels in {r:?}");
    }

    program.tap(SKILLS.about_button, 1.0);
    let_the_slides_end(&mut program);
    let picture: Picture = gpu.draw_program(&mut program).expect("the about page is drawn");
    picture.save_png(&std::path::Path::new(pictures).join("about_page.png")).expect("the picture is saved");
    assert_eq!(picture.at(picture.width - 4, picture.height - 4), BACK, "the about page covers the screen");
    assert_eq!(picture.at(4, picture.height - 4), BACK, "nothing of the skills page's left side shows through");
    for (name, node, least) in [("aboutTitle", ABOUT.about_title, 50), ("aboutText", ABOUT.about_text, 50), ("backButton", ABOUT.back_button, 5)] {
        let r: Rect = program.ui().rect(node);
        let ink: usize = ink_in(&picture, r);
        assert!(ink > least, "`{name}` draws its letters: {ink} text-colored pixels in {r:?}");
    }
}
