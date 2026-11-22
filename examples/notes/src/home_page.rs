//! First page: the notes. Picture: `ui/home_page.gogo`.

use turbodot::prelude::*;

use crate::AppState;
use crate::edit_page::EditPage;
use crate::settings_page::SettingsPage;
use crate::ui::home_page::{Handler, NAMES, PAGE};

#[derive(Default)]
pub struct HomePage;

impl Page<AppState> for HomePage {
    const PAGE: &'static BuiltPage = &PAGE;
    type Handler = Handler;

    /// Once, at program start: turns page cache on or off for this
    /// device, makes settings page ahead of first open.
    fn loaded(&mut self, host: &mut Host<AppState>) {
        host.nav.set_page_cache_enabled(!host.app.is_memory_short);
        host.nav.preload_if_cache_enabled(SettingsPage::new(host.app.notes.len()));
    }

    /// Each time page comes into view: first, and on return by back.
    fn show(&mut self, host: &mut Host<AppState>) {
        let rows: Vec<Item> = host.app.notes.iter().enumerate().map(|(i, note)| Item::Record(vec![("key".to_string(), ItemValue::Str(i.to_string())), ("text".to_string(), ItemValue::Str(note.clone()))])).collect();
        host.ui.set_items(NAMES.notes, &rows).expect("every note has a key of its own");
    }

    /// One call per user action.
    fn handle(&mut self, host: &mut Host<AppState>, event: PageEvent<Handler>) {
        match event.handler {
            Handler::AddNote => host.nav.go(EditPage::new(host.app.notes.len())),
            Handler::OpenSettings => host.nav.go(SettingsPage::new(host.app.notes.len())),
        }
    }
}
