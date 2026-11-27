//! Compiles every page in `ui/`, and names the struct of each widget that
//! has code: the build then writes each page's `Widgets` and the routing.

fn main() {
    turbodot::build::pages_with_widget_code("ui", &[("Stepper", "crate::stepper::Stepper"), ("SizePicker", "crate::size_picker::SizePicker")]);
}
