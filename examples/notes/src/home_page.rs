//! First page: the notes. Picture: `ui/home_page.gogo`.

use turbodot::prelude::*;

use crate::AppState;
use crate::clear_page::ClearPage;
use crate::edit_page::EditPage;
use crate::footer_page::FooterPage;
use crate::paper_page::PaperPage;
use crate::settings_page::SettingsPage;
use crate::toast_page::ToastPage;
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
        // Page inside this page: lives as long as this page does.
        host.nav.place(NAMES.footer, FooterPage);
        // Layer above every page: any page tells it what to say.
        host.nav.add_layer(ToastPage::default());
    }

    /// Each time page comes into view: first, and on return by back.
    fn show(&mut self, host: &mut Host<AppState>) {
        let rows: Vec<Item> = host.app.notes.iter().enumerate().map(|(i, note)| Item::Record(vec![("key".to_string(), ItemValue::Str(i.to_string())), ("text".to_string(), ItemValue::Str(note.clone()))])).collect();
        host.ui.set_items(NAMES.notes, &rows).expect("every note has a key of its own");
    }

    /// Page is in place, no page slides: at program start, and when
    /// page above it finished sliding out.
    fn arrived(&mut self, host: &mut Host<AppState>) {
        host.app.page_log.push("home arrived");
    }

    /// A back request closed the "More" menu. The page stays.
    fn back_closed_popup(&mut self, host: &mut Host<AppState>) {
        host.app.page_log.push("home menu closed by back");
    }

    /// One call per user action.
    fn handle(&mut self, host: &mut Host<AppState>, event: PageEvent<Handler>) {
        match event.handler {
            Handler::AddNote => host.nav.go(EditPage::new(host.app.notes.len())),
            Handler::OpenSettings => host.nav.go(SettingsPage::new(host.app.notes.len())),
            Handler::ClearNotes => host.nav.go(ClearPage),
            Handler::OpenPaper => host.nav.go(PaperPage::default()),
            // The file shows the menu while this Check is checked.
            Handler::More => {}
            // The menu's one line: says what this is, and closes the
            // menu by unchecking the Check that shows it.
            Handler::AboutNotes => {
                host.ui.set_checked(NAMES.more, false);
                host.nav.tell::<ToastPage>(|toast, host| toast.say(host, "Notes, a TurboDot example"));
            }
        }
    }
}
