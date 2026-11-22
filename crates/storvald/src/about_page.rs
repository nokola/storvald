//! Page that says what this version of Storvald is. Picture: `ui/about_page.gogo`.

use turbodot::prelude::*;

use crate::AppState;
use crate::ui::about_page::{Handler, NAMES, PAGE};

/// About page. Holds nothing of its own.
#[derive(Default)]
pub struct AboutPage;

impl Page<AppState> for AboutPage {
    const PAGE: &'static BuiltPage = &PAGE;
    type Handler = Handler;

    fn show(&mut self, host: &mut Host<AppState>) {
        host.ui.set_text(NAMES.about_version, &host.app.carried_line());
    }

    fn handle(&mut self, host: &mut Host<AppState>, event: PageEvent<Handler>) {
        match event.handler {
            Handler::GoBack => host.nav.back(),
        }
    }
}
