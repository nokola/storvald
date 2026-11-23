//! Layer above every page: a line that shows for a moment. Picture:
//! `ui/toast_page.gogo`.

use turbodot::prelude::*;

use crate::AppState;
use crate::ui::toast_page::{Handler, NAMES, PAGE};

/// How long a line shows.
pub const TOAST_SEC: f32 = 2.0;

#[derive(Default)]
pub struct ToastPage {
    /// Time left for the line that shows. 0: none shows.
    seconds_left: f32,
}

impl ToastPage {
    /// Shows `text` for `TOAST_SEC`. Any page: `host.nav.tell::<ToastPage>(|toast, host| toast.say(host, "Saved"))`.
    pub fn say(&mut self, host: &mut Host<AppState>, text: &str) {
        host.ui.set_text(NAMES.message, text);
        host.ui.set_visible(NAMES.plate, true);
        self.seconds_left = TOAST_SEC;
    }
}

impl Page<AppState> for ToastPage {
    const PAGE: &'static BuiltPage = &PAGE;
    type Handler = Handler;

    fn handle(&mut self, _host: &mut Host<AppState>, _event: PageEvent<Handler>) {}

    /// Counts the line's time down, hides it at 0. Asks for frames only
    /// while a line shows.
    fn frame(&mut self, host: &mut Host<AppState>, dt_sec: f32) -> bool {
        if self.seconds_left <= 0.0 {
            return false;
        }
        self.seconds_left -= dt_sec;
        if self.seconds_left <= 0.0 {
            host.ui.set_visible(NAMES.plate, false);
        }
        true
    }
}
