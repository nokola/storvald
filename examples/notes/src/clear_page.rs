//! Overlay page: asks before every note is dropped. Picture:
//! `ui/clear_page.gogo`.

use turbodot::prelude::*;

use crate::AppState;
use crate::ui::clear_page::{Handler, PAGE};

#[derive(Default)]
pub struct ClearPage;

impl Page<AppState> for ClearPage {
    const PAGE: &'static BuiltPage = &PAGE;
    type Handler = Handler;
    /// Root is see-through: home page stays drawn under this page.
    const KIND: PageKind = PageKind::Overlay;

    fn show(&mut self, _host: &mut Host<AppState>) {}

    fn handle(&mut self, host: &mut Host<AppState>, event: PageEvent<Handler>) {
        match event.handler {
            Handler::ClearAll => {
                host.app.notes.clear();
                host.nav.back();
            }
            Handler::KeepNotes => host.nav.back(),
        }
    }
}
