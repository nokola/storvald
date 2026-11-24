//! Code of the three pages. Pictures: `ui/first_page.gogo`,
//! `ui/controls_page.gogo`, `ui/list_page.gogo`.

use turbodot::prelude::*;

use crate::AppState;
use crate::ui::{controls_page, first_page, list_page};

/// Count of rows the list page writes each time it comes into view.
pub const LIST_ROW_COUNT: usize = 100;

/// First page: the page the other two open over.
pub struct FirstPage;

impl Page<AppState> for FirstPage {
    const PAGE: &'static BuiltPage = &first_page::PAGE;
    type Handler = first_page::Handler;

    fn handle(&mut self, host: &mut Host<AppState>, event: PageEvent<first_page::Handler>) {
        match event.handler {
            first_page::Handler::OpenControls => host.nav.go(ControlsPage),
            first_page::Handler::OpenList => host.nav.go(ListPage),
        }
    }
}

/// Page of many controls. Made new at every open.
pub struct ControlsPage;

impl Page<AppState> for ControlsPage {
    const PAGE: &'static BuiltPage = &controls_page::PAGE;
    type Handler = controls_page::Handler;

    fn handle(&mut self, host: &mut Host<AppState>, event: PageEvent<controls_page::Handler>) {
        match event.handler {
            controls_page::Handler::GoBack => host.nav.back(),
            controls_page::Handler::FirstSlider => host.app.slider_change_count += 1,
        }
    }
}

/// Page of one long list. Made new at every open.
pub struct ListPage;

impl Page<AppState> for ListPage {
    const PAGE: &'static BuiltPage = &list_page::PAGE;
    type Handler = list_page::Handler;

    /// Each time page comes into view: writes the list's rows.
    fn show(&mut self, host: &mut Host<AppState>) {
        let rows: Vec<Item> = (0..LIST_ROW_COUNT)
            .map(|row| Item::Record(vec![("key".to_string(), ItemValue::Str(row.to_string())), ("name".to_string(), ItemValue::Str(format!("Row {row}"))), ("hint".to_string(), ItemValue::Str(format!("hint of row {row}")))]))
            .collect();
        host.ui.set_items(list_page::NAMES.rows, &rows).expect("every row has a key of its own");
    }

    fn handle(&mut self, host: &mut Host<AppState>, event: PageEvent<list_page::Handler>) {
        match event.handler {
            list_page::Handler::GoBack => host.nav.back(),
        }
    }
}
