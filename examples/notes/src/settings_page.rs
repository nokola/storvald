//! Cached page. Picture: `ui/settings_page.gogo`.

use turbodot::prelude::*;

use crate::AppState;
use crate::ui::settings_page::{Handler, NAMES, PAGE};

/// Count of lines in page's long text.
const BODY_LINE_COUNT: usize = 80;

pub struct SettingsPage {
    /// Count of notes at page's last open.
    note_count: usize,
}

impl SettingsPage {
    /// Page opened while there are `note_count` notes.
    pub fn new(note_count: usize) -> SettingsPage {
        SettingsPage { note_count }
    }

    /// Count of notes at page's last open.
    #[cfg(test)]
    pub fn note_count(&self) -> usize {
        self.note_count
    }
}

impl CachedPage<AppState> for SettingsPage {
    const PAGE: &'static BuiltPage = &PAGE;
    type Handler = Handler;

    /// Long text, written once: later open finds it scrolled to same place.
    fn loaded(&mut self, host: &mut Host<AppState>) {
        let body: String = (1..=BODY_LINE_COUNT).map(|line| format!("Line {line} of a long text.\n")).collect();
        host.ui.set_text(NAMES.body_text, &body);
    }

    fn show(&mut self, host: &mut Host<AppState>) {
        host.ui.set_text(NAMES.note_count, &format!("{} notes when this page was opened", self.note_count));
    }

    fn handle(&mut self, host: &mut Host<AppState>, event: PageEvent<Handler>) {
        match event.handler {
            Handler::GoBack => host.nav.back(),
        }
    }

    /// Cached page opens again: takes over opener's value.
    fn reopened(&mut self, fresh: Self) {
        self.note_count = fresh.note_count;
    }
}
