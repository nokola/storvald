//! What the build writes for widgets with code (`BuildOptions::widget_code`,
//! `gogo_ui::widget`): a widget's `signal` names, the page's `Handler`
//! variants and `Widgets`, each widget's part of the trait, and the
//! mistakes it refuses. The written code itself is compiled and run by
//! the example `examples/widgets`.

use gogo_ui::emit::{WidgetCode, emit_rust_with, emit_widget_code, handler_names_with};
use gogo_ui::{compile, has_errors};

/// A page with a Stepper (code, signals), a Picker (code, two Steppers
/// inside), a Row widget with no code holding a Stepper, and a button.
const PAGE: &str = "widget Stepper {
    signal changed
    Row {
        Button minus text=\"-\"
        Label value text=\"0\"
        Button plus text=\"+\"
    }
}
widget Picker {
    signal size_changed
    Column {
        Stepper across
        Stepper down
    }
}
widget Labeled $label=\"\" {
    Row {
        Label text=$label
        Stepper stepper
    }
}
View page {
    Column {
        Stepper count
        Picker size
        Labeled zoom label=\"Zoom\"
        Button reset text=\"Reset\"
    }
}
";

/// The two widgets with code of `PAGE`, as a build script names them.
const STRUCTS: &[(&str, &str)] = &[("Stepper", "crate::Stepper"), ("Picker", "crate::Picker")];

/// Widgets with code whose events are raised in every way a page could
/// hear one: a root that is itself a button (`Go`), a button with an
/// `on:` line inside (`Bump`), and a control with a slider behavior, its
/// drag signals and a key claim inside (`Dial`).
const RAISERS: &str = "widget Go {
    signal pressed
    Button face text=\"Go\"
}
widget Bump {
    signal bumped
    Row {
        Button up text=\"+\", on:activate=stepUp
    }
}
control Knob $value=0, $min=0, $max=10 {
    behavior slider by $value in $min..$max
    signal drag_started, drag_ended
    keys enter
    Layers width=100, height=20 {
        slot track width=fill, height=10 {
            Rectangle width=fill, height=fill
        }
    }
}
widget Dial {
    signal turned
    Row {
        Knob knob
    }
}
View page {
    Column {
        Go go
        Bump bump
        Dial dial
        Button reset text=\"Reset\"
    }
}
";

/// Every way an event inside an instance with code could reach the page
/// by a name of its own is closed: the page hears the instance's signal
/// alone.
#[test]
fn no_name_raised_inside_an_instance_with_code_is_the_page_s() {
    let c = compile(RAISERS, &|_| None);
    assert!(!has_errors(&c.diags), "{:#?}", c.diags);
    let plain: Vec<String> = handler_names_with(&c.page, &WidgetCode::default());
    for name in ["go", "stepUp", "drag_started", "drag_ended", "dial", "reset"] {
        assert!(plain.iter().any(|n| n == name), "with no code, `{name}` is the page's: {plain:?}");
    }
    let structs: &[(&str, &str)] = &[("Go", "crate::Go"), ("Bump", "crate::Bump"), ("Dial", "crate::Dial")];
    let code: WidgetCode<'_> = WidgetCode { structs, signals: &c.widget_signals };
    assert_eq!(handler_names_with(&c.page, &code), ["reset"]);
    let rust: String = emit_rust_with(&c.page, "m", &code).unwrap();
    for variant in ["Go(<crate::Go", "Bump(<crate::Bump", "Dial(<crate::Dial"] {
        assert!(rust.contains(variant), "{variant}: {rust}");
    }
}

#[test]
fn a_widget_lists_its_signals_and_the_compile_hands_them_out() {
    let c = compile(PAGE, &|_| None);
    assert!(!has_errors(&c.diags), "{:#?}", c.diags);
    let signals_of = |widget: &str| c.widget_signals.iter().find(|(w, _)| w == widget).map(|(_, r)| r.clone());
    assert_eq!(signals_of("Stepper"), Some(vec!["changed".to_string()]));
    assert_eq!(signals_of("Picker"), Some(vec!["size_changed".to_string()]));
    assert_eq!(signals_of("Labeled"), Some(Vec::new()), "a widget with no `signal` line raises none");
}

#[test]
fn a_signal_named_twice_is_refused() {
    let c = compile("widget Stepper {\n    signal changed, changed\n    Panel box width=10, height=10\n}\nView page { Stepper s }\n", &|_| None);
    assert!(c.diags.iter().any(|d| d.message.contains("`signal changed` is listed twice")), "{:#?}", c.diags);
}

#[test]
fn an_empty_signal_line_is_refused() {
    let c = compile("widget Stepper {\n    signal\n    Panel box width=10, height=10\n}\nView page { Stepper s }\n", &|_| None);
    assert!(has_errors(&c.diags), "`signal` with no name: {:#?}", c.diags);
}

/// One list, two raisers: a control's `signal` line is the list its code
/// would raise, and with no code its behavior's names are heard by their
/// own names; a widget's are its code's, heard only through the instance.
///
/// Needed so a widget's signal never turns into a handler of the page by
/// itself, and a control with no code keeps its drag signals where pages
/// hear them now.
#[test]
fn a_control_with_no_code_is_heard_by_its_behavior_s_names_and_a_widget_through_its_code() {
    let c = compile(RAISERS, &|_| None);
    assert!(!has_errors(&c.diags), "{:#?}", c.diags);
    let signals_of = |widget: &str| c.widget_signals.iter().find(|(w, _)| w == widget).map(|(_, s)| s.clone());
    assert_eq!(signals_of("Knob"), Some(vec!["drag_started".to_string(), "drag_ended".to_string()]), "the list its code would raise");
    assert_eq!(signals_of("Dial"), Some(vec!["turned".to_string()]));
    let plain: Vec<String> = handler_names_with(&c.page, &WidgetCode::default());
    assert!(plain.iter().any(|n| n == "drag_started"), "a control's signal is heard by its own name: {plain:?}");
    for name in ["pressed", "bumped", "turned"] {
        assert!(!plain.iter().any(|n| n == name), "a widget's signal `{name}` is never a handler of its own: {plain:?}");
    }
}

/// Two instances of a control with a slider behavior, and a button.
const FADERS: &str = "control Fader $value=0, $min=0, $max=10 {
    behavior slider by $value in $min..$max
    signal value_changed, drag_started, drag_ended
    Layers width=100, height=20 {
        slot track width=fill, height=10 {
            Rectangle width=fill, height=fill
        }
    }
}
View page {
    Column {
        Fader volume
        Fader balance
        Button reset text=\"Reset\"
    }
}
";

/// A control with code: its `signal` line lists everything the page hears
/// from it, its behavior's drag signals among them. The behavior still
/// raises those (for the code), and the page hears each instance by its
/// name alone: no bare `drag_started` that could not say which one.
///
/// Needed so a page with two sliders knows which one the finger is on.
#[test]
fn a_control_with_code_is_heard_through_its_code_alone() {
    let c = compile(FADERS, &|_| None);
    assert!(!has_errors(&c.diags), "{:#?}", c.diags);
    let signals_of = |widget: &str| c.widget_signals.iter().find(|(w, _)| w == widget).map(|(_, s)| s.clone());
    assert_eq!(signals_of("Fader"), Some(vec!["value_changed".to_string(), "drag_started".to_string(), "drag_ended".to_string()]));
    assert_eq!(c.page.behaviors.len(), 2);
    assert!(c.page.behaviors.iter().all(|b| b.drag_started != gogo_ui::NO_STR && b.drag_ended != gogo_ui::NO_STR), "the behavior still raises the drag signals the line lists");
    let plain: Vec<String> = handler_names_with(&c.page, &WidgetCode::default());
    for name in ["volume", "balance", "drag_started", "drag_ended", "reset"] {
        assert!(plain.iter().any(|n| n == name), "with no code, `{name}` is the page's: {plain:?}");
    }
    let code: WidgetCode<'_> = WidgetCode { structs: &[("Fader", "crate::Fader")], signals: &c.widget_signals };
    assert_eq!(handler_names_with(&c.page, &code), ["reset"]);
    let rust: String = emit_rust_with(&c.page, "m", &code).unwrap();
    assert!(rust.contains("    pub enum Handler {\n        Reset,\n        Volume(<crate::Fader as gogo_ui::Widget>::Signal),\n        Balance(<crate::Fader as gogo_ui::Widget>::Signal),\n    }"), "{rust}");
    let shape: String = emit_widget_code(&[("m", &c.page)], &code).unwrap();
    assert!(shape.contains("const _: fn(&FaderSignalOfFile) = |signal| match *signal {\n    FaderSignalOfFile::ValueChanged { .. } => {}\n    FaderSignalOfFile::DragStarted { .. } => {}\n    FaderSignalOfFile::DragEnded { .. } => {}\n};"), "the code's enum is checked against the whole line: {shape}");
}

#[test]
fn a_control_s_signal_named_twice_is_refused() {
    let c = compile("control Knob $value=0, $min=0, $max=10 {\n    behavior slider by $value in $min..$max\n    signal drag_started, drag_started\n    Panel box width=10, height=10\n}\nView page { Knob k }\n", &|_| None);
    assert!(c.diags.iter().any(|d| d.message.contains("`signal drag_started` is listed twice")), "{:#?}", c.diags);
}

/// `report` is no word of the language: a widget's signals are `signal`
/// lines, and a file that says `report` does not compile.
#[test]
fn a_report_line_is_refused() {
    let c = compile("widget Stepper {\n    report changed\n    Panel box width=10, height=10\n}\nView page { Stepper s }\n", &|_| None);
    assert!(has_errors(&c.diags), "{:#?}", c.diags);
}

#[test]
fn the_page_hears_an_instance_with_code_by_its_signal_and_nothing_inside_it() {
    let c = compile(PAGE, &|_| None);
    assert!(!has_errors(&c.diags), "{:#?}", c.diags);
    // With no code, every instance hears its own buttons by its name.
    let plain: Vec<String> = handler_names_with(&c.page, &WidgetCode::default());
    for name in ["count", "size", "zoom", "reset"] {
        assert!(plain.iter().any(|n| n == name), "{name} in {plain:?}");
    }
    // With code, the buttons inside an instance are its code's.
    let code: WidgetCode<'_> = WidgetCode { structs: STRUCTS, signals: &c.widget_signals };
    let names: Vec<String> = handler_names_with(&c.page, &code);
    assert_eq!(names, ["reset"], "only the page's own button is a name of the page");

    let rust: String = emit_rust_with(&c.page, "m", &code).unwrap();
    assert!(rust.contains("    #[derive(Clone, Debug, PartialEq)]\n    pub enum Handler {\n        Reset,\n        Count(<crate::Stepper as gogo_ui::Widget>::Signal),\n        Size(<crate::Picker as gogo_ui::Widget>::Signal),\n        ZoomStepper(<crate::Stepper as gogo_ui::Widget>::Signal),\n    }"), "{rust}");
    assert!(rust.contains("        const HAS_WIDGET_CODE: bool = true;"), "{rust}");
    assert!(rust.contains("    pub struct Widgets {\n        pub count: crate::Stepper,\n        pub size: crate::Picker,\n        pub zoom_stepper: crate::Stepper,\n    }"), "{rust}");
    assert!(rust.contains("zoom_stepper: <crate::Stepper as gogo_ui::Widget>::new(&NAMES.zoom.stepper),"), "the instance inside a widget with no code is reached through it: {rust}");
    assert!(rust.contains("heard(Handler::ZoomStepper(signal))"), "{rust}");
    // No repeat: nothing in the routing would be unused, or the app's
    // build warns about the generated code.
    assert!(rust.contains("        pub fn sync_rows(&mut self, _page: &mut gogo_ui::Page, _rt: &mut gogo_ui::Runtime) {\n        }"), "{rust}");
    assert!(!rust.contains("let mut row"), "no row to pick: {rust}");

    // A page with no widget with code keeps its old `Handler`.
    let plain_rust: String = emit_rust_with(&c.page, "m", &WidgetCode::default()).unwrap();
    assert!(plain_rust.contains("    #[derive(Clone, Copy, Debug, PartialEq, Eq)]\n    pub enum Handler {"), "{plain_rust}");
    assert!(!plain_rust.contains("pub struct Widgets") && !plain_rust.contains("HAS_WIDGET_CODE"), "{plain_rust}");
}

#[test]
fn an_instance_with_code_that_raises_no_signal_has_no_variant() {
    let c = compile("widget Badge {\n    Label text=\"!\"\n}\nView page {\n    Badge badge\n}\n", &|_| None);
    assert!(!has_errors(&c.diags), "{:#?}", c.diags);
    let code: WidgetCode<'_> = WidgetCode { structs: &[("Badge", "crate::Badge")], signals: &c.widget_signals };
    let rust: String = emit_rust_with(&c.page, "m", &code).unwrap();
    assert!(rust.contains("    #[derive(Clone, Copy, Debug, PartialEq, Eq)]\n    pub enum Handler {\n    }"), "{rust}");
    assert!(rust.contains("pub badge: crate::Badge,") && rust.contains("match signal {}"), "routed, and its signals, of which there are none, go nowhere: {rust}");
    // The check of its signal enum matches the enum, not a reference to
    // it: Rust refuses a match with no arms on a reference
    // (`examples/widgets`' Note compiles it).
    let shape: String = emit_widget_code(&[("m", &c.page)], &code).unwrap();
    assert!(shape.contains("const _: fn(&BadgeSignalOfFile) = |signal| match *signal {\n};"), "{shape}");
}

#[test]
fn each_widget_with_code_gets_its_shape_and_the_check_of_its_signals() {
    let c = compile(PAGE, &|_| None);
    assert!(!has_errors(&c.diags), "{:#?}", c.diags);
    let code: WidgetCode<'_> = WidgetCode { structs: STRUCTS, signals: &c.widget_signals };
    let rust: String = emit_widget_code(&[("m", &c.page)], &code).unwrap();
    // A widget with nothing inside.
    assert!(rust.contains("impl gogo_ui::WidgetShape for crate::Stepper {\n    type Names = StepperNames;\n    type InsideSignal = gogo_ui::NothingInside;"), "{rust}");
    assert!(rust.contains("const _: fn(&StepperSignalOfFile) = |signal| match *signal {\n    StepperSignalOfFile::Changed { .. } => {}\n};"), "{rust}");
    // A widget with two widgets with code inside.
    assert!(rust.contains("pub enum PickerInside {\n    Across(<crate::Stepper as gogo_ui::Widget>::Signal),\n    Down(<crate::Stepper as gogo_ui::Widget>::Signal),\n}"), "{rust}");
    assert!(rust.contains("pub struct PickerWidgets {\n    names: PickerNames,\n    pub across: crate::Stepper,\n    pub down: crate::Stepper,\n}"), "{rust}");
    assert!(rust.contains("            if at == self.names.root {\n                break;\n            }"), "the walk stops at the widget's own root: {rust}");
    assert!(rust.contains("gogo_ui::WidgetSet::route(&mut self.widgets, page, rt, event, heard)"), "{rust}");
    assert!(rust.contains("    PickerSignalOfFile::SizeChanged { .. } => {}"), "{rust}");
}

#[test]
fn a_widget_with_code_no_file_declares_is_refused() {
    let c = compile(PAGE, &|_| None);
    let code: WidgetCode<'_> = WidgetCode { structs: &[("Stepper", "crate::Stepper"), ("Slider9", "crate::Slider9")], signals: &c.widget_signals };
    let why: String = emit_widget_code(&[("m", &c.page)], &code).unwrap_err();
    assert!(why.contains("`widget_code` names widget `Slider9` (crate::Slider9), and no page or `uses` file declares it"), "{why}");
}

/// A repeat whose rows hold a widget with code: one struct per row, a
/// `Handler` variant with the row's number, and the routing through the
/// row's root.
#[test]
fn rows_with_widgets_with_code_get_a_struct_each_and_signal_with_their_number() {
    let c = compile("widget Stepper {\n    signal changed\n    Button plus text=\"+\"\n}\nView page {\n    Column list {\n        repeat lines {\n            Row line {\n                Label text=\"Line\"\n                Stepper amount\n            }\n        }\n    }\n}\n", &|_| None);
    assert!(!has_errors(&c.diags), "{:#?}", c.diags);
    let code: WidgetCode<'_> = WidgetCode { structs: &[("Stepper", "crate::Stepper")], signals: &c.widget_signals };
    let rust: String = emit_rust_with(&c.page, "m", &code).unwrap();
    assert!(rust.contains("        Lines(usize, LinesInside),\n    }"), "{rust}");
    assert!(rust.contains("    pub enum LinesInside {\n        Amount(<crate::Stepper as gogo_ui::Widget>::Signal),\n    }"), "{rust}");
    assert!(rust.contains("    pub struct LinesWidgets {\n        pub names: LinesSlot,\n        pub amount: crate::Stepper,\n    }"), "{rust}");
    assert!(rust.contains("        pub lines: gogo_ui::Rows<LinesWidgets>,"), "{rust}");
    assert!(rust.contains("pub fn sync_rows(&mut self, page: &mut gogo_ui::Page, rt: &mut gogo_ui::Runtime) {") && rust.contains("let mut row: usize = 0;"), "{rust}");
    assert!(rust.contains("self.lines.sync(page, rt, NAMES.lines, |root| LinesWidgets::new(&LinesSlot::at(root)), |row, page, rt| gogo_ui::WidgetSet::load(row, page, rt));"), "{rust}");
    assert!(rust.contains("if let Some(found) = self.lines.iter().position(|one| one.names.root == at) {"), "{rust}");
    assert!(rust.contains("gogo_ui::WidgetSet::route(&mut self.lines[row], page, rt, event, &mut |inside| heard(Handler::Lines(row, inside)))"), "{rust}");
    assert!(rust.contains("                if at == self.names.root {\n                    break;\n                }"), "a row's walk stops at its root: {rust}");
}

#[test]
fn an_instance_with_code_no_routing_reaches_is_refused() {
    // A Stepper that IS a repeat's row: a row's names start below its
    // root, so no name reaches it.
    let c = compile("widget Stepper {\n    signal changed\n    Button plus text=\"+\"\n}\nView page {\n    Column list {\n        repeat items {\n            Stepper step\n        }\n    }\n}\n", &|_| None);
    assert!(!has_errors(&c.diags), "{:#?}", c.diags);
    let code: WidgetCode<'_> = WidgetCode { structs: &[("Stepper", "crate::Stepper")], signals: &c.widget_signals };
    let why: String = emit_rust_with(&c.page, "m", &code).unwrap_err();
    assert!(why.contains("of widget `Stepper` has code, and no routing reaches it"), "{why}");
}

/// The signals one event raises come out in the order they were made, past
/// the ones kept on the stack too, and leave the buffer empty.
#[test]
fn signals_come_out_in_order_past_the_stack_too() {
    let mut signals: gogo_ui::Signals<u32> = gogo_ui::Signals::new();
    for at in 0..3 {
        signals.push(at);
    }
    assert_eq!(signals.drain().collect::<Vec<u32>>(), [0, 1, 2]);
    assert!(signals.is_empty());
    let past: u32 = gogo_ui::widget::SIGNALS_ON_STACK as u32 + 4;
    for at in 0..past {
        signals.push(at);
    }
    assert_eq!(signals.len(), past as usize);
    assert_eq!(signals.drain().collect::<Vec<u32>>(), (0..past).collect::<Vec<u32>>());
    assert!(signals.is_empty() && signals.drain().next().is_none());
}

#[test]
fn a_signal_variant_that_takes_a_handler_s_name_is_refused() {
    let c = compile("widget Stepper {\n    signal changed\n    Button plus text=\"+\"\n}\nView page {\n    Column {\n        Stepper count\n        Button countButton text=\"x\", on:activate=count\n    }\n}\n", &|_| None);
    assert!(!has_errors(&c.diags), "{:#?}", c.diags);
    let code: WidgetCode<'_> = WidgetCode { structs: &[("Stepper", "crate::Stepper")], signals: &c.widget_signals };
    let why: String = emit_rust_with(&c.page, "m", &code).unwrap_err();
    assert!(why.contains("reach the page as `Handler::Count`, and a handler of the file has that name too"), "{why}");
}

/// `source` compiled with `widgets_with_code` named: its errors.
fn errors_with_code(source: &str, widgets_with_code: &[&str], files: &[(&str, &str)]) -> Vec<gogo_ui::Diagnostic> {
    let loader = |rel: &str| files.iter().find(|(name, _)| *name == rel).map(|(_, text)| text.to_string());
    let c = gogo_ui::compile_with(source, &loader, gogo_ui::CompileOptions { widgets_with_code, ..gogo_ui::CompileOptions::default() });
    c.diags.into_iter().filter(|d| d.severity == gogo_ui::Severity::Error).collect()
}

/// A Stepper whose root is a button, a Labeled widget with no code, and a
/// page that wires both placements with an `on:` line (line 10, line 11).
const WIRED: &str = "widget Stepper {
    signal changed
    Button plus text=\"+\"
}
widget Labeled {
    Button tap text=\"t\"
}
View page {
    Column {
        Stepper count on:activate=emit(Counted)
        Labeled plain on:activate=emit(Plain)
    }
}
";

/// An `on:` line on a placement of a widget with code is refused at its
/// line: the widget's code hears the tap, so the page never would. With no
/// code named, or on a widget with no code, the line is the page's.
#[test]
fn an_on_line_on_a_placement_of_a_widget_with_code_is_refused_at_its_line() {
    assert!(errors_with_code(WIRED, &[], &[]).is_empty(), "with no code named the lines are the page's");
    let errors = errors_with_code(WIRED, &["Stepper"], &[]);
    assert_eq!(errors.len(), 1, "{errors:#?}");
    assert_eq!((errors[0].code.as_str(), errors[0].line, errors[0].file.as_str()), ("G5005", 10, ""), "{}", errors[0]);
    assert_eq!(errors[0].message, "`on:activate` on `count`: widget `Stepper` has code, which hears the taps on it, so this line never reaches the page; take it out and match the widget's signal (`Handler::Count(..)`)");
    // Two `on:` words on the line: each is named.
    let both = errors_with_code(&WIRED.replace("on:activate=emit(Counted)", "on:press=emit(Down), on:release=emit(Up)"), &["Stepper"], &[]);
    let words: Vec<&str> = both.iter().map(|e| e.message.split(' ').next().unwrap()).collect();
    assert_eq!(words, ["`on:press`", "`on:release`"], "{both:#?}");
    // With no name there is no variant to name (and no routing either).
    let unnamed = errors_with_code(&WIRED.replace("Stepper count on:", "Stepper on:"), &["Stepper"], &[]);
    assert_eq!(unnamed.len(), 1, "{unnamed:#?}");
    assert_eq!(unnamed[0].message, "`on:activate` on `(no name)`: widget `Stepper` has code, which hears the taps on it, so this line never reaches the page; take it out and match the widget's signal");
}

/// Inside a widget with code, the line would reach that widget's code; a
/// line in a `uses` file is named with that file.
#[test]
fn an_on_line_inside_a_widget_names_who_would_hear_it_and_its_file() {
    let parts: &str = "widget Stepper {
    signal changed
    Button plus text=\"+\"
}
widget Picker {
    signal size_changed
    Column {
        Stepper across on:press=emit(Grab)
    }
}
";
    let page: &str = "uses \"parts.gogo\"\nView page {\n    Picker size\n}\n";
    assert!(errors_with_code(page, &[], &[("parts.gogo", parts)]).is_empty());
    let errors = errors_with_code(page, &["Stepper", "Picker"], &[("parts.gogo", parts)]);
    assert_eq!(errors.len(), 1, "{errors:#?}");
    assert_eq!((errors[0].code.as_str(), errors[0].line, errors[0].file.as_str()), ("G5005", 8, "parts.gogo"), "{}", errors[0]);
    assert_eq!(errors[0].message, "`on:press` on `across`: widget `Stepper` has code, which hears the taps on it, so this line never reaches `Picker`'s code; take it out and match the widget's signal");
    // Picker with no code: the Stepper's signal reaches the page, by a
    // path the message does not spell.
    let errors = errors_with_code(page, &["Stepper"], &[("parts.gogo", parts)]);
    assert_eq!(errors.len(), 1, "{errors:#?}");
    assert!(errors[0].message.ends_with("so this line never reaches the page; take it out and match the widget's signal"), "{}", errors[0]);
    // A widget whose root is itself a placement: the line is still the
    // widget's file's, though the root is built as a line of the page.
    let wrap: String = format!("{parts}widget Wrap {{\n    Stepper inner on:activate=emit(Tap)\n}}\n");
    let errors = errors_with_code("uses \"parts.gogo\"\nView page {\n    Wrap wrapped\n}\n", &["Stepper"], &[("parts.gogo", &wrap)]);
    assert_eq!(errors.len(), 1, "{errors:#?}");
    assert_eq!((errors[0].code.as_str(), errors[0].line, errors[0].file.as_str()), ("G5005", 12, "parts.gogo"), "{}", errors[0]);
}

/// A widget with code and no signal says so; a control's value line is an
/// event, not a tap.
#[test]
fn an_on_line_on_a_widget_with_no_signal_or_on_a_control_is_refused_too() {
    let source: &str = "widget Note {
    Button face text=\"i\"
}
control Fader $value=0, $min=0, $max=10 {
    behavior slider by $value in $min..$max
    signal moved
    Layers width=100, height=20 {
        slot track width=fill, height=10 {
            Rectangle width=fill, height=fill
        }
    }
}
View page {
    Column {
        Note help on:activate=emit(Help)
        Fader volume on:change=emit(Moved)
    }
}
";
    assert!(errors_with_code(source, &[], &[]).is_empty(), "{:#?}", errors_with_code(source, &[], &[]));
    let errors = errors_with_code(source, &["Note", "Fader"], &[]);
    let lines: Vec<(u32, &str)> = errors.iter().map(|e| (e.line, e.message.as_str())).collect();
    assert_eq!(lines, [
        (15, "`on:activate` on `help`: widget `Note` has code, which hears the taps on it, so this line never reaches the page; take it out: `Note` raises no signal"),
        (16, "`on:change` on `volume`: widget `Fader` has code, which hears the events on it, so this line never reaches the page; take it out and match the widget's signal (`Handler::Volume(..)`)"),
    ]);
}

/// `build_pages` hands the compile the widgets it names code for: a line
/// that would be lost stops the build at its file and line, and `BUILD`
/// carries the list for a hot reload.
#[test]
fn a_build_refuses_the_line_and_writes_the_widgets_with_code_down() {
    let dir = std::env::temp_dir().join(format!("gogo_widget_code_on_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let ui = dir.join("ui");
    std::fs::create_dir_all(&ui).unwrap();
    let out = dir.join("pages.rs");
    fn options(dir: &std::path::Path) -> gogo_ui::BuildOptions<'_> {
        gogo_ui::BuildOptions { widget_code: &[("Stepper", "crate::Stepper")], ..gogo_ui::BuildOptions::new(dir) }
    }
    std::fs::write(ui.join("page.gogo"), WIRED).unwrap();
    let why: String = gogo_ui::build_pages(&ui, &out, options(&dir)).unwrap_err();
    assert!(why.contains("page.gogo:10: error G5005: `on:activate` on `count`"), "{why}");
    std::fs::write(ui.join("page.gogo"), WIRED.replace(" on:activate=emit(Counted)", "")).unwrap();
    gogo_ui::build_pages(&ui, &out, options(&dir)).unwrap();
    let source: String = std::fs::read_to_string(&out).unwrap();
    let _ = std::fs::remove_dir_all(&dir);
    assert!(source.contains("    widgets_with_code: &[\"Stepper\"],\n"), "{source}");
}
