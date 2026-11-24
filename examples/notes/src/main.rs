//! Small notes program on TurboDot, in the shape every client has: one
//! line builds pages (`build.rs`), one declares them, `main` starts the
//! program, plus a struct and a `.gogo` file per page.
//!
//! - `home_page`: the notes. Opens the other two pages.
//! - `edit_page`: a new note. Asks once before back drops it.
//! - `settings_page`: cached page. Later open shows same view.
//! - `clear_page`: overlay page. Home page stays in view under it.
//! - `dropped_page`: overlay page that takes the clear page's place.
//! - `paper_page`: page with a GPU picture of its own under its nodes.
//! - `footer_page`: page placed inside the home page and the edit page.
//! - `toast_page`: layer above every page, shows a line for a moment.
//!
//! Run it with `cargo run -p turbodot-example-notes`. Esc is back.

mod clear_page;
mod dropped_page;
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
    /// What pages wrote when they arrived and when they left, in order.
    /// A test reads it.
    pub page_log: Vec<&'static str>,
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
    use crate::dropped_page::DroppedPage;
    use crate::edit_page::EditPage;
    use crate::footer_page::FooterPage;
    use crate::toast_page::{TOAST_SEC, ToastPage};
    use crate::paper_page::{LINE, LINE_EVERY_TEXELS, PAPER, PAPER_SIDE_TEXELS, PaperPage};
    use crate::settings_page::{MADE_PICTURE_COLOR, MADE_PICTURE_SIDE_PX, SettingsPage};
    use crate::ui::{clear_page, dropped_page, edit_page, footer_page, home_page, settings_page, toast_page};

    const HOME: home_page::Names = home_page::NAMES;
    const CLEAR: clear_page::Names = clear_page::NAMES;
    const DROPPED: dropped_page::Names = dropped_page::NAMES;
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

    /// Frames until no page is on its way in or out any more.
    fn let_the_slides_end(program: &mut Program<AppState>) {
        for _ in 0..60 {
            frame(program, 0.016);
            if !program.is_page_arriving_or_leaving() {
                return;
            }
        }
        panic!("a page is still on its way after a second");
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

    /// Page made ahead of its first open plays its nodes' `on:show` at
    /// that open, once: not while it waits unseen, and not at a later
    /// open, which shows same view.
    ///
    /// Needed because an entrance played when page was made is over
    /// before anyone sees page.
    #[test]
    fn a_page_made_ahead_plays_its_entrances_at_its_first_open() {
        use turbodot::gogo::Hook;
        let is_count_coming_in = |program: &Program<AppState>| -> bool { program.ui().runtime.is_hook_playing(&program.ui().page, SETTINGS.note_count, Hook::Show) };
        let mut program: Program<AppState> = started(AppState::default());
        assert_eq!(program.cached_page_count(), 1, "the settings page is made ahead");
        frame(&mut program, 0.016);
        assert!(!program.frame(SIZE_EPX.0, SIZE_EPX.1, 0.016), "nothing plays while it waits");

        program.tap(HOME.open_settings, 1.0);
        frame(&mut program, 0.016);
        assert!(is_count_coming_in(&program), "its first open: the count comes in");
        let_the_slides_end(&mut program);
        assert!(program.back());
        let_the_slides_end(&mut program);

        program.tap(HOME.open_settings, 2.0);
        frame(&mut program, 0.016);
        assert!(program.is_page_arriving_or_leaving() && !is_count_coming_in(&program), "a later open: page slides in, the count is as it was");
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
        assert!(program.is_page_arriving_or_leaving(), "the settings page is on its way out");
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
        let_the_slides_end(&mut program);
        program.tap(DROPPED.got_it, 5.0);
        assert_eq!(program.ui().rows(HOME.notes).len(), 0, "home page wrote its list again");
    }

    /// Page opened in place of another: that one leaves unasked, and
    /// back from the new page shows the page under both.
    ///
    /// Needed for a dialog that leads to another: user closes one
    /// dialog, not a pile.
    #[test]
    fn a_page_opened_in_place_of_another_takes_its_place() {
        let mut program: Program<AppState> = started(AppState { notes: vec!["Note 1".to_string()], ..AppState::default() });
        let live_at_home: usize = program.live_page_count();
        program.tap(HOME.clear_notes, 1.0);
        let_the_slides_end(&mut program);

        program.tap(CLEAR.clear_all, 2.0);
        assert!(program.page::<DroppedPage>().is_some(), "the new page is on screen");
        assert_eq!(program.depth(), 2, "in the clear page's place, not over it");
        assert!(program.is_page_arriving_or_leaving() && program.is_drawn::<ClearPage>(), "the clear page slides out");
        let_the_slides_end(&mut program);
        assert!(!program.is_drawn::<ClearPage>());
        assert_eq!(program.live_page_count(), live_at_home + 1, "the clear page is freed");
        assert_eq!(program.app().page_log.last(), Some(&"clear left"), "its `left` ran, with no back request asked of it");

        assert!(program.back());
        assert!(program.page::<HomePage>().is_some(), "back shows the page under both");
        assert_eq!(program.ui().rows(HOME.notes).len(), 0, "which the clear page told to write its list again");
    }

    /// Page is told when it arrived (in place, no page sliding) and when
    /// it left (off screen), with pages placed in it. Cached page too.
    ///
    /// Needed for what must wait for a slide's end: a native view shown
    /// at open is on screen before the page it belongs to.
    #[test]
    fn a_page_is_told_when_it_arrived_and_when_it_left() {
        let mut program: Program<AppState> = started(AppState::default());
        assert_eq!(program.app().page_log, ["home arrived", "footer arrived"], "first page arrives with no slide");

        program.app_mut().page_log.clear();
        program.tap(HOME.add_note, 1.0);
        frame(&mut program, 0.016);
        assert_eq!(program.app().page_log, [""; 0], "nothing while the edit page slides in");
        let_the_slides_end(&mut program);
        assert_eq!(program.app().page_log, ["edit arrived", "footer arrived"], "at its slide's end, with its own footer");

        program.app_mut().page_log.clear();
        program.back();
        program.back();
        frame(&mut program, 0.016);
        assert_eq!(program.app().page_log, [""; 0], "nothing while it slides out");
        let_the_slides_end(&mut program);
        assert_eq!(program.app().page_log, ["edit left", "home arrived", "footer arrived"], "page that left first, then page that shows");

        // Back before the slide in ended: page never arrived, and home
        // page arrives again.
        program.app_mut().page_log.clear();
        program.tap(HOME.add_note, 2.0);
        frame(&mut program, 0.016);
        program.back();
        program.back();
        let_the_slides_end(&mut program);
        assert_eq!(program.app().page_log, ["edit left", "home arrived", "footer arrived"]);

        // Cached page: `left` each time it slid out, and it stays cached.
        program.app_mut().page_log.clear();
        open_settings(&mut program, 3.0);
        program.back();
        let_the_slides_end(&mut program);
        assert_eq!(program.app().page_log, ["settings arrived", "settings left", "home arrived", "footer arrived"]);
        assert_eq!(program.cached_page_count(), 1);
    }

    /// Program follows its pages' files in a folder its host names: a
    /// save there shows in the open page.
    ///
    /// Needed on a phone, where the files are not where build read
    /// them.
    #[test]
    fn a_program_follows_page_files_in_a_folder_its_host_names() {
        let folder = std::path::PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/../../target/notes_pages_followed"));
        let _ = std::fs::remove_dir_all(&folder);
        std::fs::create_dir_all(&folder).expect("a folder for the pages' files");
        for entry in std::fs::read_dir(concat!(env!("CARGO_MANIFEST_DIR"), "/ui")).expect("the pages' folder") {
            let path = entry.expect("a file").path();
            std::fs::copy(&path, folder.join(path.file_name().expect("a file name"))).expect("the file is copied");
        }
        let mut program: Program<AppState> = Program::following_page_files_in(&crate::ui::BUILT, AppState::default(), HomePage, &folder).expect("the pages can be shown");
        frame(&mut program, 0.0);
        assert_eq!(program.ui().text_of(HOME.title), Some("Notes"));

        let file = folder.join("home_page.gogo");
        let saved: String = std::fs::read_to_string(&file).expect("the home page's file").replace("text=\"Notes\"", "text=\"My notes\"");
        std::fs::write(&file, saved).expect("the save");
        for _ in 0..200 {
            std::thread::sleep(std::time::Duration::from_millis(25));
            frame(&mut program, 0.016);
            if program.ui().text_of(HOME.title) == Some("My notes") {
                return;
            }
        }
        panic!("the save did not show within five seconds: the title reads {:?}", program.ui().text_of(HOME.title));
    }

    /// Page whose file names no anim for its root is in place at once
    /// and gone at once, on back and when another page takes its place.
    /// Page under it does not move.
    ///
    /// Needed because how a page comes and goes is its file's to say:
    /// engine adds no motion of its own.
    #[test]
    fn a_page_whose_file_names_no_anim_comes_and_goes_at_once() {
        /// `dropped_page.gogo` as a dialog that leads to the clear page.
        struct SwapPage;
        impl Page<AppState> for SwapPage {
            const PAGE: &'static BuiltPage = &dropped_page::PAGE;
            type Handler = dropped_page::Handler;
            const KIND: PageKind = PageKind::Overlay;
            fn handle(&mut self, host: &mut Host<AppState>, event: PageEvent<dropped_page::Handler>) {
                match event.handler {
                    dropped_page::Handler::GotIt => host.nav.replace(ClearPage),
                }
            }
        }
        /// `home_page.gogo` as a fullscreen page over the home page: its
        /// root has no `on:show` and no `on:hide`.
        struct PlainPage;
        impl Page<AppState> for PlainPage {
            const PAGE: &'static BuiltPage = &home_page::PAGE;
            type Handler = home_page::Handler;
            fn handle(&mut self, _host: &mut Host<AppState>, _event: PageEvent<home_page::Handler>) {}
        }
        let mut program: Program<AppState> = started(AppState::default());
        let live_at_home: usize = program.live_page_count();
        program.go(DroppedPage);
        assert!(program.page::<DroppedPage>().is_some());
        assert!(!program.is_page_arriving_or_leaving(), "in place at once");
        frame(&mut program, 0.016);
        assert_eq!(program.ui().drawn_pose(DROPPED.dropped_page), None, "where its file has it, solid");

        program.tap(DROPPED.got_it, 1.0);
        assert!(program.page::<HomePage>().is_some());
        assert!(!program.is_page_arriving_or_leaving(), "gone at once, and the page under it does not move");
        assert_eq!(program.live_page_count(), live_at_home, "freed at once");
        assert!(program.app().page_log.contains(&"dropped left"), "its `left` ran");

        // Fullscreen page gone at once: the home page under it has
        // nothing to come back from, so its `on:uncover` does not play.
        program.go(PlainPage);
        frame(&mut program, 0.016);
        assert!(!program.is_page_arriving_or_leaving() && program.drawn_page_count() == 1, "in place at once, over the home page");
        assert!(program.back());
        frame(&mut program, 0.016);
        assert!(!program.is_page_arriving_or_leaving() && program.ui().drawn_pose(HOME.home_page).is_none(), "the home page is there at once, in place");

        program.go(SwapPage);
        frame(&mut program, 0.016);
        program.tap(DROPPED.got_it, 2.0);
        assert!(program.page::<ClearPage>().is_some(), "the clear page takes its place");
        assert_eq!(program.live_page_count(), live_at_home + 1, "the page with no anim is freed at once");
        assert!(program.is_page_arriving_or_leaving(), "the clear page comes in as its file says");
    }

    /// Page comes and goes as its file says: root's `on:show` plays as
    /// page opens, its `on:hide` as it leaves, and page under a leaving
    /// fullscreen page plays its `on:uncover`. Page under stays drawn
    /// for as long as the page over it is on its way.
    #[test]
    fn a_page_comes_and_goes_as_its_file_says() {
        let mut program: Program<AppState> = started(AppState::default());
        program.tap(HOME.open_settings, 1.0);
        frame(&mut program, 0.016);
        let coming: turbodot::pages::PagePose = program.ui().drawn_pose(SETTINGS.settings_page).expect("the settings page is on its way in");
        assert!(coming.offset_x > 40.0 && coming.offset_x < 60.0 && coming.alpha > 0.0 && coming.alpha < 0.2, "from the right, still clear: {coming:?}");
        assert_eq!(program.drawn_page_count(), 2, "the home page shows under it");
        // Engine moves page's finished picture: page's own draw list
        // stays where page is laid out.
        assert!(matches!(program.ui().draws.first(), Some(turbodot::gogo::Draw::Quad { rect, color, .. }) if rect.x == 0.0 && color.0[3] == 255), "the page's own picture is in place, solid: {:?}", program.ui().draws.first());
        let_the_slides_end(&mut program);
        assert_eq!(program.ui().drawn_pose(SETTINGS.settings_page), None, "in place, solid");
        assert_eq!(program.drawn_page_count(), 1, "and covers the home page");

        assert!(program.back());
        frame(&mut program, 0.1);
        assert!(program.is_drawn::<SettingsPage>() && program.is_page_arriving_or_leaving(), "the settings page is on its way out");
        let back: turbodot::pages::PagePose = program.ui().drawn_pose(HOME.home_page).expect("the home page comes back");
        assert!(back.offset_x < 0.0 && back.offset_x > -30.0 && back.alpha > 0.0, "from the left: {back:?}");
        let_the_slides_end(&mut program);
        assert_eq!(program.ui().drawn_pose(HOME.home_page), None);
        assert_eq!((program.drawn_page_count(), program.cached_page_count()), (1, 1), "the settings page is cached");

        // Cached page comes in again the same way: what its way out
        // moved is not left standing.
        program.tap(HOME.open_settings, 2.0);
        frame(&mut program, 0.016);
        let again: turbodot::pages::PagePose = program.ui().drawn_pose(SETTINGS.settings_page).expect("on its way in again");
        assert!((again.offset_x - coming.offset_x).abs() < 0.01 && (again.alpha - coming.alpha).abs() < 0.01, "{again:?} against {coming:?}");
        let_the_slides_end(&mut program);
        assert_eq!(program.ui().drawn_pose(SETTINGS.settings_page), None);
    }

    /// What a page writes when it is told it arrived is laid out in that
    /// same frame.
    ///
    /// Needed because the frame in which a page's way ends may be the
    /// last one a still program asks for: a write laid out a frame
    /// later would show only at next touch.
    #[test]
    fn what_a_page_writes_when_it_arrives_is_laid_out_in_that_frame() {
        /// `settings_page.gogo` as a page that writes a longer title once it is
        /// in place.
        struct ArrivingPage;
        impl Page<AppState> for ArrivingPage {
            const PAGE: &'static BuiltPage = &settings_page::PAGE;
            type Handler = settings_page::Handler;
            fn handle(&mut self, _host: &mut Host<AppState>, _event: PageEvent<settings_page::Handler>) {}
            fn arrived(&mut self, host: &mut Host<AppState>) {
                host.ui.set_text(SETTINGS.title, "Settings, in place now");
            }
        }
        let mut program: Program<AppState> = started(AppState::default());
        program.go(ArrivingPage);
        frame(&mut program, 0.016);
        let short: f32 = program.ui().rect(SETTINGS.title).w;
        let_the_slides_end(&mut program);
        let long: f32 = program.ui().rect(SETTINGS.title).w;
        assert!(long > short * 2.0, "the frame its way ended laid the longer title out: {short} then {long}");
    }

    /// Page placed in a node is drawn where an anim has that node now,
    /// as see-through as it: it comes in with the page it is placed in.
    #[test]
    fn a_placed_page_moves_and_fades_with_the_node_it_is_placed_in() {
        let mut program: Program<AppState> = started(AppState::default());
        assert_eq!(program.placed_ui(HOME.footer).expect("the home page's footer").pose, None, "in place while nothing moves");
        program.tap(HOME.add_note, 1.0);
        frame(&mut program, 0.016);
        frame(&mut program, 0.016);
        let page: turbodot::pages::PagePose = program.ui().drawn_pose(EDIT.edit_page).expect("the edit page is on its way in");
        assert_eq!(program.placed_ui(EDIT.footer).expect("the edit page's footer").pose, Some(page), "the footer is where its page is");
        let_the_slides_end(&mut program);
        frame(&mut program, 0.016);
        assert_eq!(program.placed_ui(EDIT.footer).expect("the edit page's footer").pose, None, "and in place with it");
    }

    /// Placed page whose own root names an anim plays it where it is
    /// placed: engine moves its finished picture, as it does for a page
    /// of the back stack.
    #[test]
    fn a_placed_page_plays_its_own_roots_anim() {
        /// `home_page.gogo` as a first page that places the clear page,
        /// whose root slides in, where its footer goes.
        struct HolderPage;
        impl Page<AppState> for HolderPage {
            const PAGE: &'static BuiltPage = &home_page::PAGE;
            type Handler = home_page::Handler;
            fn loaded(&mut self, host: &mut Host<AppState>) {
                host.nav.place(HOME.footer, ClearPage);
            }
            fn handle(&mut self, _host: &mut Host<AppState>, _event: PageEvent<home_page::Handler>) {}
        }
        let mut program: Program<AppState> = Program::new(&crate::ui::BUILT, AppState::default(), HolderPage).expect("the pages can be shown");
        frame(&mut program, 0.0);
        frame(&mut program, 0.016);
        let coming: turbodot::pages::PagePose = program.placed_ui(HOME.footer).expect("the placed page").pose.expect("its root's `on:show` plays");
        assert!(coming.offset_x > 40.0 && coming.alpha < 0.2, "from the right, still clear: {coming:?}");
        for _ in 0..25 {
            frame(&mut program, 0.016);
        }
        assert_eq!(program.placed_ui(HOME.footer).expect("the placed page").pose, None, "in place, solid");
    }

    /// Page under a leaving overlay stays where it is: it never left the
    /// screen. Page under a leaving fullscreen page comes back.
    #[test]
    fn the_page_under_a_leaving_overlay_does_not_move() {
        let mut program: Program<AppState> = started(AppState { notes: vec!["Note 1".to_string()], ..AppState::default() });
        program.tap(HOME.clear_notes, 1.0);
        let_the_slides_end(&mut program);
        program.tap(CLEAR.keep_notes, 2.0);
        frame(&mut program, 0.016);
        assert!(program.is_page_arriving_or_leaving(), "the overlay is on its way out");
        assert!(program.page::<HomePage>().is_some() && program.ui().drawn_pose(HOME.home_page).is_none(), "the home page under it stays in place");
        let_the_slides_end(&mut program);

        open_settings(&mut program, 3.0);
        assert!(program.back());
        frame(&mut program, 0.016);
        assert!(program.page::<HomePage>().is_some() && program.ui().drawn_pose(HOME.home_page).is_some(), "under a fullscreen page it comes back");
    }

    /// Mouse is one pointer with several buttons: its press starts with
    /// the first button and ends when the last one lifts. A button other
    /// than the left one presses no button of a page.
    #[test]
    fn a_button_of_a_page_takes_the_left_mouse_button_alone() {
        use turbodot::gogo::{Buttons, Mods};
        let middle = |rect: Rect| -> (f32, f32) { (rect.x + rect.w / 2.0, rect.y + rect.h / 2.0) };
        let mut program: Program<AppState> = started(AppState::default());
        let (x, y): (f32, f32) = middle(program.ui().rect(HOME.add_note));
        // Middle button alone: nothing.
        program.mouse_button(Buttons::MIDDLE, true, x, y, 1.0, Mods::NONE);
        program.mouse_button(Buttons::MIDDLE, false, x, y, 1.1, Mods::NONE);
        assert_eq!(program.depth(), 1, "the middle button opens no page");
        // Left, then middle too, then left lifts: the press goes on until
        // the last button lifts.
        program.mouse_button(Buttons::PRIMARY, true, x, y, 3.0, Mods::NONE);
        program.mouse_button(Buttons::MIDDLE, true, x, y, 3.1, Mods::NONE);
        program.mouse_button(Buttons::PRIMARY, false, x, y, 3.2, Mods::NONE);
        assert_eq!(program.depth(), 1, "one press, and it has not ended");
        program.mouse_button(Buttons::MIDDLE, false, x, y, 3.3, Mods::NONE);
        assert_eq!(program.depth(), 2, "it ends when the last button lifts");
        // A lift with no button down (it went down outside the window) is nothing.
        assert!(!program.mouse_button(Buttons::PRIMARY, false, x, y, 4.0, Mods::NONE));
    }

    /// Mouse move and wheel turn say whether a page took them, as a
    /// touch does.
    ///
    /// Needed by host with pages of its own under the program's: on
    /// false it hands them the move.
    #[test]
    fn the_mouse_says_whether_a_page_is_under_it() {
        let middle = |rect: Rect| -> (f32, f32) { (rect.x + rect.w / 2.0, rect.y + rect.h / 2.0) };
        let mut program: Program<AppState> = started(AppState::default());
        let (x, y): (f32, f32) = middle(program.ui().rect(HOME.title));
        assert!(program.hover(x, y), "a page that takes every touch is under the mouse everywhere");
        assert!(program.wheel(x, y, 1.0, turbodot::gogo::Mods::NONE));

        program.tap(HOME.open_paper, 2.0);
        let_the_slides_end(&mut program);
        let (x, y): (f32, f32) = middle(program.ui().rect(crate::ui::paper_page::NAMES.hint));
        assert!(!program.hover(x, y), "the mouse over the paper is the host's");
        assert!(!program.wheel(x, y, 1.0, turbodot::gogo::Mods::NONE));
        let (x, y): (f32, f32) = middle(program.ui().rect(crate::ui::paper_page::NAMES.go_back));
        assert!(program.hover(x, y), "over the page's button it is the page's");
        assert!(program.wheel(x, y, 1.0, turbodot::gogo::Mods::NONE));
    }

    /// Wheel turn goes to the page that has something for it under the
    /// mouse, and says so. Page that lets touches through takes a turn
    /// over its `Scroll`, plain text in it included, and no turn beside
    /// it: nothing moves then, and host hears false.
    ///
    /// Needed by host with something of its own under the program's
    /// pages: a turn that scrolled a list must not zoom a painting
    /// too.
    #[test]
    fn a_wheel_turn_moves_one_thing_and_says_whether_a_page_took_it() {
        /// `settings_page.gogo` as a first page that lets touches
        /// beside its nodes through.
        struct GlassSettingsPage;
        impl Page<AppState> for GlassSettingsPage {
            const PAGE: &'static BuiltPage = &settings_page::PAGE;
            type Handler = settings_page::Handler;
            const TAKES_EVERY_TOUCH: bool = false;
            fn loaded(&mut self, host: &mut Host<AppState>) {
                let body: String = (1..=80).map(|line| format!("Line {line} of a long text.\n")).collect();
                host.ui.set_text(SETTINGS.body_text, &body);
            }
            fn handle(&mut self, _host: &mut Host<AppState>, _event: PageEvent<settings_page::Handler>) {}
        }
        let middle = |rect: Rect| -> (f32, f32) { (rect.x + rect.w / 2.0, rect.y + rect.h / 2.0) };
        let mut program: Program<AppState> = Program::new(&crate::ui::BUILT, AppState::default(), GlassSettingsPage).expect("the pages can be shown");
        frame(&mut program, 0.0);
        frame(&mut program, 0.016);
        let (x, y): (f32, f32) = middle(program.ui().rect(SETTINGS.body));
        assert!(program.wheel(x, y, -2.0, turbodot::gogo::Mods::NONE), "the turn over the text is the page's: its Scroll moves");
        frame(&mut program, 0.016);
        let scrolled: f32 = program.ui().scroll_of(SETTINGS.body)[1];
        assert!(scrolled > 0.0, "the text moved");

        let (x, y): (f32, f32) = middle(program.ui().rect(SETTINGS.title));
        assert!(!program.wheel(x, y, -2.0, turbodot::gogo::Mods::NONE), "over a label beside the Scroll the page has nothing for the turn");
        frame(&mut program, 0.016);
        assert_eq!(program.ui().scroll_of(SETTINGS.body)[1], scrolled, "and nothing moved");

        // Page that takes every touch keeps a turn it has nothing for.
        let mut program: Program<AppState> = started(AppState::default());
        open_settings(&mut program, 1.0);
        let (x, y): (f32, f32) = middle(program.ui().rect(SETTINGS.title));
        assert!(program.wheel(x, y, -2.0, turbodot::gogo::Mods::NONE), "a turn over a dialog is the dialog's");
        frame(&mut program, 0.016);
        assert_eq!(program.ui().scroll_of(SETTINGS.body)[1], 0.0, "with nothing under the mouse to turn, nothing moves");
    }

    /// Program says when a fullscreen page is in place: nothing under
    /// its pages shows then.
    ///
    /// Needed by host that draws under the program: it skips that
    /// drawing meanwhile.
    #[test]
    fn a_program_says_when_it_covers_what_is_under_it() {
        /// `clear_page.gogo` as a see-through first page.
        struct GlassPage;
        impl Page<AppState> for GlassPage {
            const PAGE: &'static BuiltPage = &clear_page::PAGE;
            type Handler = clear_page::Handler;
            const KIND: PageKind = PageKind::Overlay;
            fn handle(&mut self, _host: &mut Host<AppState>, _event: PageEvent<clear_page::Handler>) {}
        }
        let mut program: Program<AppState> = Program::new(&crate::ui::BUILT, AppState::default(), GlassPage).expect("the pages can be shown");
        frame(&mut program, 0.0);
        assert!(!program.covers_screen(), "a see-through first page covers nothing");
        program.go(SettingsPage::new(0));
        assert!(!program.covers_screen(), "a page that slides in lets what is under it show");
        let_the_slides_end(&mut program);
        assert!(program.covers_screen(), "in place, it covers");
        assert!(program.back());
        assert!(!program.covers_screen(), "on its way out it covers no more");

        assert!(started(AppState::default()).covers_screen(), "a fullscreen first page covers");
    }

    /// First page answers a back request like any page: it may use the
    /// request itself. `Program::back` says false only when first page
    /// said it leaves: then host does its own step (a phone puts the
    /// program in background).
    #[test]
    fn the_first_page_can_use_a_back_request_before_the_host_does() {
        /// `clear_page.gogo` as a first page with a panel that back
        /// closes first.
        struct PanelPage {
            is_panel_open: bool,
        }
        impl Page<AppState> for PanelPage {
            const PAGE: &'static BuiltPage = &clear_page::PAGE;
            type Handler = clear_page::Handler;
            fn handle(&mut self, _host: &mut Host<AppState>, _event: PageEvent<clear_page::Handler>) {}
            fn back(&mut self, _host: &mut Host<AppState>) -> Back {
                if !self.is_panel_open {
                    return Back::Leave;
                }
                self.is_panel_open = false;
                Back::Stay
            }
        }
        let mut program: Program<AppState> = Program::new(&crate::ui::BUILT, AppState::default(), PanelPage { is_panel_open: true }).expect("the pages can be shown");
        frame(&mut program, 0.0);
        assert!(program.back(), "first page used the request: host does nothing");
        assert!(!program.page::<PanelPage>().expect("the first page").is_panel_open);
        assert!(!program.back(), "first page leaves: host's turn");
        assert!(!program.back(), "and again, each time");
        assert_eq!(program.depth(), 1, "first page stays");
        assert!(program.page::<PanelPage>().is_some());
    }

    /// Host of a program reads which field is being edited, feeds it an
    /// on-screen keyboard's text, cuts, copies and pastes, and reads
    /// where the edit bar belongs: all without the page's own code.
    ///
    /// Needed on a phone: keyboard, clipboard and edit bar are the
    /// host's, the field is a page's.
    #[test]
    fn a_host_reaches_the_field_a_user_edits() {
        let mut program: Program<AppState> = started(AppState::default());
        program.tap(HOME.add_note, 1.0);
        let_the_slides_end(&mut program);
        frame(&mut program, 0.016);
        assert_eq!((program.caret_holder(), program.has_focus(), program.ime_text()), (None, false, None), "no field is being edited");

        program.tap(EDIT.note_text, 2.0);
        let (page, field) = program.caret_holder().expect("the tapped field holds the caret");
        assert_eq!(field, EDIT.note_text);
        assert!(program.has_focus());
        assert_eq!(program.page_ui(page).expect("the edit page").rect(field), program.ui().rect(EDIT.note_text), "host reads the field's place");
        assert_eq!(program.ime_text(), Some((String::new(), 0, 0)));

        // On-screen keyboard sends whole text with its caret.
        program.ime_replace("milk and eggs", 13, 13);
        assert_eq!(program.ui().text_of(EDIT.note_text), Some("milk and eggs"));
        assert_eq!(program.ui().text_of(EDIT.typed_count), Some("13 characters"), "the page was handed the change");
        assert_eq!(program.ime_text(), Some(("milk and eggs".to_string(), 13, 13)));

        // Copy, cut, paste.
        assert_eq!(program.selected_text(), None);
        program.select_all();
        assert_eq!(program.selected_text().as_deref(), Some("milk and eggs"));
        assert_eq!(program.cut_selected_text().as_deref(), Some("milk and eggs"));
        assert_eq!(program.ui().text_of(EDIT.note_text), Some(""));
        assert_eq!(program.ui().text_of(EDIT.typed_count), Some("0 characters"));
        assert_eq!(program.cut_selected_text(), None, "nothing selected, nothing cut");
        program.paste("bread");
        assert_eq!(program.ui().text_of(EDIT.note_text), Some("bread"));
        assert_eq!(program.ui().text_of(EDIT.typed_count), Some("5 characters"));

        // Finger held still on the field raises the edit bar.
        frame(&mut program, 0.016);
        assert_eq!(program.edit_bar_raised(), None);
        let place: Rect = program.ui().rect(EDIT.note_text);
        let (x, y): (f32, f32) = (place.x + 12.0, place.y + place.h / 2.0);
        assert!(program.touch_finger(Phase::Down, x, y, 3.0, 7));
        for _ in 0..4 {
            frame(&mut program, 0.2);
        }
        assert_eq!(program.edit_bar_raised(), Some((page, EDIT.note_text)));
        program.touch_finger(Phase::Up, x, y, 3.8, 7);
        program.dismiss_edit_bar();
        assert_eq!(program.edit_bar_raised(), None);

        // Page covered by another: its field holds no caret for the
        // host, so the keyboard goes down. Back in view: it does again.
        program.go(SettingsPage::new(0));
        let_the_slides_end(&mut program);
        assert_eq!((program.caret_holder(), program.ime_text()), (None, None), "the edit page is covered");
        program.back();
        let_the_slides_end(&mut program);
        assert_eq!(program.caret_holder(), Some((page, EDIT.note_text)));

        // What user typed is the note.
        program.tap(EDIT.save_note, 5.0);
        assert_eq!(program.app().notes, ["bread"]);
        let_the_slides_end(&mut program);
        assert_eq!(program.caret_holder(), None, "the freed page holds no caret");
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

    /// Page writes the draw commands of its `Surface draw=commands` when
    /// the Surface first shows and when its size changes, with no GPU.
    /// In every other frame the engine keeps them and does not ask.
    ///
    /// Needed because such a page runs in a test like any other: what it
    /// draws is checked with no GPU.
    #[test]
    fn a_page_writes_its_surface_s_commands_with_no_gpu() {
        use crate::paper_page::{MARGIN, MARGIN_FROM_RIGHT_EPX, MARGIN_WIDTH_EPX};
        use turbodot::gogo::{Color_Straight_Srgb_4xU8_RGBA, Draw};
        let margin: NodeId = crate::ui::paper_page::NAMES.margin.node();
        let line = |width: f32, height: f32| Draw::Quad { rect: Rect::new(width - MARGIN_FROM_RIGHT_EPX, 0.0, MARGIN_WIDTH_EPX, height), color: Color_Straight_Srgb_4xU8_RGBA(MARGIN), corner: 0.0, border: 0.0, border_color: Color_Straight_Srgb_4xU8_RGBA::TRANSPARENT };

        let mut program: Program<AppState> = started(AppState::default());
        program.tap(HOME.open_paper, 1.0);
        let_the_slides_end(&mut program);
        let (width, height): (f32, f32) = program.size();
        assert_eq!(program.ui().page.surface_commands(margin), [line(width, height)], "written when the Surface first showed");
        // The page's own list holds the Surface's place, not its commands.
        let whole: Rect = program.ui().rect(margin);
        assert_eq!((whole.w, whole.h), (width, height));
        assert!(program.ui().draws.contains(&Draw::Surface { rect: whole, surface: margin, alpha: 1.0 }), "{:?}", program.ui().draws);

        // A window of another size: asked again, for that size.
        program.frame(width - 100.0, height + 50.0, 0.016);
        assert_eq!(program.ui().page.surface_commands(margin), [line(width - 100.0, height + 50.0)]);
    }

    /// Page's own GPU picture comes in with its nodes: where an anim
    /// has page's root, and as see-through.
    ///
    /// Needed because the picture is not a node: with no word from the
    /// engine it would sit in place, solid, while its page slides in
    /// over it.
    ///
    /// Ignored unless asked for: needs a GPU adapter.
    #[test]
    #[ignore = "needs a GPU adapter: run with --include-ignored on a machine that has one"]
    fn a_pages_own_gpu_picture_moves_and_fades_with_its_root() {
        let mut program: Program<AppState> = started(AppState::default());
        let gpu: turbodot::Gpu = turbodot::Gpu::new().expect("this test needs a GPU adapter");
        program.go(PaperPage::default());
        frame(&mut program, 0.016);
        let pose: turbodot::pages::PagePose = program.ui().drawn_pose(crate::ui::paper_page::NAMES.paper_page).expect("the paper page is on its way in");
        assert!(pose.alpha < 0.2, "nearly clear on its first frame: {pose:?}");
        let picture: turbodot::Picture = gpu.draw_program(&mut program).expect("the pages are drawn");
        // The paper is in this picture, the first with the page in it:
        // its texture was handed over while the picture was drawn.
        assert!(program.ui().draws.iter().any(|draw| matches!(draw, turbodot::gogo::Draw::Image { .. })), "{:?}", program.ui().draws);
        let row_px: u32 = picture.height / PAPER_SIDE_TEXELS;
        let between_lines: [u8; 4] = picture.at(picture.width - 10, (LINE_EVERY_TEXELS - 6) * row_px + row_px / 2);
        assert!((0..3).all(|c| between_lines[c].abs_diff(PAPER[c]) > 60), "the home page shows through the paper: {between_lines:?}");
        // The margin line is draw commands of the same page: it moves and
        // fades with the page too. Read half way in, where the line is
        // neither clear nor solid.
        let mut pose: turbodot::pages::PagePose = pose;
        while pose.alpha < 0.3 {
            frame(&mut program, 0.016);
            pose = program.ui().drawn_pose(crate::ui::paper_page::NAMES.paper_page).expect("the paper page is still on its way in");
        }
        assert!(pose.alpha < 0.8, "half way in: {pose:?}");
        let picture: turbodot::Picture = gpu.draw_program(&mut program).expect("the pages are drawn");
        let margin_x: f32 = picture.width as f32 - crate::paper_page::MARGIN_FROM_RIGHT_EPX + pose.offset_x;
        assert!(margin_x >= 8.0 && margin_x < picture.width as f32 - 2.0, "the margin line is inside the picture: {margin_x} of {}, {pose:?}", picture.width);
        let y: u32 = (LINE_EVERY_TEXELS - 6) * row_px + row_px / 2;
        let on_the_margin: [u8; 4] = picture.at(margin_x.round() as u32, y);
        let beside_it: [u8; 4] = picture.at(margin_x.round() as u32 - 6, y);
        assert!(on_the_margin[0].abs_diff(crate::paper_page::MARGIN[0]) > 15, "the home page shows through the margin line: {on_the_margin:?}");
        // The line is drawn over the paper, each as clear as the page:
        // redder than the paper beside it, and not yet its own red.
        assert!(on_the_margin[0] > beside_it[0] + 20, "the margin line shows where the page is now: {on_the_margin:?} beside {beside_it:?}");
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
        // The margin line, draw commands of the page's second Surface:
        // over the paper, under the page's nodes, from top to bottom.
        let margin_x: u32 = picture.width - crate::paper_page::MARGIN_FROM_RIGHT_EPX as u32;
        for y in [2, picture.height / 2, picture.height - 3] {
            assert_eq!(picture.at(margin_x, y), crate::paper_page::MARGIN, "the margin line at row {y}");
            assert_ne!(picture.at(margin_x - 2, y), crate::paper_page::MARGIN, "paper left of the margin line at row {y}");
        }
        let title: Rect = program.ui().rect(crate::ui::paper_page::NAMES.title);
        let ink: usize = (title.y as u32..(title.y + title.h) as u32).flat_map(|y| (title.x as u32..(title.x + title.w) as u32).map(move |x| (x, y))).filter(|&(x, y)| picture.at(x, y) == [0x20, 0x20, 0x20, 0xFF]).count();
        assert!(ink > 20, "page's title is drawn over the paper: {ink} text-colored pixels");
        picture.save_png(&std::path::PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/../../target/pictures/notes_paper_page.png"))).expect("the picture is saved");

        let on_screen: u32 = program.app().paper_frames_drawn;
        assert!(on_screen >= 1);
        // A second picture is drawn by a renderer made new: the page
        // handed its texture over once, and the new renderer gets it
        // again.
        let second: turbodot::Picture = gpu.draw_program(&mut program).expect("the pages are drawn");
        assert_eq!(program.app().paper_frames_drawn, on_screen + 1, "each frame while page is on screen");
        assert_eq!(second.at(far_right, (LINE_EVERY_TEXELS - 6) * row_px + row_px / 2), PAPER, "the paper in the second picture of the page");

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
        assert!(program.ui().runtime.locale.is_some(), "a page has the language from the moment it is made: its `loaded` writes in it");
        assert_eq!(program.ui().runtime.safe, SAFE, "and the insets");
        program.frame(300.0, 900.0, 0.016);
        assert_eq!(program.ui().runtime.safe, SAFE, "page opened later");
        assert!(program.ui().runtime.locale.is_some(), "page opened later");
    }

    /// Picture page's code makes shows in its Surface from the frame
    /// after it was made, with no GPU. The Surface keeps the size its
    /// file gives it: a second picture of another size moves nothing.
    ///
    /// Needed because such a page runs in a test like any other: its
    /// layout is checked with no GPU.
    #[test]
    fn a_picture_made_in_code_shows_in_its_surface_with_no_gpu() {
        let mut program: Program<AppState> = started(AppState::default());
        open_settings(&mut program, 1.0);
        let side: f32 = MADE_PICTURE_SIDE_PX as f32;
        let made: Rect = program.ui().rect(SETTINGS.made_in_code.node());
        assert_eq!((made.w, made.h), (side, side), "the size the file says");
        let shows_a_picture = |program: &Program<AppState>| -> bool {
            let ui = program.ui();
            let rect: Rect = ui.rect(SETTINGS.made_in_code.node());
            ui.draws.iter().any(|draw| matches!(draw, turbodot::gogo::Draw::Image { rect: drawn, .. } if *drawn == rect))
        };
        assert!(shows_a_picture(&program), "the Surface draws the picture the page's code made");
        program.ui_mut().set_picture(SETTINGS.made_in_code, 20, 6, vec![0xFF; 20 * 6 * 4]);
        frame(&mut program, 0.016);
        let made: Rect = program.ui().rect(SETTINGS.made_in_code.node());
        assert_eq!((made.w, made.h), (side, side), "the second picture is fitted into the same Surface");
        assert!(shows_a_picture(&program));
    }

    /// Picture page's code makes shows its pixels where its Surface sits,
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
        let (x, y): (u32, u32) = center(program.ui().rect(SETTINGS.made_in_code.node()));
        for round in 0..2 {
            let picture: turbodot::Picture = gpu.draw_program(&mut program).expect("the pages are drawn");
            assert_eq!(picture.at(x, y), MADE_PICTURE_COLOR, "the pixels the page made, picture {round} of the page");
        }

        let red: Vec<u8> = RED.repeat((MADE_PICTURE_SIDE_PX * MADE_PICTURE_SIDE_PX) as usize);
        program.ui_mut().set_picture(SETTINGS.made_in_code, MADE_PICTURE_SIDE_PX, MADE_PICTURE_SIDE_PX, red);
        frame(&mut program, 0.016);
        let picture: turbodot::Picture = gpu.draw_program(&mut program).expect("the pages are drawn");
        assert_eq!(picture.at(x, y), RED, "the second picture in the first one's place");

        // Page freed: its picture leaves the GPU. Made again: shows again.
        program.back();
        let_the_slides_end(&mut program);
        program.free_cached_pages();
        gpu.draw_program(&mut program).expect("the pages are drawn");
        open_settings(&mut program, 5.0);
        let (x, y): (u32, u32) = center(program.ui().rect(SETTINGS.made_in_code.node()));
        let picture: turbodot::Picture = gpu.draw_program(&mut program).expect("the pages are drawn");
        assert_eq!(picture.at(x, y), MADE_PICTURE_COLOR, "the page made again shows its own picture");
    }

    /// Host tells a live page something from outside the pages, and
    /// hears when no such page is alive.
    #[test]
    fn a_host_tells_a_page_something() {
        let mut program: Program<AppState> = started(AppState::default());
        assert!(program.tell::<ToastPage>(|toast, host| toast.say(host, "From the host")));
        frame(&mut program, 0.016);
        assert_eq!(program.layer_ui::<ToastPage>().expect("the toast layer").text_of(TOAST.message), Some("From the host"));
        assert!(!program.tell::<EditPage>(|_, _| panic!("no edit page is alive")), "no such page: the host hears it");
    }

    /// Page closed and opened again within its slide out is alive
    /// twice. A tell reaches the one on screen, not the one leaving.
    #[test]
    fn a_tell_reaches_the_page_on_screen_and_not_one_of_its_type_that_is_leaving() {
        let mut program: Program<AppState> = started(AppState { notes: vec!["Note 1".to_string()], ..AppState::default() });
        let live_at_home: usize = program.live_page_count();
        program.tap(HOME.clear_notes, 1.0);
        let_the_slides_end(&mut program);
        assert!(program.back(), "the clear page uses the back request");
        assert!(program.is_page_arriving_or_leaving() && program.is_drawn::<ClearPage>(), "the clear page slides out");
        program.go(ClearPage);
        assert_eq!(program.live_page_count(), live_at_home + 2, "two clear pages are alive: one leaving, one new");
        assert!(program.tell::<ClearPage>(|_, host| host.ui.set_text(CLEAR.clear_all, "told")));
        assert_eq!(program.ui().text_of(CLEAR.clear_all), Some("told"), "the page on screen was told");
        let_the_slides_end(&mut program);
        assert_eq!(program.live_page_count(), live_at_home + 1, "the one that left is freed");
        assert_eq!(program.ui().text_of(CLEAR.clear_all), Some("told"), "and the page on screen keeps what it was told");
    }

    /// Host with a page of its own beside a program's pages, drawn by
    /// one renderer: the program's picture loader uploads what the
    /// host's page draws, keeps a `release=unused` picture that page
    /// names while a page of the program is freed, and frees it once
    /// the host says its page is gone. In a picture the host's page
    /// shows above the program's.
    ///
    /// Ignored unless asked for: needs a GPU adapter.
    #[test]
    #[ignore = "needs a GPU adapter: run with --include-ignored on a machine that has one"]
    fn a_hosts_own_page_gets_its_pictures_and_shows_above_the_programs() {
        use std::time::{Duration, Instant};
        use turbodot::gogo::{Orientation, ScreenClass};
        use turbodot::pages::{PageId, PagePlatform, PageStack};

        const BACK: [u8; 4] = [0x17, 0x18, 0x1C, 0xFF];
        /// `clear_page.gogo` as a first page: names no picture.
        struct PlainPage;
        impl Page<AppState> for PlainPage {
            const PAGE: &'static BuiltPage = &clear_page::PAGE;
            type Handler = clear_page::Handler;
            fn handle(&mut self, _host: &mut Host<AppState>, _event: PageEvent<clear_page::Handler>) {}
        }
        let mut program: Program<AppState> = Program::new(&crate::ui::BUILT, AppState::default(), PlainPage).expect("the pages can be shown");
        frame(&mut program, 0.0);
        let gpu: turbodot::Gpu = turbodot::Gpu::new().expect("this test needs a GPU adapter");
        let banner = program.assets().images.get("res://assets/banner.jpg").expect("the jpeg");

        // Host's own page: the settings page's file, laid out with the
        // program's text engine and picture table.
        let mut host_pages: PageStack = PageStack::new();
        let settings: PageId = host_pages.add(PageUi::from_static(&settings_page::PAGE));
        let platform: PagePlatform = PagePlatform::new([0.0; 4], Orientation::from_view(SIZE_EPX.0, SIZE_EPX.1), ScreenClass::from_view(SIZE_EPX.0, SIZE_EPX.1), None);
        host_pages.frame(0.0, (SIZE_EPX.0 as u32, SIZE_EPX.1 as u32), 1.0, &platform, program.assets());
        let place: Rect = host_pages.get(settings).rect(SETTINGS.banner);
        let (x, y): (u32, u32) = ((place.x + place.w / 2.0) as u32, (place.y + place.h / 2.0) as u32);

        // In a picture: the host's page above the program's, with its
        // picture.
        let alone: turbodot::Picture = gpu.draw_program(&mut program).expect("the pages are drawn");
        assert_ne!(alone.at(2, 2), BACK, "the program's own page is see-through at its corner");
        let both: turbodot::Picture = gpu.draw_program_with_host_pages(&mut program, &host_pages).expect("the pages are drawn");
        assert_eq!(both.at(2, 2), BACK, "the host's page covers the program's");
        assert_ne!(both.at(x, y), BACK, "the host's page shows its picture");

        // Frame by frame, with a renderer of the host's own.
        let mut renderer: gogo_wgpu::UiRenderer = gogo_wgpu::UiRenderer::new(&gpu.device, turbodot::wgpu::TextureFormat::Rgba8Unorm);
        program.assets_mut().images.forget_uploads();
        program.gpu_draw(&mut renderer, &gpu);
        assert!(!program.is_loading_pictures() && program.assets().images.pending_bytes(banner).is_some(), "the program alone asks for no picture of the host's page");
        let deadline: Instant = Instant::now() + Duration::from_secs(5);
        while program.assets().images.pending_bytes(banner).is_some() {
            program.gpu_draw_beside(&mut renderer, &gpu, &host_pages);
            assert!(Instant::now() < deadline, "the host's picture was never uploaded");
            std::thread::yield_now();
        }

        // A page of the program is freed: the host's live page still
        // names the picture.
        program.go(ClearPage);
        let_the_slides_end(&mut program);
        program.back();
        let_the_slides_end(&mut program);
        program.gpu_draw_beside(&mut renderer, &gpu, &host_pages);
        assert!(program.assets().images.pending_bytes(banner).is_none(), "kept: a live page of the host names it");

        // The host's page is gone, and the host says so.
        host_pages.remove(settings);
        program.gpu_draw_beside(&mut renderer, &gpu, &host_pages);
        assert!(program.assets().images.pending_bytes(banner).is_none(), "nothing is freed until the host says a page is gone");
        program.host_freed_a_page();
        program.gpu_draw_beside(&mut renderer, &gpu, &host_pages);
        assert!(program.assets().images.pending_bytes(banner).is_some(), "freed: no live page names it");
    }

    /// Host reaches the text engine and the picture table the program's
    /// pages use, and adds a picture's key of its own there.
    ///
    /// Needed by a host with pages of its own beside the program's: one
    /// renderer draws both.
    #[test]
    fn a_host_reaches_the_programs_picture_table() {
        let mut program: Program<AppState> = started(AppState::default());
        assert!(program.assets().images.get("res://assets/note.png").is_some(), "a picture a page names");
        let before: usize = program.assets().images.len();
        let key = program.assets_mut().images.intern("host://its-own-picture");
        assert_eq!(program.assets().images.len(), before + 1);
        assert_eq!(program.assets().images.name(key), "host://its-own-picture");
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

    /// Second picture of one program shows its letters, as first does.
    ///
    /// Needed because `Gpu::draw_program` makes a renderer for each
    /// picture, and each renderer must upload the letters by itself: a
    /// second picture came out with every plate and no letters.
    ///
    /// Ignored unless asked for: needs a GPU adapter.
    #[test]
    #[ignore = "needs a GPU adapter: run with --include-ignored on a machine that has one"]
    fn a_second_picture_of_a_program_shows_its_letters() {
        const INK: [u8; 4] = [0xE8, 0xE8, 0xEC, 0xFF];
        let mut program: Program<AppState> = started(AppState::default());
        let gpu: turbodot::Gpu = turbodot::Gpu::new().expect("this test needs a GPU adapter");
        let title: Rect = program.ui().rect(HOME.title);
        let ink_in_title = |picture: &turbodot::Picture| -> usize { (title.y as u32..(title.y + title.h) as u32).flat_map(|y| (title.x as u32..(title.x + title.w) as u32).map(move |x| (x, y))).filter(|&(x, y)| picture.at(x, y) == INK).count() };

        let first: turbodot::Picture = gpu.draw_program(&mut program).expect("the pages are drawn");
        let first_ink: usize = ink_in_title(&first);
        assert!(first_ink > 20, "the precondition: first picture shows the title: {first_ink} ink-colored pixels");
        let second: turbodot::Picture = gpu.draw_program(&mut program).expect("the pages are drawn");
        assert_eq!(ink_in_title(&second), first_ink, "second picture shows the title too");
        assert!(second == first, "and is the first, pixel for pixel");
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
