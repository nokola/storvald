//! A new note. Picture: `ui/edit_page.gogo`.

use turbodot::prelude::*;

use crate::AppState;
use crate::footer_page::FooterPage;
use crate::toast_page::ToastPage;
use crate::ui::edit_page::{Handler, NAMES, PAGE};

pub struct EditPage {
    /// Note's place in `AppState::notes` once saved.
    index: usize,
    is_saved: bool,
    /// True when a back request was answered with "not saved" already.
    did_warn: bool,
}

impl EditPage {
    /// Page for the note that will be number `index` + 1.
    pub fn new(index: usize) -> EditPage {
        EditPage { index, is_saved: false, did_warn: false }
    }

    /// Note's title, and its text when user typed none.
    fn title(&self) -> String {
        format!("Note {}", self.index + 1)
    }
}

impl Page<AppState> for EditPage {
    const PAGE: &'static BuiltPage = &PAGE;
    type Handler = Handler;

    /// Same footer page the home page has, one of its own: freed with
    /// this page.
    fn loaded(&mut self, host: &mut Host<AppState>) {
        host.nav.place(NAMES.footer, FooterPage);
    }

    fn show(&mut self, host: &mut Host<AppState>) {
        host.ui.set_text(NAMES.title, &self.title());
    }

    fn arrived(&mut self, host: &mut Host<AppState>) {
        host.app.page_log.push("edit arrived");
    }

    /// Slide out ended: last call before page is freed.
    fn left(&mut self, host: &mut Host<AppState>) {
        host.app.page_log.push("edit left");
    }

    fn handle(&mut self, host: &mut Host<AppState>, event: PageEvent<Handler>) {
        match event.handler {
            // Each change of the field: typing, a paste, a cut, the
            // on-screen keyboard.
            Handler::NoteEdited | Handler::NoteText => {
                let typed: usize = host.ui.text_of(NAMES.note_text).unwrap_or("").chars().count();
                host.ui.set_text(NAMES.typed_count, &format!("{typed} characters"));
            }
            Handler::SaveNote => {
                let typed: String = host.ui.text_of(NAMES.note_text).unwrap_or("").to_string();
                host.app.notes.push(if typed.is_empty() { self.title() } else { typed });
                self.is_saved = true;
                host.nav.tell::<ToastPage>(|toast, host| toast.say(host, "Saved"));
                host.nav.back();
            }
        }
    }

    /// First back request on unsaved note shows warning and stays. Second one leaves.
    fn back(&mut self, host: &mut Host<AppState>) -> Back {
        if self.is_saved || self.did_warn {
            return Back::Leave;
        }
        self.did_warn = true;
        host.ui.set_visible(NAMES.unsaved, true);
        Back::Stay
    }
}
