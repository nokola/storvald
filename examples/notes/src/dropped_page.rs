//! Overlay page: says every note is dropped. Opens in place of the
//! clear page (`Nav::replace`). Picture: `ui/dropped_page.gogo`.

use turbodot::prelude::*;

use crate::AppState;
use crate::ui::dropped_page::{Handler, PAGE};

pub struct DroppedPage;

impl Page<AppState> for DroppedPage {
    const PAGE: &'static BuiltPage = &PAGE;
    type Handler = Handler;
    /// Root is see-through: home page stays drawn under this page.
    const KIND: PageKind = PageKind::Overlay;

    fn handle(&mut self, host: &mut Host<AppState>, event: PageEvent<Handler>) {
        match event.handler {
            Handler::GotIt => host.nav.back(),
        }
    }
}
