//! Part placed inside other pages: count of notes, and a button that
//! adds one. Picture: `ui/footer_page.gogo`.

use turbodot::prelude::*;

use crate::AppState;
use crate::home_page::HomePage;
use crate::ui::footer_page::{Handler, NAMES, PAGE};

pub struct FooterPage;

impl Page<AppState> for FooterPage {
    const PAGE: &'static BuiltPage = &PAGE;
    type Handler = Handler;

    /// Called with the page it sits in: each time that page comes into
    /// view.
    fn show(&mut self, host: &mut Host<AppState>) {
        host.ui.set_text(NAMES.note_count, &format!("{} notes", host.app.notes.len()));
    }

    /// Called with the page it sits in.
    fn arrived(&mut self, host: &mut Host<AppState>) {
        host.app.page_log.push("footer arrived");
    }

    fn handle(&mut self, host: &mut Host<AppState>, event: PageEvent<Handler>) {
        match event.handler {
            Handler::AddSample => {
                host.app.notes.push(format!("Sample {}", host.app.notes.len() + 1));
                self.show(host);
                // Home page shows the notes: it writes its list again.
                host.nav.tell::<HomePage>(|home, host| home.show(host));
            }
        }
    }
}
