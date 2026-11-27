//! The page that places a Stepper, a SizePicker, a LabeledStepper and a
//! Fader (`ui/counter_page.gogo`).

use turbodot::prelude::*;

use crate::ui::counter_page::{self, Handler};

/// How many lines the page makes.
pub const LINES: usize = 3;

/// The page, with its widgets' code and what it heard.
pub struct CounterPage {
    /// The page's instances of widgets with code, made by the build.
    pub widgets: counter_page::Widgets,
    /// Every handler the page was called with, in order. A test reads it.
    pub heard: Vec<Handler>,
}

impl CounterPage {
    pub fn new() -> CounterPage {
        CounterPage { widgets: counter_page::Widgets::new(), heard: Vec::with_capacity(64) }
    }
}

impl Page for CounterPage {
    const PAGE: &'static BuiltPage = &counter_page::PAGE;
    type Handler = Handler;

    fn widgets(&mut self) -> Option<&mut dyn WidgetSet<Handler>> {
        Some(&mut self.widgets)
    }

    /// Three lines, each with a Stepper. Their structs are made with the
    /// rows; `sync_rows` makes them now, so this call can write into them.
    fn loaded(&mut self, host: &mut Host) {
        let ui: &mut PageUi = &mut *host.ui;
        ui.runtime.set_count(&mut ui.page, counter_page::NAMES.lines, LINES);
        self.widgets.sync_rows(&mut ui.page, &mut ui.runtime);
        for (line, row) in self.widgets.lines.iter_mut().enumerate() {
            row.amount.set_value(&mut ui.page, &mut ui.runtime, line as i32 * 10);
        }
    }

    fn handle(&mut self, host: &mut Host, event: PageEvent<Handler>) {
        match &event.handler {
            Handler::Reset => self.widgets.count.set_value(&mut host.ui.page, &mut host.ui.runtime, 0),
            Handler::Count(_) | Handler::Size(_) | Handler::ZoomStepper(_) | Handler::Volume(_) | Handler::Lines(..) => {}
        }
        self.heard.push(event.handler);
    }
}
