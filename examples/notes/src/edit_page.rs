//! A new note. Picture: `ui/edit_page.gogo`.

use turbodot::prelude::*;

use crate::AppState;
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

    fn text(&self) -> String {
        format!("Note {}", self.index + 1)
    }
}

impl Page<AppState> for EditPage {
    const PAGE: &'static BuiltPage = &PAGE;
    type Handler = Handler;

    fn show(&mut self, host: &mut Host<AppState>) {
        host.ui.set_text(NAMES.title, &self.text());
    }

    fn handle(&mut self, host: &mut Host<AppState>, event: PageEvent<Handler>) {
        match event.handler {
            Handler::SaveNote => {
                host.app.notes.push(self.text());
                self.is_saved = true;
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
