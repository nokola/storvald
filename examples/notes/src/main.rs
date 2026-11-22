//! Small notes program on TurboDot, in the shape every client has: one
//! line builds pages (`build.rs`), one declares them, one starts the
//! program, plus a struct and a `.gogo` file per page.
//!
//! - `home_page`: the notes. Opens the other two pages.
//! - `edit_page`: a new note. Asks once before back drops it.
//! - `settings_page`: cached page. Later open shows same view.
//!
//! Run it with `cargo run -p turbodot-example-notes`. Esc is back.

mod edit_page;
mod home_page;
mod settings_page;

turbodot::built_pages!();

use home_page::HomePage;

/// Program-wide state. Every page reaches it as `host.app`.
#[derive(Default)]
pub struct AppState {
    pub notes: Vec<String>,
    /// True when device has too little memory to cache pages.
    pub is_memory_short: bool,
}

turbodot::start!(AppState, HomePage);

#[cfg(test)]
mod tests {
    use turbodot::Program;
    use turbodot::gogo::{Phase, Rect};

    use super::*;
    use crate::edit_page::EditPage;
    use crate::settings_page::SettingsPage;
    use crate::ui::{edit_page, home_page, settings_page};

    const HOME: home_page::Names = home_page::NAMES;
    const EDIT: edit_page::Names = edit_page::NAMES;
    const SETTINGS: settings_page::Names = settings_page::NAMES;

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
        let mut program: Program<AppState> = started(AppState { notes: Vec::new(), is_memory_short: true });
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
        let picture: turbodot::Picture = gpu.draw_program(&program).expect("the pages are drawn");
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
    }
}
