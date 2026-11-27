//! Widgets with code on TurboDot. A widget whose file lists `report`
//! lines has a struct of its own (`impl Widget`); the build script names
//! it (`build.rs`), and the build writes the rest: each page's `Widgets`,
//! a field per placed instance, and the routing that hands an event to
//! the instance it happened in and its report to the page.
//!
//! - `stepper`: a number with a minus and a plus button.
//! - `size_picker`: a width and a height, each a Stepper inside it.
//! - `counter_page`: places a Stepper, a SizePicker, and a Stepper inside
//!   a widget with no code (`LabeledStepper`).
//!
//! Run it with `cargo run -p turbodot-example-widgets`.

mod counter_page;
mod size_picker;
mod stepper;

turbodot::built_pages!();

use counter_page::CounterPage;
use turbodot::{Program, WindowOptions};

/// Opens a window on the counter page, stays until it is closed.
///
/// Returns why the page or the window could not show.
fn main() -> Result<(), String> {
    let program: Program<()> = Program::new(&ui::BUILT, (), CounterPage::new())?;
    turbodot::start(WindowOptions { title: "Widgets", width: 480.0, height: 640.0 }, program)
}

#[cfg(test)]
mod tests {
    use turbodot::Program;
    use turbodot::alloc_count::{self, CountingAlloc};
    use turbodot::gogo::{Event, EventKind, NodeId, PropKey, Value, WidgetSet, resolve};
    use turbodot::prelude::*;

    use super::*;
    use crate::size_picker::SizePickerReport;
    use crate::stepper::StepperReport;
    use crate::ui::counter_page::{self, Handler};

    #[global_allocator]
    static ALLOC: CountingAlloc = CountingAlloc;

    /// Held by every test for as long as it runs.
    ///
    /// Needed because the allocation count is of the whole process: a test
    /// on another thread at the same time adds its allocations to it.
    static ONE_TEST_AT_A_TIME: std::sync::Mutex<()> = std::sync::Mutex::new(());

    const PAGE: counter_page::Names = counter_page::NAMES;

    /// Size tests lay the page out for, in epx.
    const SIZE_EPX: (f32, f32) = (480.0, 640.0);

    /// Program on the counter page after its first frame.
    fn started() -> Program<()> {
        let mut program: Program<()> = Program::new(&crate::ui::BUILT, (), CounterPage::new()).expect("the page can be shown");
        program.frame(SIZE_EPX.0, SIZE_EPX.1, 0.0);
        program
    }

    /// What the page heard.
    fn heard(program: &Program<()>) -> Vec<Handler> {
        program.page::<CounterPage>().expect("the counter page is on screen").heard.clone()
    }

    /// The text a label shows.
    fn text(program: &Program<()>, node: NodeId) -> String {
        let ui = program.ui();
        match resolve(&ui.page, &ui.runtime).prop(&ui.page, node, PropKey::Text) {
            Some(Value::VText { text, .. }) => ui.page.str(text).to_string(),
            other => panic!("{node:?} shows no text: {other:?}"),
        }
    }

    /// A tap on a Stepper's plus reaches the Stepper's code, which shows
    /// the new number and reports it; the page hears the report under the
    /// instance's name, in the same call.
    ///
    /// Needed because a widget's code that never hears its taps looks
    /// right and does nothing.
    #[test]
    fn a_tap_on_a_stepper_reaches_its_code_and_its_report_reaches_the_page() {
        let _one = ONE_TEST_AT_A_TIME.lock().unwrap_or_else(|e| e.into_inner());
        let mut program: Program<()> = started();
        program.tap(PAGE.count.plus, 1.0);
        program.tap(PAGE.count.plus, 2.0);
        program.tap(PAGE.count.minus, 3.0);
        assert_eq!(heard(&program), [Handler::Count(StepperReport::Changed(1)), Handler::Count(StepperReport::Changed(2)), Handler::Count(StepperReport::Changed(1))]);
        program.frame(SIZE_EPX.0, SIZE_EPX.1, 0.016);
        assert_eq!(text(&program, PAGE.count.value), "1");
    }

    /// A Stepper inside a widget with code reports to that widget, and the
    /// page hears that widget's report: the size, with both numbers.
    #[test]
    fn a_stepper_inside_a_widget_with_code_reports_through_that_widget() {
        let _one = ONE_TEST_AT_A_TIME.lock().unwrap_or_else(|e| e.into_inner());
        let mut program: Program<()> = started();
        program.tap(PAGE.size.width_stepper.plus, 1.0);
        program.tap(PAGE.size.height_stepper.plus, 2.0);
        program.tap(PAGE.size.height_stepper.plus, 3.0);
        assert_eq!(heard(&program), [Handler::Size(SizePickerReport::SizeChanged { width: 2, height: 1 }), Handler::Size(SizePickerReport::SizeChanged { width: 2, height: 2 }), Handler::Size(SizePickerReport::SizeChanged { width: 2, height: 3 })]);
    }

    /// A Stepper inside a widget with no code is the page's own to route:
    /// the page hears it under the path of its names (`zoom.stepper`).
    #[test]
    fn a_stepper_inside_a_widget_without_code_reports_to_the_page() {
        let _one = ONE_TEST_AT_A_TIME.lock().unwrap_or_else(|e| e.into_inner());
        let mut program: Program<()> = started();
        program.tap(PAGE.zoom.stepper.minus, 1.0);
        assert_eq!(heard(&program), [Handler::ZoomStepper(StepperReport::Changed(-1))]);
    }

    /// A Stepper in a row the page made reports with the row's number; the
    /// row's struct was made and loaded with the row, and the page wrote
    /// into it at once.
    ///
    /// Needed for rows a page makes while it runs (a list of settings,
    /// one row per setting): their widgets would hear nothing otherwise.
    #[test]
    fn a_stepper_in_a_row_reports_with_its_row() {
        let _one = ONE_TEST_AT_A_TIME.lock().unwrap_or_else(|e| e.into_inner());
        let mut program: Program<()> = started();
        let rows: Vec<NodeId> = program.ui().rows(PAGE.lines);
        assert_eq!(rows.len(), crate::counter_page::LINES);
        let second: counter_page::LinesSlot = counter_page::LinesSlot::at(rows[1]);
        assert_eq!(text(&program, second.amount.value), "10", "the page wrote into the row's Stepper at load");
        program.tap(second.amount.plus, 1.0);
        let third: counter_page::LinesSlot = counter_page::LinesSlot::at(rows[2]);
        program.tap(third.amount.minus, 2.0);
        assert_eq!(heard(&program), [Handler::Lines(1, counter_page::LinesInside::Amount(StepperReport::Changed(11))), Handler::Lines(2, counter_page::LinesInside::Amount(StepperReport::Changed(19)))]);
        program.frame(SIZE_EPX.0, SIZE_EPX.1, 0.016);
        assert_eq!(text(&program, second.amount.value), "11");
    }

    /// A row made after the page loaded gets its struct at the next frame,
    /// loaded, without the page asking: its Stepper shows 0 and reports.
    #[test]
    fn a_row_made_later_gets_its_widgets_at_the_next_frame() {
        let _one = ONE_TEST_AT_A_TIME.lock().unwrap_or_else(|e| e.into_inner());
        let mut program: Program<()> = started();
        {
            let ui: &mut PageUi = program.ui_mut();
            ui.runtime.set_count(&mut ui.page, PAGE.lines, crate::counter_page::LINES + 1);
        }
        program.frame(SIZE_EPX.0, SIZE_EPX.1, 0.016);
        let rows: Vec<NodeId> = program.ui().rows(PAGE.lines);
        let last: counter_page::LinesSlot = counter_page::LinesSlot::at(rows[crate::counter_page::LINES]);
        assert_eq!(program.page::<CounterPage>().expect("on screen").widgets.lines.len(), crate::counter_page::LINES + 1);
        assert_eq!(text(&program, last.amount.value), "0", "loaded with its row");
        program.tap(last.amount.plus, 1.0);
        assert_eq!(heard(&program), [Handler::Lines(crate::counter_page::LINES, counter_page::LinesInside::Amount(StepperReport::Changed(1)))]);
    }

    /// The page's own call into a widget reports nothing: Reset sets the
    /// Stepper to 0 and the page hears only the Reset button.
    #[test]
    fn a_page_s_own_call_into_a_widget_reports_nothing() {
        let _one = ONE_TEST_AT_A_TIME.lock().unwrap_or_else(|e| e.into_inner());
        let mut program: Program<()> = started();
        program.tap(PAGE.count.plus, 1.0);
        program.tap(PAGE.reset, 2.0);
        assert_eq!(heard(&program), [Handler::Count(StepperReport::Changed(1)), Handler::Reset]);
        program.frame(SIZE_EPX.0, SIZE_EPX.1, 0.016);
        assert_eq!(text(&program, PAGE.count.value), "0");
    }

    /// The widgets load when the page is made, a widget's inner widgets
    /// before it: the SizePicker's Steppers show 1, the value it set after
    /// they showed 0.
    #[test]
    fn widgets_load_before_the_page_shows_inner_ones_first() {
        let _one = ONE_TEST_AT_A_TIME.lock().unwrap_or_else(|e| e.into_inner());
        let program: Program<()> = started();
        assert_eq!(text(&program, PAGE.count.value), "0");
        assert_eq!(text(&program, PAGE.size.width_stepper.value), "1");
        assert_eq!(text(&program, PAGE.size.height_stepper.value), "1");
        assert_eq!(program.page::<CounterPage>().expect("on screen").widgets.size.size(), (1, 1));
    }

    /// Routing an event to a widget two levels down, its report up through
    /// the widget around it to the page, and the frame after it, allocate
    /// nothing once warm.
    ///
    /// A frame comes after each event, as in a program: the runtime keeps
    /// the texts a widget rewrote until the next layout.
    #[test]
    fn routing_an_event_to_a_widget_allocates_nothing() {
        let _one = ONE_TEST_AT_A_TIME.lock().unwrap_or_else(|e| e.into_inner());
        let mut program: Program<()> = started();
        let mut widgets: counter_page::Widgets = counter_page::Widgets::new();
        {
            let ui = program.ui_mut();
            widgets.load(&mut ui.page, &mut ui.runtime);
        }
        let plus: Event = Event { node: PAGE.size.width_stepper.plus, kind: EventKind::Activate, action: None };
        let minus: Event = Event { node: PAGE.size.width_stepper.minus, kind: EventKind::Activate, action: None };
        // And a Stepper in a row.
        let row: NodeId = program.ui().rows(PAGE.lines)[1];
        let row_plus: Event = Event { node: counter_page::LinesSlot::at(row).amount.plus, kind: EventKind::Activate, action: None };
        let row_minus: Event = Event { node: counter_page::LinesSlot::at(row).amount.minus, kind: EventKind::Activate, action: None };
        let mut reports: u32 = 0;
        let mut step_and_frame = |program: &mut Program<()>, reports: &mut u32| {
            for event in [&plus, &minus, &row_plus, &row_minus] {
                let ui = program.ui_mut();
                assert!(widgets.route(&mut ui.page, &mut ui.runtime, event, &mut |_| *reports += 1), "the event is a widget's");
            }
            program.frame(SIZE_EPX.0, SIZE_EPX.1, 0.016);
        };
        for _ in 0..10 {
            step_and_frame(&mut program, &mut reports);
        }
        let before: u64 = alloc_count::allocations();
        for _ in 0..50 {
            step_and_frame(&mut program, &mut reports);
        }
        let made: u64 = alloc_count::allocations() - before;
        assert_eq!(reports, 240, "each event reached the page as one report");
        assert_eq!(made, 0, "200 routed events and their 50 frames allocated {made} times");
    }

    /// The counter page's `Widgets`, counting what the engine calls.
    struct CountedWidgets {
        widgets: counter_page::Widgets,
        loads: u32,
        frames: u32,
        after_layouts: u32,
        /// How many frames the widgets ask for, from the first.
        frames_asked: u32,
    }

    impl WidgetSet<Handler> for CountedWidgets {
        fn route(&mut self, page: &mut turbodot::gogo::Page, rt: &mut turbodot::gogo::Runtime, event: &Event, heard: &mut dyn FnMut(Handler)) -> bool {
            self.widgets.route(page, rt, event, heard)
        }
        fn load(&mut self, page: &mut turbodot::gogo::Page, rt: &mut turbodot::gogo::Runtime) {
            self.loads += 1;
            self.widgets.load(page, rt);
        }
        fn frame(&mut self, page: &mut turbodot::gogo::Page, rt: &mut turbodot::gogo::Runtime, dt_sec: f32) -> bool {
            self.frames += 1;
            let _ = self.widgets.frame(page, rt, dt_sec);
            self.frames <= self.frames_asked
        }
        fn after_layout(&mut self, page: &mut turbodot::gogo::Page, rt: &mut turbodot::gogo::Runtime, rects: &[turbodot::gogo::Rect]) {
            assert!(!rects.is_empty(), "the page's rectangles of this frame");
            self.after_layouts += 1;
            self.widgets.after_layout(page, rt, rects);
        }
    }

    /// The counter page with `CountedWidgets`.
    struct CountingPage {
        widgets: CountedWidgets,
    }
    impl Page for CountingPage {
        const PAGE: &'static BuiltPage = &counter_page::PAGE;
        type Handler = Handler;
        fn widgets(&mut self) -> Option<&mut dyn WidgetSet<Handler>> {
            Some(&mut self.widgets)
        }
        fn handle(&mut self, _host: &mut Host, _event: PageEvent<Handler>) {}
    }

    /// The engine loads the widgets once, calls their `frame` every frame
    /// before the layout and their `after_layout` after a frame that laid
    /// the page out; a frame a widget asks for keeps the program drawing.
    ///
    /// Needed because a widget with something on its way (a hold that
    /// repeats, a fade) would stop between two touches.
    #[test]
    fn the_engine_calls_the_widgets_each_frame_and_after_layout() {
        let _one = ONE_TEST_AT_A_TIME.lock().unwrap_or_else(|e| e.into_inner());
        let page: CountingPage = CountingPage { widgets: CountedWidgets { widgets: counter_page::Widgets::new(), loads: 0, frames: 0, after_layouts: 0, frames_asked: 3 } };
        let mut program: Program<()> = Program::new(&crate::ui::BUILT, (), page).expect("the page can be shown");
        let mut asks: Vec<bool> = Vec::new();
        for _ in 0..5 {
            asks.push(program.frame(SIZE_EPX.0, SIZE_EPX.1, 0.016));
        }
        let counted: &CountedWidgets = &program.page::<CountingPage>().expect("on screen").widgets;
        assert_eq!(counted.loads, 1, "loaded once, when the page was made");
        assert_eq!(counted.frames, 5, "every frame");
        assert!(counted.after_layouts >= 1, "after the first layout at least");
        assert_eq!(asks[..3], [true, true, true], "the frames the widgets asked for: {asks:?}");
        assert_eq!(asks[4], false, "and none after: {asks:?}");
        // A rewrite of a widget's text lays the page out again: its
        // `after_layout` hears of it.
        let before: u32 = counted.after_layouts;
        program.tap(PAGE.count.plus, 1.0);
        program.frame(SIZE_EPX.0, SIZE_EPX.1, 0.016);
        assert!(program.page::<CountingPage>().expect("on screen").widgets.after_layouts > before, "the layout of the new number");
    }

    /// The counter page as a cached page, with `CountedWidgets`.
    struct CachedCounter {
        widgets: CountedWidgets,
        heard: Vec<Handler>,
    }
    impl CachedPage for CachedCounter {
        const PAGE: &'static BuiltPage = &counter_page::PAGE;
        type Handler = Handler;
        fn widgets(&mut self) -> Option<&mut dyn WidgetSet<Handler>> {
            Some(&mut self.widgets)
        }
        fn handle(&mut self, _host: &mut Host, event: PageEvent<Handler>) {
            self.heard.push(event.handler);
        }
        fn reopened(&mut self, _fresh: CachedCounter) {}
    }

    /// A cached page's widgets are loaded, framed and laid out, and its
    /// taps routed, as a plain page's.
    #[test]
    fn a_cached_page_s_widgets_hear_the_engine_too() {
        let _one = ONE_TEST_AT_A_TIME.lock().unwrap_or_else(|e| e.into_inner());
        let page: CachedCounter = CachedCounter { widgets: CountedWidgets { widgets: counter_page::Widgets::new(), loads: 0, frames: 0, after_layouts: 0, frames_asked: 1 }, heard: Vec::new() };
        let mut program: Program<()> = Program::new(&crate::ui::BUILT, (), page).expect("the page can be shown");
        let asked: bool = program.frame(SIZE_EPX.0, SIZE_EPX.1, 0.016);
        program.tap(PAGE.size.width_stepper.plus, 1.0);
        program.frame(SIZE_EPX.0, SIZE_EPX.1, 0.016);
        let cached: &CachedCounter = program.page::<CachedCounter>().expect("on screen");
        assert_eq!(cached.heard, [Handler::Size(SizePickerReport::SizeChanged { width: 2, height: 1 })]);
        assert_eq!((cached.widgets.loads, cached.widgets.frames), (1, 2));
        assert!(cached.widgets.after_layouts >= 1 && asked, "laid out, and the frame the widgets asked for: {asked}");
        assert_eq!(text(&program, PAGE.size.width_stepper.value), "2");
    }

    /// The counter page's file, with a struct that does not hand over its
    /// `Widgets`.
    struct ForgetfulPage;
    impl Page for ForgetfulPage {
        const PAGE: &'static BuiltPage = &counter_page::PAGE;
        type Handler = Handler;
        fn handle(&mut self, _host: &mut Host, _event: PageEvent<Handler>) {}
    }

    /// A page whose file places widgets with code and that hands over no
    /// `Widgets` stops when it is made, naming the fix.
    ///
    /// Needed because its widgets would look right and do nothing.
    #[test]
    #[should_panic(expected = "places widgets with code")]
    fn a_page_that_forgets_its_widgets_stops_when_it_is_made() {
        let _one = ONE_TEST_AT_A_TIME.lock().unwrap_or_else(|e| e.into_inner());
        let mut program: Program<()> = Program::new(&crate::ui::BUILT, (), ForgetfulPage).expect("the page can be shown");
        program.frame(SIZE_EPX.0, SIZE_EPX.1, 0.0);
    }
}
