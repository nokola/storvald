//! The code of widget `SizePicker` (`ui/size_picker.gogo`): a width and a
//! height, each a Stepper inside it.

use turbodot::gogo::{Page, Runtime, Widget, WidgetCx, WidgetEvent};

use crate::stepper::StepperReport;
use crate::ui::{SizePickerInside, SizePickerNames, SizePickerWidgets};

/// What a SizePicker tells the page that places it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum SizePickerReport {
    /// The user stepped the width or the height.
    SizeChanged { width: i32, height: i32 },
}

/// One placed SizePicker.
pub struct SizePicker {
    /// The two Steppers inside it. The build's routing reaches this
    /// field by its name: it is `widgets`, and seen by the crate.
    pub(crate) widgets: SizePickerWidgets,
}

impl SizePicker {
    /// The width and the height as the Steppers show them.
    pub fn size(&self) -> (i32, i32) {
        (self.widgets.width_stepper.value(), self.widgets.height_stepper.value())
    }
}

impl Widget for SizePicker {
    type Report = SizePickerReport;

    fn new(names: &SizePickerNames) -> SizePicker {
        SizePicker { widgets: SizePickerWidgets::new(names) }
    }

    /// A size starts at 1 by 1. The Steppers have loaded first, at 0.
    fn loaded(&mut self, page: &mut Page, rt: &mut Runtime) {
        self.widgets.width_stepper.set_value(page, rt, 1);
        self.widgets.height_stepper.set_value(page, rt, 1);
    }

    fn handle(&mut self, cx: &mut WidgetCx<'_, SizePickerReport>, event: WidgetEvent<'_, SizePickerInside>) {
        match event {
            WidgetEvent::Inside(SizePickerInside::WidthStepper(StepperReport::Changed(_)) | SizePickerInside::HeightStepper(StepperReport::Changed(_))) => {
                let (width, height) = self.size();
                cx.report(SizePickerReport::SizeChanged { width, height });
            }
            // The label "Size" takes no touch.
            WidgetEvent::Own(_) => {}
        }
    }
}
