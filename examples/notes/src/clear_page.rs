//! Overlay page: asks before every note is dropped. Picture:
//! `ui/clear_page.gogo`.

use turbodot::prelude::*;

use crate::AppState;
use crate::dropped_page::DroppedPage;
use crate::home_page::HomePage;
use crate::ui::clear_page::{Handler, PAGE};

#[derive(Default)]
pub struct ClearPage;

impl Page<AppState> for ClearPage {
    const PAGE: &'static BuiltPage = &PAGE;
    type Handler = Handler;
    /// Root is see-through: home page stays drawn under this page.
    const KIND: PageKind = PageKind::Overlay;

    fn handle(&mut self, host: &mut Host<AppState>, event: PageEvent<Handler>) {
        match event.handler {
            Handler::ClearAll => {
                host.app.notes.clear();
                // Home page shows under this page: it writes its list
                // again now, not when it comes back on top.
                host.nav.tell::<HomePage>(|home, host| home.show(host));
                // Next dialog takes this one's place: back from it
                // returns to home page.
                host.nav.replace(DroppedPage);
            }
            Handler::KeepNotes => host.nav.back(),
        }
    }

    fn left(&mut self, host: &mut Host<AppState>) {
        host.app.page_log.push("clear left");
    }
}
