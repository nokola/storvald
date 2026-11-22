//! What an app does with its page.

use gogo_ui::Event;

use crate::ui::Ui;

/// What an app does with its page.
///
/// Needed so the same app code runs in a window and in a test with no
/// window: `run` calls these two, and a test calls them itself.
pub trait App {
    /// Writes what the app knows into the page: texts, list rows, the
    /// selection.
    ///
    /// Called before the first frame, and again after a reload that
    /// brought a new page in, which starts with none of it.
    fn show(&mut self, ui: &mut Ui);

    /// Says what the page's events mean to the app.
    fn handle(&mut self, ui: &mut Ui, events: &[Event]);
}
