//! The code of widget `Note` (`ui/note.gogo`): a line of text the page's
//! code writes. Its file has no `signal` line, so its signal enum is
//! `NoSignal` and the page gets no handler for it.

use turbodot::gogo::{NoSignal, NothingInside, Page, Runtime, Widget, WidgetCx, WidgetEvent};

use crate::ui::NoteNames;

/// One placed Note: the text it shows.
pub struct Note {
    names: NoteNames,
    text: String,
}

impl Note {
    pub fn text(&self) -> &str {
        &self.text
    }

    /// Shows `text` from the page's code.
    pub fn set_text(&mut self, page: &mut Page, rt: &mut Runtime, text: &str) {
        self.text.clear();
        self.text.push_str(text);
        rt.set_text(page, self.names.root, text);
    }
}

impl Widget for Note {
    type Signal = NoSignal;

    fn new(names: &NoteNames) -> Note {
        Note { names: *names, text: String::from("Ready") }
    }

    /// Shows the text it keeps: "Ready" until the page writes one.
    fn loaded(&mut self, page: &mut Page, rt: &mut Runtime) {
        rt.set_text(page, self.names.root, &self.text);
    }

    /// Nothing to do: a Note's label takes no taps.
    fn handle(&mut self, _cx: &mut WidgetCx<'_, NoSignal>, _event: WidgetEvent<'_, NothingInside>) {}
}
