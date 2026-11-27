//! A widget's code: the Rust struct that drives one placed instance of a
//! widget, and the generated sets that hand it its events.
//!
//! Needed so a widget with logic (a slider row, a switch, a color picker)
//! is written once and wired by the build: a page that places it gets
//! the widget's reports as variants of its own `Handler`, and cannot
//! forget to hand the widget its events (todo-storvald.md/SV69 in
//! FanRust, the app this came from).
//!
//! How the parts fit:
//! - The widget's file lists what it reports: `report size_changed`.
//! - The app's build script names the struct of each widget with code
//!   (`BuildOptions::widget_code`). The build then writes, per widget,
//!   its `WidgetShape` (its names, the reports of the widgets inside it,
//!   their routing), and per page a `Widgets` struct: one field per
//!   instance with code, and a `WidgetSet` impl that routes events.
//! - The app writes `impl Widget` for the struct: its report enum, `new`,
//!   `handle`, and what it does on load, per frame and after layout.
//!
//! A report means the user did something: a page's own call into a
//! widget's struct reports nothing.

use crate::input::Event;
use crate::layout::Rect;
use crate::page::Page;
use crate::runtime::Runtime;

/// The part of a widget's code the build writes (`build_pages`, from the
/// widget's file and `BuildOptions::widget_code`).
///
/// Needed so the widget's author writes only what the widget does: the
/// names it is built over and the widgets inside it come from the file.
pub trait WidgetShape {
    /// The instance's node ids (`<Widget>Names`).
    type Names: Copy;
    /// The reports of the widgets with code inside this one
    /// (`<Widget>Inside`), or `NothingInside` when it has none.
    type InsideReport;

    /// Hands `event` to the widget with code inside this one that holds
    /// its node, and that widget's reports to `heard`.
    ///
    /// Returns false when no widget inside holds the node: the event is
    /// this widget's own.
    fn route_inside(&mut self, page: &mut Page, rt: &mut Runtime, event: &Event, heard: &mut dyn FnMut(Self::InsideReport)) -> bool;

    /// `Widget::load_all` of every widget with code inside this one.
    fn load_inside(&mut self, page: &mut Page, rt: &mut Runtime);

    /// `Widget::frame_all` of every widget with code inside this one.
    ///
    /// Returns true while one of them needs more frames.
    fn frame_inside(&mut self, page: &mut Page, rt: &mut Runtime, dt_sec: f32) -> bool;

    /// `Widget::after_layout_all` of every widget with code inside this one.
    fn after_layout_inside(&mut self, page: &mut Page, rt: &mut Runtime, rects: &[Rect]);
}

/// The struct that drives one placed instance of a widget: what the app
/// writes for a widget with code.
///
/// The page that places the widget owns the struct (a field of its
/// generated `Widgets`), so the page's code calls it directly, with no
/// lookup.
pub trait Widget: WidgetShape + Sized {
    /// What the widget tells the page. Its variants are exactly the
    /// widget's `report` names in the file, in Rust spelling: the build
    /// checks it.
    type Report: Clone + std::fmt::Debug + PartialEq;

    /// The struct over one instance, before the page shows.
    fn new(names: &Self::Names) -> Self;

    /// An event on the widget's own nodes, or a report of a widget with
    /// code inside it. Reports go out through `cx.report`.
    fn handle(&mut self, cx: &mut WidgetCx<'_, Self::Report>, event: WidgetEvent<'_, Self::InsideReport>);

    /// Writes what the widget keeps into its nodes, once the page is
    /// made. The widgets inside it have loaded first.
    fn loaded(&mut self, page: &mut Page, rt: &mut Runtime) {
        let _ = (page, rt);
    }

    /// Once a frame, before the page is laid out.
    ///
    /// Returns true while the widget needs more frames.
    fn frame(&mut self, page: &mut Page, rt: &mut Runtime, dt_sec: f32) -> bool {
        let _ = (page, rt, dt_sec);
        false
    }

    /// After the page was laid out: each node has its rectangle of this
    /// frame in `rects`.
    fn after_layout(&mut self, page: &mut Page, rt: &mut Runtime, rects: &[Rect]) {
        let _ = (page, rt, rects);
    }

    /// Hands `event`, an event on a node of this instance, to the widget
    /// inside that holds it, or to this widget; its reports to `report`.
    ///
    /// The generated sets call it; a page calls its set, not this.
    fn route(&mut self, page: &mut Page, rt: &mut Runtime, event: &Event, report: &mut dyn FnMut(Self::Report)) {
        let mut heard: Reports<Self::InsideReport> = Reports::new();
        let is_inside: bool = self.route_inside(page, rt, event, &mut |inside| heard.push(inside));
        let mut cx: WidgetCx<'_, Self::Report> = WidgetCx { page, rt, report };
        if !is_inside {
            self.handle(&mut cx, WidgetEvent::Own(event));
            return;
        }
        for inside in heard.drain() {
            self.handle(&mut cx, WidgetEvent::Inside(inside));
        }
    }

    /// `loaded` of the widgets inside, then of this one.
    fn load_all(&mut self, page: &mut Page, rt: &mut Runtime) {
        self.load_inside(page, rt);
        self.loaded(page, rt);
    }

    /// `frame` of the widgets inside, then of this one.
    ///
    /// Returns true while one of them needs more frames.
    fn frame_all(&mut self, page: &mut Page, rt: &mut Runtime, dt_sec: f32) -> bool {
        let is_inside_moving: bool = self.frame_inside(page, rt, dt_sec);
        self.frame(page, rt, dt_sec) | is_inside_moving
    }

    /// `after_layout` of the widgets inside, then of this one.
    fn after_layout_all(&mut self, page: &mut Page, rt: &mut Runtime, rects: &[Rect]) {
        self.after_layout_inside(page, rt, rects);
        self.after_layout(page, rt, rects);
    }
}

/// What `Widget::handle` gets.
#[derive(Debug)]
pub enum WidgetEvent<'a, I> {
    /// An event on one of the widget's own nodes.
    Own(&'a Event),
    /// A report of a widget with code inside this one, named by that
    /// instance (`BrushOptionsInside::SliderSize(...)`).
    Inside(I),
}

/// What a widget's `handle` works with: the page it sits on, that page's
/// runtime, and the way out for its reports.
pub struct WidgetCx<'a, R> {
    pub page: &'a mut Page,
    pub rt: &'a mut Runtime,
    report: &'a mut dyn FnMut(R),
}

impl<'a, R> WidgetCx<'a, R> {
    /// A context over `page` and `rt` whose reports go to `report`.
    ///
    /// Needed by a test that drives one widget by hand, and by a host
    /// whose page is not built (a page compiled at run time).
    pub fn new(page: &'a mut Page, rt: &'a mut Runtime, report: &'a mut dyn FnMut(R)) -> WidgetCx<'a, R> {
        WidgetCx { page, rt, report }
    }

    /// Tells the page that places the widget: the page gets it in the same
    /// call, as a variant of its `Handler`.
    pub fn report(&mut self, report: R) {
        (self.report)(report)
    }
}

/// The routing of a page's (or a widget's) instances with code: the
/// generated `Widgets` of a page, and `<Widget>Widgets` of a widget.
///
/// `H` is what a report becomes on that level: the page's `Handler`, or
/// the widget's `<Widget>Inside`.
pub trait WidgetSet<H> {
    /// Hands `event` to the instance with code that holds its node, and
    /// that instance's reports, as `H`, to `heard`.
    ///
    /// Returns false when no instance with code holds the node: the event
    /// is the level's own (the page's handler table answers it).
    fn route(&mut self, page: &mut Page, rt: &mut Runtime, event: &Event, heard: &mut dyn FnMut(H)) -> bool;

    /// `Widget::load_all` of every instance, in the file's order.
    fn load(&mut self, page: &mut Page, rt: &mut Runtime);

    /// `Widget::frame_all` of every instance.
    ///
    /// Returns true while one of them needs more frames.
    fn frame(&mut self, page: &mut Page, rt: &mut Runtime, dt_sec: f32) -> bool;

    /// `Widget::after_layout_all` of every instance.
    fn after_layout(&mut self, page: &mut Page, rt: &mut Runtime, rects: &[Rect]);
}

/// One struct per row a `repeat` made, in the order the rows were made:
/// the widgets with code in each row (the build's `<Repeat>Widgets`).
///
/// Needed because a page makes rows while it runs: each row's widgets
/// need their struct, made and loaded with the row, and kept as long as
/// the row (a row is never taken away, only hidden).
pub struct Rows<T> {
    rows: Vec<T>,
}

impl<T> Rows<T> {
    /// No rows yet, nothing allocated.
    pub fn new() -> Rows<T> {
        Rows { rows: Vec::new() }
    }

    /// Makes the struct of every row `repeat` made since the last call,
    /// with `make` from the row's root, and loads it with `load`.
    ///
    /// The generated `Widgets` calls it when it loads, before it routes an
    /// event and after each layout; a page that made rows and wants their
    /// structs at once calls `Widgets::sync_rows`.
    ///
    /// Allocates only when a row was made.
    pub fn sync(&mut self, page: &mut Page, rt: &mut Runtime, repeat: crate::page::NodeId, make: impl Fn(crate::page::NodeId) -> T, load: impl Fn(&mut T, &mut Page, &mut Runtime)) {
        while self.rows.len() < rt.row_slots(repeat).len() {
            let root: crate::page::NodeId = rt.row_slots(repeat)[self.rows.len()];
            let mut row: T = make(root);
            load(&mut row, page, rt);
            self.rows.push(row);
        }
    }
}

impl<T> Default for Rows<T> {
    fn default() -> Rows<T> {
        Rows::new()
    }
}

impl<T> std::ops::Deref for Rows<T> {
    type Target = [T];

    fn deref(&self) -> &[T] {
        &self.rows
    }
}

impl<T> std::ops::DerefMut for Rows<T> {
    fn deref_mut(&mut self) -> &mut [T] {
        &mut self.rows
    }
}

/// The reports of a widget that has no widgets with code inside it: no
/// value can be made.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NothingInside {}

/// The reports of a widget that reports nothing (no `report` line).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NoReport {}

/// Reports kept for a moment, until the struct that made them is free
/// again: on the stack for the first few, so routing allocates nothing.
///
/// Needed because a widget inside another is a field of it: its reports
/// cannot reach the outer widget's `handle` while it still runs.
pub struct Reports<T> {
    first: [Option<T>; REPORTS_ON_STACK],
    count: usize,
    /// Past `REPORTS_ON_STACK`: one event that reports that much is rare,
    /// and allocates only then.
    more: Vec<T>,
}

/// How many reports of one event `Reports` keeps without allocating.
pub const REPORTS_ON_STACK: usize = 8;

impl<T> Reports<T> {
    /// No reports, nothing allocated.
    pub fn new() -> Reports<T> {
        Reports { first: [const { None }; REPORTS_ON_STACK], count: 0, more: Vec::new() }
    }

    /// Keeps `report` after the ones before it.
    pub fn push(&mut self, report: T) {
        if self.count < REPORTS_ON_STACK {
            self.first[self.count] = Some(report);
        } else {
            self.more.push(report);
        }
        self.count += 1;
    }

    /// How many reports are kept.
    pub fn len(&self) -> usize {
        self.count
    }

    /// True when no report is kept.
    pub fn is_empty(&self) -> bool {
        self.count == 0
    }

    /// The reports in the order they came, each taken out.
    pub fn drain(&mut self) -> impl Iterator<Item = T> + '_ {
        let on_stack: usize = self.count.min(REPORTS_ON_STACK);
        self.count = 0;
        self.first[..on_stack].iter_mut().filter_map(Option::take).chain(self.more.drain(..))
    }
}

impl<T> Default for Reports<T> {
    fn default() -> Reports<T> {
        Reports::new()
    }
}
