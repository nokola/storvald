//! Small notes program on TurboDot, in the shape every client has: one
//! line builds pages (`build.rs`), one declares them, `main` starts the
//! program, plus a struct and a `.gogo` file per page.
//!
//! - `home_page`: the notes. Opens the other two pages.
//! - `edit_page`: a new note. Asks once before back drops it.
//! - `settings_page`: cached page. Later open shows same view.
//! - `clear_page`: overlay page. Home page stays in view under it.
//! - `paper_page`: page with a GPU picture of its own under its nodes.
//! - `footer_page`: page placed inside the home page and the edit page.
//! - `toast_page`: layer above every page, shows a line for a moment.
//!
//! Run it with `cargo run -p turbodot-example-notes`. Esc is back.

mod clear_page;
mod edit_page;
mod footer_page;
mod home_page;
mod paper_page;
mod settings_page;
mod toast_page;

turbodot::built_pages!();

use home_page::HomePage;
use turbodot::{Program, WindowOptions};

/// Program-wide state. Every page reaches it as `host.app`.
#[derive(Default)]
pub struct AppState {
    pub notes: Vec<String>,
    /// True when device has too little memory to cache pages.
    pub is_memory_short: bool,
    /// Count of frames paper page did its GPU drawing for. A test reads
    /// it: engine skips that drawing while another page covers the page.
    pub paper_frames_drawn: u32,
}

/// Opens window on home page, stays until window is closed. Debug build:
/// pages follow their `.gogo` files.
///
/// Returns why pages or window could not show.
fn main() -> Result<(), String> {
    let program: Program<AppState> = Program::following_page_files(&ui::BUILT, AppState::default(), HomePage)?;
    turbodot::start(WindowOptions { title: "Notes", width: 960.0, height: 640.0 }, program)
}

#[cfg(test)]
mod tests {
    use turbodot::Program;
    use turbodot::prelude::*;
    use turbodot::gogo::{Phase, Rect};

    use super::*;
    use crate::clear_page::ClearPage;
    use crate::edit_page::EditPage;
    use crate::footer_page::FooterPage;
    use crate::toast_page::{TOAST_SEC, ToastPage};
    use crate::paper_page::{LINE, LINE_EVERY_TEXELS, PAPER, PAPER_SIDE_TEXELS, PaperPage};
    use crate::settings_page::{MADE_PICTURE_COLOR, MADE_PICTURE_SIDE_PX, SettingsPage};
    use crate::ui::{clear_page, edit_page, footer_page, home_page, settings_page, toast_page};

    const HOME: home_page::Names = home_page::NAMES;
    const CLEAR: clear_page::Names = clear_page::NAMES;
    const EDIT: edit_page::Names = edit_page::NAMES;
    const SETTINGS: settings_page::Names = settings_page::NAMES;
    const FOOTER: footer_page::Names = footer_page::NAMES;
    const TOAST: toast_page::Names = toast_page::NAMES;

    /// Size tests lay pages out for, in epx.
    const SIZE_EPX: (f32, f32) = (480.0, 640.0);

    /// One frame, `dt` seconds after last.
    fn frame(program: &mut Program<AppState>, dt: f32) {
        program.frame(SIZE_EPX.0, SIZE_EPX.1, dt);
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
    fn started(app: AppState) -> Program<AppState> {
        let mut program: Program<AppState> = Program::new(&crate::ui::BUILT, app, HomePage).expect("the pages can be shown");
        frame(&mut program, 0.0);
        program
    }

    /// Opens settings page from home page, waits for its slide.
    fn open_settings(program: &mut Program<AppState>, t: f64) {
        program.tap(HOME.open_settings, t);
        assert!(program.page::<SettingsPage>().is_some(), "the settings page is on screen");
        let_the_slides_end(program);
    }

    #[test]
    fn the_program_opens_on_the_notes_with_the_settings_page_made_ahead() {
        let program: Program<AppState> = started(AppState::default());
        assert_eq!(program.depth(), 1);
        assert!(program.page::<HomePage>().is_some());
        assert_eq!(program.ui().rows(HOME.notes).len(), 0);
        assert_eq!(program.cached_page_count(), 1, "the settings page is made ahead of its first open");
        assert_eq!(program.drawn_page_count(), 1, "a cached page is not drawn");
    }

    /// Save keeps note and goes back. Home page writes its list again
    /// when it comes back into view.
    #[test]
    fn a_saved_note_is_in_the_list_on_the_way_back() {
        let mut program: Program<AppState> = started(AppState::default());
        program.tap(HOME.add_note, 1.0);
        assert_eq!(program.depth(), 2);
        assert!(program.page::<EditPage>().is_some());
        assert_eq!(program.ui().text_of(EDIT.title), Some("Note 1"));
        let_the_slides_end(&mut program);

        program.tap(EDIT.save_note, 2.0);
        assert_eq!(program.depth(), 1);
        assert_eq!(program.app().notes, ["Note 1"]);
        assert_eq!(program.ui().rows(HOME.notes).len(), 1);
        let_the_slides_end(&mut program);
        assert_eq!(program.drawn_page_count(), 1, "the edit page is freed");
    }

    /// First back request on unsaved note stays and shows warning. Second
    /// one drops the note.
    #[test]
    fn back_on_a_note_that_is_not_saved_stays_once() {
        let mut program: Program<AppState> = started(AppState::default());
        program.tap(HOME.add_note, 1.0);
        let_the_slides_end(&mut program);
        let hidden: Rect = program.ui().rect(EDIT.unsaved);
        assert_eq!(hidden.h, 0.0, "the warning takes no space before it is shown");

        assert!(program.back(), "the page used the request");
        assert_eq!(program.depth(), 2, "and stayed");
        frame(&mut program, 0.016);
        assert!(program.ui().rect(EDIT.unsaved).h > 0.0, "the warning shows");

        assert!(program.back());
        assert_eq!(program.depth(), 1);
        assert!(program.app().notes.is_empty());
    }

    /// Cached page opened again shows same view: text scrolled to same
    /// place. Takes over opener's value.
    #[test]
    fn the_settings_page_is_shown_again_as_it_was_left() {
        let mut program: Program<AppState> = started(AppState::default());
        open_settings(&mut program, 1.0);
        assert_eq!((program.depth(), program.cached_page_count()), (2, 0));
        assert_eq!(program.ui().text_of(SETTINGS.note_count), Some("0 notes when this page was opened"));
        program.ui_mut().set_scroll(SETTINGS.body, [0.0, 200.0]);
        frame(&mut program, 0.016);
        assert_eq!(program.ui().scroll_of(SETTINGS.body), [0.0, 200.0], "the text is long enough to scroll");

        program.tap(SETTINGS.go_back, 2.0);
        let_the_slides_end(&mut program);
        assert_eq!((program.depth(), program.cached_page_count()), (1, 1), "the page is cached, not freed");

        program.tap(HOME.add_note, 3.0);
        let_the_slides_end(&mut program);
        program.tap(EDIT.save_note, 4.0);
        let_the_slides_end(&mut program);

        open_settings(&mut program, 5.0);
        assert_eq!((program.depth(), program.cached_page_count()), (2, 0));
        assert_eq!(program.ui().scroll_of(SETTINGS.body), [0.0, 200.0], "the same page: its text is where it was left");
        assert_eq!(program.page::<SettingsPage>().map(SettingsPage::note_count), Some(1), "`reopened` took the new count over");
        assert_eq!(program.ui().text_of(SETTINGS.note_count), Some("1 notes when this page was opened"));
    }

    /// Page cache off: nothing made ahead, cached page freed on back as
    /// any page.
    #[test]
    fn a_device_short_of_memory_caches_no_page() {
        let mut program: Program<AppState> = started(AppState { is_memory_short: true, ..AppState::default() });
        assert_eq!(program.cached_page_count(), 0, "nothing is made ahead");
        open_settings(&mut program, 1.0);
        program.ui_mut().set_scroll(SETTINGS.body, [0.0, 200.0]);
        frame(&mut program, 0.016);
        assert!(program.back());
        let_the_slides_end(&mut program);
        assert_eq!(program.cached_page_count(), 0, "the page is freed");

        open_settings(&mut program, 2.0);
        assert_eq!(program.ui().scroll_of(SETTINGS.body), [0.0, 0.0], "a new page");
    }

    /// `free_cached_pages` frees cached page. Next open makes it again.
    #[test]
    fn free_cached_pages_frees_the_cached_page() {
        let mut program: Program<AppState> = started(AppState::default());
        assert_eq!(program.cached_page_count(), 1);
        program.free_cached_pages();
        assert_eq!(program.cached_page_count(), 0);
        open_settings(&mut program, 1.0);
        assert_eq!(program.depth(), 2);
        assert!(program.ui().text_of(SETTINGS.body_text).is_some_and(|text| text.starts_with("Line 1 ")), "`loaded` ran for the new page");
    }

    /// Cached page opened again while still sliding out is the same page:
    /// slide stops, page is neither cached nor made a second time.
    #[test]
    fn a_cached_page_opened_again_while_it_slides_out_is_the_same_page() {
        let mut program: Program<AppState> = started(AppState::default());
        open_settings(&mut program, 1.0);
        program.ui_mut().set_scroll(SETTINGS.body, [0.0, 150.0]);
        frame(&mut program, 0.016);

        assert!(program.back());
        frame(&mut program, 0.016);
        assert!(program.is_sliding(), "the settings page is on its way out");
        program.tap(HOME.open_settings, 2.0);
        assert_eq!((program.depth(), program.cached_page_count()), (2, 0));
        let_the_slides_end(&mut program);
        assert_eq!(program.ui().scroll_of(SETTINGS.body), [0.0, 150.0], "the same page");
        assert_eq!((program.cached_page_count(), program.drawn_page_count()), (0, 1));
    }

    /// Home page stays drawn under an overlay page, also after overlay
    /// slid in. Under a fullscreen page it does not (tests above).
    #[test]
    fn the_page_under_an_overlay_stays_drawn() {
        let mut program: Program<AppState> = started(AppState { notes: vec!["Note 1".to_string()], ..AppState::default() });
        program.tap(HOME.clear_notes, 1.0);
        assert!(program.page::<ClearPage>().is_some());
        let_the_slides_end(&mut program);
        assert_eq!((program.depth(), program.drawn_page_count()), (2, 2), "home page shows under the overlay");

        program.tap(CLEAR.keep_notes, 2.0);
        let_the_slides_end(&mut program);
        assert_eq!((program.depth(), program.drawn_page_count()), (1, 1));
        assert_eq!(program.app().notes.len(), 1, "Keep drops nothing");

        program.tap(HOME.clear_notes, 3.0);
        let_the_slides_end(&mut program);
        program.tap(CLEAR.clear_all, 4.0);
        assert!(program.app().notes.is_empty());
        assert_eq!(program.ui().rows(HOME.notes).len(), 0, "home page wrote its list again");
    }

    /// A fullscreen page over an overlay covers both pages under it,
    /// and back shows the overlay with home page under it again.
    #[test]
    fn a_fullscreen_page_over_an_overlay_covers_what_is_under_it() {
        let mut program: Program<AppState> = started(AppState::default());
        program.tap(HOME.clear_notes, 1.0);
        let_the_slides_end(&mut program);
        program.go(EditPage::new(0));
        assert_eq!(program.drawn_page_count(), 3, "all three show while the edit page slides in");
        let_the_slides_end(&mut program);
        assert_eq!((program.depth(), program.drawn_page_count()), (3, 1));

        assert!(program.back(), "the unsaved note asks first");
        assert!(program.back());
        let_the_slides_end(&mut program);
        assert_eq!((program.depth(), program.drawn_page_count()), (2, 2));
    }

    /// Page with a see-through root that says nothing of its kind (so:
    /// fullscreen) stops a debug build with the reason.
    #[test]
    #[should_panic(expected = "page `clearPage` is `PageKind::Fullscreen`, but its root does not cover the window")]
    fn a_see_through_page_that_says_fullscreen_stops_a_debug_build() {
        /// `clear_page.gogo` with no `KIND` line.
        struct NoKindPage;
        impl Page<AppState> for NoKindPage {
            const PAGE: &'static BuiltPage = &clear_page::PAGE;
            type Handler = clear_page::Handler;
            fn handle(&mut self, _host: &mut Host<AppState>, _event: PageEvent<clear_page::Handler>) {}
        }
        let mut program: Program<AppState> = started(AppState::default());
        program.go(NoKindPage);
        let_the_slides_end(&mut program);
        frame(&mut program, 0.016);
    }

    /// Fullscreen page with a see-through root passes the debug check of
    /// its kind when it says it draws its own GPU picture, and covers the
    /// page under it.
    #[test]
    fn a_page_with_its_own_gpu_picture_covers_the_page_under_it() {
        let mut program: Program<AppState> = started(AppState::default());
        program.tap(HOME.open_paper, 1.0);
        assert!(program.page::<PaperPage>().is_some());
        let_the_slides_end(&mut program);
        frame(&mut program, 0.016);
        assert_eq!((program.depth(), program.drawn_page_count()), (2, 1));
        assert_eq!(program.app().paper_frames_drawn, 0, "no GPU, no GPU drawing");
    }

    /// Page's own picture shows under its nodes, and engine calls page's
    /// GPU drawing only while page is drawn: each frame while it is on
    /// screen or under an overlay, never under a fullscreen page.
    ///
    /// Ignored unless asked for: needs a GPU adapter (see the test above
    /// this file's last).
    #[test]
    #[ignore = "needs a GPU adapter: run with --include-ignored on a machine that has one"]
    fn a_page_draws_its_own_gpu_picture_only_while_it_is_drawn() {
        let mut program: Program<AppState> = started(AppState::default());
        let gpu: turbodot::Gpu = turbodot::Gpu::new().expect("this test needs a GPU adapter");
        program.go(PaperPage::default());
        let_the_slides_end(&mut program);

        // Paper texture is stretched over the window: each texel row is
        // this many pixels tall, read at its middle.
        let picture: turbodot::Picture = gpu.draw_program(&mut program).expect("the pages are drawn");
        let row_px: u32 = picture.height / PAPER_SIDE_TEXELS;
        let far_right: u32 = picture.width - 10;
        // Smooth sampling blends a line's row a little with the paper row
        // beside it: the line's color, within 8 of 255 per channel.
        let line: [u8; 4] = picture.at(far_right, (LINE_EVERY_TEXELS - 1) * row_px + row_px / 2);
        assert!((0..4).all(|c| line[c].abs_diff(LINE[c]) <= 8), "a ruled line: {line:?}");
        assert_eq!(picture.at(far_right, (LINE_EVERY_TEXELS - 6) * row_px + row_px / 2), PAPER, "paper between two lines");
        let title: Rect = program.ui().rect(crate::ui::paper_page::NAMES.title);
        let ink: usize = (title.y as u32..(title.y + title.h) as u32).flat_map(|y| (title.x as u32..(title.x + title.w) as u32).map(move |x| (x, y))).filter(|&(x, y)| picture.at(x, y) == [0x20, 0x20, 0x20, 0xFF]).count();
        assert!(ink > 20, "page's title is drawn over the paper: {ink} text-colored pixels");
        picture.save_png(&std::path::PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/../../target/pictures/notes_paper_page.png"))).expect("the picture is saved");

        let on_screen: u32 = program.app().paper_frames_drawn;
        assert!(on_screen >= 1);
        gpu.draw_program(&mut program).expect("the pages are drawn");
        assert_eq!(program.app().paper_frames_drawn, on_screen + 1, "each frame while page is on screen");

        program.go(SettingsPage::new(0));
        let_the_slides_end(&mut program);
        let covered: u32 = program.app().paper_frames_drawn;
        gpu.draw_program(&mut program).expect("the pages are drawn");
        gpu.draw_program(&mut program).expect("the pages are drawn");
        assert_eq!(program.app().paper_frames_drawn, covered, "not while a fullscreen page covers it");

        assert!(program.back());
        let_the_slides_end(&mut program);
        program.go(ClearPage);
        let_the_slides_end(&mut program);
        let under_overlay: u32 = program.app().paper_frames_drawn;
        gpu.draw_program(&mut program).expect("the pages are drawn");
        assert_eq!(program.app().paper_frames_drawn, under_overlay + 1, "each frame while page shows under an overlay");
    }

    /// Picture has its size in the first frame, read from its file's
    /// header, with no GPU and before a pixel is decoded.
    ///
    /// Needed because a page lays out around its pictures: without the
    /// size an Image is 0 wide until its pixels come, and user sees the
    /// page jump.
    #[test]
    fn a_picture_has_its_size_in_the_first_frame_with_no_gpu() {
        let mut program: Program<AppState> = started(AppState::default());
        let icon: Rect = program.ui().rect(HOME.note_icon);
        assert_eq!((icon.w, icon.h), (24.0, 24.0), "the png's size");
        let title: Rect = program.ui().rect(HOME.title);
        assert!(title.x >= icon.x + icon.w, "the title sits right of the picture: {title:?} {icon:?}");
        open_settings(&mut program, 1.0);
        let banner: Rect = program.ui().rect(SETTINGS.banner);
        assert_eq!((banner.w, banner.h), (64.0, 16.0), "the jpeg's size");
    }

    /// Layer shows above every page when a page tells it to, takes no
    /// touch beside its nodes, counts its own time by its frame call,
    /// and asks for frames only while it shows.
    ///
    /// Needed for what any page may show over itself and what goes away
    /// with time: a toast.
    #[test]
    fn a_layer_shows_above_the_pages_for_a_while_and_takes_no_touch() {
        let mut program: Program<AppState> = started(AppState::default());
        frame(&mut program, 0.016);
        assert!(program.layer::<ToastPage>().is_some(), "the home page added the layer");
        let shows = |program: &Program<AppState>| -> bool { let plate: Rect = program.layer_ui::<ToastPage>().expect("the toast").rect(TOAST.plate); plate.w > 0.0 && plate.h > 0.0 };
        assert!(!shows(&program), "nothing to say yet");
        assert!(!program.frame(SIZE_EPX.0, SIZE_EPX.1, 0.016), "a still program asks for no frame");

        program.tap(HOME.add_note, 1.0);
        let_the_slides_end(&mut program);
        program.tap(EDIT.save_note, 2.0);
        frame(&mut program, 0.016);
        assert!(shows(&program), "the edit page told the toast");
        assert_eq!(program.layer_ui::<ToastPage>().expect("the toast").text_of(TOAST.message), Some("Saved"));
        let_the_slides_end(&mut program);
        assert!(program.frame(SIZE_EPX.0, SIZE_EPX.1, 0.016), "the toast asks for frames while it shows");

        // A tap on a home page's button goes through the layer.
        program.tap(HOME.open_settings, 3.0);
        assert!(program.page::<SettingsPage>().is_some(), "the layer took no touch");
        let_the_slides_end(&mut program);
        assert!(shows(&program), "the toast stays above the page opened under it");

        for _ in 0..((TOAST_SEC / 0.1) as usize + 2) {
            frame(&mut program, 0.1);
        }
        assert!(!shows(&program), "gone after its time");
        frame(&mut program, 0.016);
        assert!(!program.frame(SIZE_EPX.0, SIZE_EPX.1, 0.016), "and asks for no more frames");
    }

    /// Layer is drawn above the page on screen: its plate shows where the
    /// page under it has its background.
    ///
    /// Ignored unless asked for: needs a GPU adapter.
    #[test]
    #[ignore = "needs a GPU adapter: run with --include-ignored on a machine that has one"]
    fn a_layer_is_drawn_above_the_pages() {
        // Toast's own color: no node of the home page has it.
        const PLATE: [u8; 4] = [0x2E, 0x5C, 0x3E, 0xFF];
        let mut program: Program<AppState> = started(AppState::default());
        program.tap(HOME.add_note, 1.0);
        let_the_slides_end(&mut program);
        program.tap(EDIT.save_note, 2.0);
        let_the_slides_end(&mut program);
        let plate: Rect = program.layer_ui::<ToastPage>().expect("the toast").rect(TOAST.plate);
        let gpu: turbodot::Gpu = turbodot::Gpu::new().expect("this test needs a GPU adapter");
        let picture: turbodot::Picture = gpu.draw_program(&mut program).expect("the pages are drawn");
        assert_eq!(picture.at((plate.x + 3.0) as u32, (plate.y + 3.0) as u32), PLATE, "the toast's plate, over the home page");
    }

    /// Page placed inside another is laid out in the node it was placed
    /// in, takes a tap there and tells the page it sits in. It is freed
    /// with that page.
    ///
    /// Needed for a part with its own file and code that several pages
    /// show: a tool strip, a panel.
    #[test]
    fn a_page_placed_inside_a_page_lives_in_its_node_and_dies_with_its_page() {
        let mut program: Program<AppState> = started(AppState::default());
        frame(&mut program, 0.016);
        let place: Rect = program.ui().rect(HOME.footer);
        let footer: &PageUi = program.placed_ui(HOME.footer).expect("the footer page is placed");
        assert!(program.placed::<FooterPage>(HOME.footer).is_some());
        assert!(program.is_drawn::<FooterPage>());
        let button: Rect = footer.rect(FOOTER.add_sample);
        assert!(place.w > 0.0 && place.contains(button.x, button.y) && place.contains(button.x + button.w - 0.5, button.y + button.h - 0.5), "its button {button:?} sits inside the place {place:?}");
        assert_eq!(footer.text_of(FOOTER.note_count), Some("0 notes"), "its `show` ran");

        let (x, y): (f32, f32) = (button.x + button.w / 2.0, button.y + button.h / 2.0);
        assert!(program.touch_finger(Phase::Down, x, y, 1.0, 1), "the placed page takes the finger");
        assert!(program.touch_finger(Phase::Up, x, y, 1.05, 1));
        frame(&mut program, 0.016);
        assert_eq!(program.app().notes.len(), 1, "its handler ran");
        assert_eq!(program.placed_ui(HOME.footer).expect("the footer page").text_of(FOOTER.note_count), Some("1 notes"));
        assert_eq!(program.ui().rows(HOME.notes).len(), 1, "the page it sits in was told");

        // Settings page covers the home page: what is placed in it is not
        // drawn either.
        open_settings(&mut program, 1.5);
        assert!(!program.is_drawn::<FooterPage>(), "covered with its page");
        program.back();
        let_the_slides_end(&mut program);
        assert!(program.is_drawn::<FooterPage>(), "back in view with its page");

        // The edit page places a footer of its own: two more pages while
        // it lives, none after.
        let live_before: usize = program.live_page_count();
        program.tap(HOME.add_note, 2.0);
        let_the_slides_end(&mut program);
        frame(&mut program, 0.016);
        assert_eq!(program.live_page_count(), live_before + 2);
        let place: Rect = program.ui().rect(EDIT.footer);
        let button: Rect = program.placed_ui(EDIT.footer).expect("the edit page's footer").rect(FOOTER.add_sample);
        assert!(place.contains(button.x, button.y), "{button:?} in {place:?}");
        program.back();
        program.back();
        let_the_slides_end(&mut program);
        assert_eq!(program.live_page_count(), live_before, "the placed page was freed with its page");
        assert!(program.placed_ui(HOME.footer).is_some(), "the home page's own footer stays");
    }

    /// Page placed inside another is drawn over the node it was placed
    /// in, and not while its page is covered.
    ///
    /// Ignored unless asked for: needs a GPU adapter.
    #[test]
    #[ignore = "needs a GPU adapter: run with --include-ignored on a machine that has one"]
    fn a_page_placed_inside_a_page_is_drawn_with_its_page() {
        const PLATE: [u8; 4] = [0x2C, 0x2F, 0x38, 0xFF];
        let mut program: Program<AppState> = started(AppState::default());
        frame(&mut program, 0.016);
        let gpu: turbodot::Gpu = turbodot::Gpu::new().expect("this test needs a GPU adapter");
        let button: Rect = program.placed_ui(HOME.footer).expect("the footer page").rect(FOOTER.add_sample);
        let (x, y): (u32, u32) = ((button.x + 3.0) as u32, (button.y + button.h / 2.0) as u32);
        let picture: turbodot::Picture = gpu.draw_program(&mut program).expect("the pages are drawn");
        assert_eq!(picture.at(x, y), PLATE, "the placed page's button, left of its text");

        open_settings(&mut program, 1.0);
        let picture: turbodot::Picture = gpu.draw_program(&mut program).expect("the pages are drawn");
        assert_ne!(picture.at(x, y), PLATE, "covered with its page by the settings page");
    }

    /// Touch says whether a page took it. Page takes one on its empty
    /// space, unless it says otherwise: the paper page lets it go on to
    /// the host, and still takes one on its button. Finger keeps its id
    /// from down to up.
    ///
    /// Needed for a painting under the pages: host hands it the fingers
    /// no page took.
    #[test]
    fn a_touch_says_whether_a_page_took_it() {
        let middle = |rect: Rect| -> (f32, f32) { (rect.x + rect.w / 2.0, rect.y + rect.h / 2.0) };
        let mut program: Program<AppState> = started(AppState::default());
        let (x, y): (f32, f32) = middle(program.ui().rect(HOME.title));
        assert!(program.touch_finger(Phase::Down, x, y, 1.0, 7), "a page takes a touch beside its buttons");
        assert!(program.touch_finger(Phase::Up, x, y, 1.05, 7));
        assert!(!program.touch_finger(Phase::Up, x, y, 1.1, 9), "a finger that never came down belongs to no page");

        program.tap(HOME.open_paper, 2.0);
        let_the_slides_end(&mut program);
        assert!(program.page::<PaperPage>().is_some());
        let (x, y): (f32, f32) = middle(program.ui().rect(crate::ui::paper_page::NAMES.hint));
        assert!(!program.touch_finger(Phase::Down, x, y, 3.0, 7), "a touch on the paper goes on to the host");
        assert!(!program.touch_finger(Phase::Up, x, y, 3.05, 7));
        let (x, y): (f32, f32) = middle(program.ui().rect(crate::ui::paper_page::NAMES.go_back));
        assert!(program.touch_finger(Phase::Down, x, y, 4.0, 3), "its button takes a finger");
        assert!(program.touch_finger(Phase::Up, x, y, 4.05, 3));
        let_the_slides_end(&mut program);
        assert!(program.page::<PaperPage>().is_none(), "and the button did its work");
    }

    /// Screen's facts and language a host hands a program reach every
    /// page, also one opened later, and stay: frame of another size does
    /// not put program's own guess back.
    ///
    /// Needed on a phone: with insets lost at a turn of the screen, page
    /// draws under the notch.
    #[test]
    fn the_hosts_screen_facts_and_language_reach_every_page_and_stay() {
        use std::sync::Arc;
        use turbodot::gogo::{Locale, Orientation, ScreenClass};
        const SAFE: [f32; 4] = [0.0, 32.0, 0.0, 16.0];

        let mut program: Program<AppState> = started(AppState::default());
        assert_eq!(program.ui().runtime.safe, [0.0; 4], "no insets until a host says some");
        assert!(program.ui().runtime.locale.is_none());
        program.set_screen_and_insets(SAFE, Orientation::Landscape, ScreenClass::LandscapeSmall);
        program.set_locale(Some(Arc::new(Locale::english(crate::ui::KEYS))), "bg");
        frame(&mut program, 0.016);
        assert_eq!(program.ui().runtime.safe, SAFE);
        assert!(program.ui().runtime.locale.is_some());

        // Tall frame: program's own guess would be an upright screen
        // with no insets.
        program.frame(300.0, 900.0, 0.016);
        assert_eq!(program.ui().runtime.safe, SAFE, "host's insets stay");

        program.tap(HOME.add_note, 1.0);
        assert!(program.page::<EditPage>().is_some());
        program.frame(300.0, 900.0, 0.016);
        assert_eq!(program.ui().runtime.safe, SAFE, "page opened later");
        assert!(program.ui().runtime.locale.is_some(), "page opened later");
    }

    /// Picture page's code makes has its size in the frame after it was
    /// made, with no GPU; a second one of another size moves the page
    /// again.
    ///
    /// Needed because such a page runs in a test like any other: its
    /// layout is checked with no GPU.
    #[test]
    fn a_picture_made_in_code_has_its_size_with_no_gpu() {
        let mut program: Program<AppState> = started(AppState::default());
        open_settings(&mut program, 1.0);
        let side: f32 = MADE_PICTURE_SIDE_PX as f32;
        let made: Rect = program.ui().rect(SETTINGS.made_in_code);
        assert_eq!((made.w, made.h), (side, side), "the size the page's code said");
        program.ui_mut().set_picture(SETTINGS.made_in_code, 20, 6, &[0xFF; 20 * 6 * 4]);
        frame(&mut program, 0.016);
        let made: Rect = program.ui().rect(SETTINGS.made_in_code);
        assert_eq!((made.w, made.h), (20.0, 6.0), "the second picture's size");
    }

    /// Picture page's code makes shows its pixels where its Image sits,
    /// in every picture of the page; a second one takes its place; page
    /// freed, then made again, shows it again.
    ///
    /// Ignored unless asked for: needs a GPU adapter.
    #[test]
    #[ignore = "needs a GPU adapter: run with --include-ignored on a machine that has one"]
    fn a_picture_made_in_code_shows_its_pixels() {
        const RED: [u8; 4] = [0xE0, 0x20, 0x20, 0xFF];
        let center = |rect: Rect| -> (u32, u32) { ((rect.x + rect.w / 2.0) as u32, (rect.y + rect.h / 2.0) as u32) };

        let mut program: Program<AppState> = started(AppState::default());
        let gpu: turbodot::Gpu = turbodot::Gpu::new().expect("this test needs a GPU adapter");
        open_settings(&mut program, 1.0);
        let (x, y): (u32, u32) = center(program.ui().rect(SETTINGS.made_in_code));
        for round in 0..2 {
            let picture: turbodot::Picture = gpu.draw_program(&mut program).expect("the pages are drawn");
            assert_eq!(picture.at(x, y), MADE_PICTURE_COLOR, "the pixels the page made, picture {round} of the page");
        }

        let red: Vec<u8> = RED.repeat((MADE_PICTURE_SIDE_PX * MADE_PICTURE_SIDE_PX) as usize);
        program.ui_mut().set_picture(SETTINGS.made_in_code, MADE_PICTURE_SIDE_PX, MADE_PICTURE_SIDE_PX, &red);
        frame(&mut program, 0.016);
        let picture: turbodot::Picture = gpu.draw_program(&mut program).expect("the pages are drawn");
        assert_eq!(picture.at(x, y), RED, "the second picture in the first one's place");

        // Page freed: its picture leaves the GPU. Made again: shows again.
        program.back();
        let_the_slides_end(&mut program);
        program.free_cached_pages();
        gpu.draw_program(&mut program).expect("the pages are drawn");
        open_settings(&mut program, 5.0);
        let (x, y): (u32, u32) = center(program.ui().rect(SETTINGS.made_in_code));
        let picture: turbodot::Picture = gpu.draw_program(&mut program).expect("the pages are drawn");
        assert_eq!(picture.at(x, y), MADE_PICTURE_COLOR, "the page made again shows its own picture");
    }

    /// Picture loads late only when every line that names it says
    /// `loading=lazy`: the jpeg does, the png is also named by a line
    /// without the mark.
    ///
    /// Needed because a page that shows a picture at once wants it in its
    /// first frame: decoded late, user sees an empty place first.
    #[test]
    fn a_picture_loads_late_only_when_every_line_says_so() {
        let table: turbodot::pages::ImageTable = turbodot::pictures::image_table(crate::ui::BUILT.tables, crate::ui::BUILT.release_unused);
        let (icon, banner) = (table.get("res://assets/note.png").expect("the png"), table.get("res://assets/banner.jpg").expect("the jpeg"));
        assert!(!table.is_lazy(icon), "the home page names the png with no mark");
        assert!(table.is_lazy(banner), "every line that names the jpeg says lazy");
        assert_eq!((table.size(icon), table.size(banner)), (Some((24, 24)), Some((64, 16))));
        assert_eq!(table.pending_count(), 2, "both still to decode");
    }

    /// Pictures show in a picture of the pages: a png, and a jpeg marked
    /// `loading=lazy`, each in its own colors where its Image sits. Second
    /// picture of same page shows them again.
    ///
    /// Ignored unless asked for: needs a GPU adapter.
    #[test]
    #[ignore = "needs a GPU adapter: run with --include-ignored on a machine that has one"]
    fn pictures_show_where_their_images_sit() {
        const ORANGE: [u8; 4] = [255, 136, 0, 255];
        const BLUE: [u8; 4] = [32, 96, 192, 255];
        let center = |rect: Rect| -> (u32, u32) { ((rect.x + rect.w / 2.0) as u32, (rect.y + rect.h / 2.0) as u32) };

        let mut program: Program<AppState> = started(AppState::default());
        let gpu: turbodot::Gpu = turbodot::Gpu::new().expect("this test needs a GPU adapter");
        let (x, y): (u32, u32) = center(program.ui().rect(HOME.note_icon));
        for round in 0..2 {
            let picture: turbodot::Picture = gpu.draw_program(&mut program).expect("the pages are drawn");
            assert_eq!(picture.at(x, y), ORANGE, "the png, picture {round} of the page");
        }

        open_settings(&mut program, 1.0);
        let (x, y): (u32, u32) = center(program.ui().rect(SETTINGS.banner));
        let picture: turbodot::Picture = gpu.draw_program(&mut program).expect("the pages are drawn");
        let seen: [u8; 4] = picture.at(x, y);
        // A jpeg does not keep a color to the last digit.
        assert!((0..4).all(|i| seen[i].abs_diff(BLUE[i]) <= 3), "the lazy jpeg: {seen:?}");
    }
    /// Touch before first frame lands on nothing, and outline is empty.
    #[test]
    fn a_touch_before_the_first_frame_does_nothing() {
        let mut program: Program<AppState> = Program::new(&crate::ui::BUILT, AppState::default(), HomePage).expect("the pages can be shown");
        assert_eq!(program.ui().outline(), "");
        program.touch(Phase::Down, 50.0, 600.0, 0.0);
        program.touch(Phase::Up, 50.0, 600.0, 0.05);
        assert_eq!(program.depth(), 1);
        frame(&mut program, 0.0);
        assert!(program.ui().outline().starts_with("View homePage x=0 y=0 w=480 h=640"), "{}", program.ui().outline());
    }

    /// Pages drawn on a real GPU at two device pixels per epx: twice the
    /// pixels, page's own colors where page says, same bytes in a PNG read
    /// back from disk.
    ///
    /// Ignored unless asked for: needs a GPU adapter, which a hosted test
    /// machine may not have, and a test that returned early there would
    /// count as passed without having run. On a machine with a GPU:
    ///
    ///     cargo test -p turbodot-example-notes -- --include-ignored
    #[test]
    #[ignore = "needs a GPU adapter: run with --include-ignored on a machine that has one"]
    fn the_pages_are_drawn_to_a_picture_in_device_pixels() {
        const BACK: [u8; 4] = [0x17, 0x18, 0x1C, 0xFF];
        const PLATE: [u8; 4] = [0x2C, 0x2F, 0x38, 0xFF];

        let mut program: Program<AppState> = started(AppState::default());
        program.set_epx(2.0);
        frame(&mut program, 0.0);
        let gpu: turbodot::Gpu = turbodot::Gpu::new().expect("this test needs a GPU adapter");
        let picture: turbodot::Picture = gpu.draw_program(&mut program).expect("the pages are drawn");
        assert_eq!((picture.width, picture.height), (960, 1280));
        assert_eq!(picture.at(950, 20), BACK, "the page's background");
        let button: Rect = program.ui().rect(HOME.add_note);
        assert_eq!(picture.at(((button.x + 3.0) * 2.0) as u32, ((button.y + button.h / 2.0) * 2.0) as u32), PLATE, "the button's plate, left of its text");

        let file = std::path::PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/../../target/pictures/notes_home_page.png"));
        picture.save_png(&file).expect("the picture is saved");
        let decoder = png::Decoder::new(std::io::BufReader::new(std::fs::File::open(&file).expect("the saved file")));
        let mut reader = decoder.read_info().expect("a PNG");
        let mut bytes: Vec<u8> = vec![0; reader.output_buffer_size().expect("a size")];
        let info = reader.next_frame(&mut bytes).expect("the pixels");
        assert_eq!((info.width, info.height, info.color_type), (960, 1280, png::ColorType::Rgba));
        assert_eq!(&bytes[..info.buffer_size()], &picture.pixels.Bytes[..], "the file holds the picture's bytes");

        // Overlay: home page's button still shows through its dark sheet,
        // darker than before and not black.
        program.tap(HOME.clear_notes, 1.0);
        let_the_slides_end(&mut program);
        let picture: turbodot::Picture = gpu.draw_program(&mut program).expect("the pages are drawn");
        let seen: [u8; 4] = picture.at(((button.x + 3.0) * 2.0) as u32, ((button.y + button.h / 2.0) * 2.0) as u32);
        assert!(seen[0] > 0 && seen[0] < PLATE[0] && seen[3] == 0xFF, "home page's button under the overlay's sheet: {seen:?}");
    }
}
