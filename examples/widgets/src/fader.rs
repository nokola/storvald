//! The code of control `Fader` (`ui/fader.gogo`): a number the user drags
//! with the control's slider behavior. The behavior's drag signals reach
//! this code first, and it passes them on to the page.

use std::fmt::Write as _;

use turbodot::gogo::page::Action;
use turbodot::gogo::{EventKind, NothingInside, Page, Runtime, Value, Widget, WidgetCx, WidgetEvent};

use crate::ui::FaderNames;

/// What a Fader tells the page that places it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum FaderSignal {
    /// The user moved the number to this value.
    ValueChanged(f32),
    /// A finger took the Fader (the behavior's `drag_started`).
    DragStarted,
    /// The finger let go, or the drag was called off (`drag_ended`).
    DragEnded,
}

/// One placed Fader: its number, and the text the number shows as.
pub struct Fader {
    names: FaderNames,
    value: f32,
    /// The number as text, reused so a drag allocates nothing.
    text: String,
}

impl Fader {
    pub fn value(&self) -> f32 {
        self.value
    }

    /// Sets the number from the page's code. Raises no signal: the page
    /// knows what it set.
    pub fn set_value(&mut self, page: &mut Page, rt: &mut Runtime, value: f32) {
        self.value = value;
        rt.set_value(page, self.names.root, value);
        self.show(page, rt);
    }

    /// Writes the number into the control's label.
    fn show(&mut self, page: &mut Page, rt: &mut Runtime) {
        self.text.clear();
        let _ = write!(self.text, "{:.0}", self.value);
        rt.set_text(page, self.names.value, &self.text);
    }
}

impl Widget for Fader {
    type Signal = FaderSignal;

    fn new(names: &FaderNames) -> Fader {
        Fader { names: *names, value: 0.0, text: String::with_capacity(12) }
    }

    fn loaded(&mut self, page: &mut Page, rt: &mut Runtime) {
        self.show(page, rt);
    }

    /// A drag sample or its end moves the number; the behavior's drag
    /// signals go on to the page as they are.
    fn handle(&mut self, cx: &mut WidgetCx<'_, FaderSignal>, event: WidgetEvent<'_, NothingInside>) {
        let event = match event {
            WidgetEvent::Own(event) => event,
            WidgetEvent::Inside(nothing) => match nothing {},
        };
        match event.kind {
            // A drag's samples come as `Edit`, its end as `Change`: the
            // end repeats the last sample's value, and says nothing new.
            EventKind::Edit(Value::VNum(value)) | EventKind::Change(Value::VNum(value)) => {
                if value == self.value {
                    return;
                }
                self.value = value;
                self.show(cx.page, cx.rt);
                cx.emit(FaderSignal::ValueChanged(value));
            }
            EventKind::Signal => {
                let Some(Action::Emit(name)) = event.action else { return };
                match cx.page.str(name) {
                    "drag_started" => cx.emit(FaderSignal::DragStarted),
                    "drag_ended" => cx.emit(FaderSignal::DragEnded),
                    _ => {}
                }
            }
            _ => {}
        }
    }
}
