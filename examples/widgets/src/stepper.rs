//! The code of widget `Stepper` (`ui/stepper.gogo`): a whole number the
//! user steps with a minus and a plus button.

use std::fmt::Write as _;

use turbodot::gogo::{EventKind, NothingInside, Page, Runtime, Widget, WidgetCx, WidgetEvent};

use crate::ui::StepperNames;

/// What a Stepper tells the page that places it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum StepperSignal {
    /// The user stepped the number to this value.
    Changed(i32),
}

/// One placed Stepper: its number, and the text the number shows as.
pub struct Stepper {
    names: StepperNames,
    value: i32,
    /// The number as text, reused so a step allocates nothing.
    text: String,
}

impl Stepper {
    pub fn value(&self) -> i32 {
        self.value
    }

    /// Sets the number from the page's code. Raises no signal: the page
    /// knows what it set.
    pub fn set_value(&mut self, page: &mut Page, rt: &mut Runtime, value: i32) {
        self.value = value;
        self.show(page, rt);
    }

    /// Writes the number into the widget's label.
    fn show(&mut self, page: &mut Page, rt: &mut Runtime) {
        self.text.clear();
        let _ = write!(self.text, "{}", self.value);
        rt.set_text(page, self.names.value, &self.text);
    }
}

impl Widget for Stepper {
    type Signal = StepperSignal;

    fn new(names: &StepperNames) -> Stepper {
        Stepper { names: *names, value: 0, text: String::with_capacity(12) }
    }

    /// A Stepper starts at 0 when its page is made.
    fn loaded(&mut self, page: &mut Page, rt: &mut Runtime) {
        self.value = 0;
        self.show(page, rt);
    }

    fn handle(&mut self, cx: &mut WidgetCx<'_, StepperSignal>, event: WidgetEvent<'_, NothingInside>) {
        let event = match event {
            WidgetEvent::Own(event) => event,
            WidgetEvent::Inside(nothing) => match nothing {},
        };
        if !matches!(event.kind, EventKind::Activate) {
            return;
        }
        let step: i32 = if event.node == self.names.plus {
            1
        } else if event.node == self.names.minus {
            -1
        } else {
            return;
        };
        self.value += step;
        self.show(cx.page, cx.rt);
        cx.emit(StepperSignal::Changed(self.value));
    }
}
