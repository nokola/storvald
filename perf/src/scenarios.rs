//! What is measured: one function a scenario. Each sets the program up,
//! then runs one step inside `counted`.
//!
//! One scenario a process (`storvald-perf run <name>`): what an earlier
//! scenario left warm would change the next one's counts.

use turbodot::Program;
use turbodot::gogo::{self, NodeId, Phase, Rect};

use crate::AppState;
use crate::counts::{Counts, counted};
use crate::pages::{ControlsPage, FirstPage, ListPage};
use crate::ui::{self, controls_page, first_page, list_page};

/// One measured step of the program.
pub struct Scenario {
    /// Its name in every row and file.
    pub name: &'static str,
    /// What is counted, as the results page says it.
    pub what: &'static str,
    /// Sets up, runs the counted part, returns its counts.
    pub run: fn() -> Counts,
}

/// Every scenario, in the order the results page lists them.
pub const SCENARIOS: &[Scenario] = &[
    Scenario { name: "compile_controls_page", what: "gogo compiles the controls page's file", run: compile_controls_page },
    Scenario { name: "start_program", what: "program is made and runs its first frame", run: start_program },
    Scenario { name: "open_controls_page", what: "tap that opens the controls page: page is made", run: open_controls_page },
    Scenario { name: "first_frame_of_controls_page", what: "first frame of the controls page: its first layout", run: first_frame_of_controls_page },
    Scenario { name: "open_list_page", what: "tap that opens the list page: page and its 100 rows are made", run: open_list_page },
    Scenario { name: "first_frame_of_list_page", what: "first frame of the list page: its first layout", run: first_frame_of_list_page },
    Scenario { name: "open_list_page_again", what: "same tap, at page's second open", run: open_list_page_again },
    Scenario { name: "first_frame_of_list_page_again", what: "first frame of the list page at its second open", run: first_frame_of_list_page_again },
    Scenario { name: "draw_list_of_list_page", what: "draw list of the list page, walked whole", run: draw_list_of_list_page },
    Scenario { name: "still_frame", what: "frame of the controls page while nothing changes", run: still_frame },
    Scenario { name: "mouse_move", what: "mouse moves onto another row of the controls page, and the frame after", run: mouse_move },
    Scenario { name: "scroll_step", what: "finger pans the list one step, and the frame after", run: scroll_step },
    Scenario { name: "slider_drag_step", what: "finger drags a slider one step, and the frame after", run: slider_drag_step },
    Scenario { name: "text_change", what: "code writes a new text to one label, and the frame after", run: text_change },
    Scenario { name: "slide_frame", what: "frame while the controls page slides in", run: slide_frame },
    Scenario { name: "leave_and_come_back", what: "controls page leaves and first page comes back, to the end of both anims", run: leave_and_come_back },
];

/// Scenario of that name, if there is one.
pub fn find(name: &str) -> Option<&'static Scenario> {
    SCENARIOS.iter().find(|scenario| scenario.name == name)
}

/// Size pages are laid out for, in epx: a phone held upright.
const SIZE_EPX: (f32, f32) = (480.0, 800.0);
const FRAME_SECONDS: f32 = 0.016;

const FIRST: first_page::Names = first_page::NAMES;
const CONTROLS: controls_page::Names = controls_page::NAMES;
const LIST: list_page::Names = list_page::NAMES;

/// One frame, 16 ms after the last.
fn frame(program: &mut Program<AppState>) {
    program.frame(SIZE_EPX.0, SIZE_EPX.1, FRAME_SECONDS);
}

/// Frames until no page is on its way in or out, and one more.
fn let_the_slides_end(program: &mut Program<AppState>) {
    for _ in 0..120 {
        frame(program);
        if !program.is_page_arriving_or_leaving() {
            frame(program);
            return;
        }
    }
    panic!("a page is still on its way after two seconds");
}

/// Program on its first page, after three frames.
fn started() -> Program<AppState> {
    let mut program: Program<AppState> = Program::new(&ui::BUILT, AppState::default(), FirstPage).expect("the pages can be shown");
    for _ in 0..3 {
        frame(&mut program);
    }
    program
}

/// Taps `button` of the first page and waits for the page it opens.
fn open(program: &mut Program<AppState>, button: NodeId, t: f64) {
    program.tap(button, t);
    let_the_slides_end(program);
}

/// Goes back and waits for the page under to be in place.
fn close(program: &mut Program<AppState>) {
    assert!(program.back(), "a page is open over the first page");
    let_the_slides_end(program);
}

fn compile_controls_page() -> Counts {
    let files = |name: &str| -> Option<String> {
        match name {
            "look.gogo" => Some(include_str!("../ui/look.gogo").to_string()),
            "sliding_page.gogo" => Some(include_str!("../ui/sliding_page.gogo").to_string()),
            _ => None,
        }
    };
    let mut node_count: usize = 0;
    let counts: Counts = counted(|| {
        let compiled = gogo::compile(include_str!("../ui/controls_page.gogo"), &files);
        assert!(!gogo::has_errors(&compiled.diags), "{:?}", compiled.diags);
        node_count = compiled.page.nodes.len();
    });
    assert!(node_count > 100, "the controls page is a page of many nodes: {node_count}");
    counts
}

fn start_program() -> Counts {
    let mut program: Option<Program<AppState>> = None;
    let counts: Counts = counted(|| {
        let mut made: Program<AppState> = Program::new(&ui::BUILT, AppState::default(), FirstPage).expect("the pages can be shown");
        frame(&mut made);
        program = Some(made);
    });
    assert_eq!(program.expect("made above").depth(), 1);
    counts
}

fn open_controls_page() -> Counts {
    let mut program: Program<AppState> = started();
    let counts: Counts = counted(|| program.tap(FIRST.open_controls, 1.0));
    assert!(program.page::<ControlsPage>().is_some(), "the tap opened the controls page");
    counts
}

fn first_frame_of_controls_page() -> Counts {
    let mut program: Program<AppState> = started();
    program.tap(FIRST.open_controls, 1.0);
    let counts: Counts = counted(|| frame(&mut program));
    assert!(counts.nodes_placed > 100, "the first frame lays the page out: {counts:?}");
    counts
}

fn open_list_page() -> Counts {
    let mut program: Program<AppState> = started();
    let counts: Counts = counted(|| program.tap(FIRST.open_list, 1.0));
    assert!(program.page::<ListPage>().is_some(), "the tap opened the list page");
    counts
}

fn first_frame_of_list_page() -> Counts {
    let mut program: Program<AppState> = started();
    program.tap(FIRST.open_list, 1.0);
    let counts: Counts = counted(|| frame(&mut program));
    assert!(counts.nodes_placed > 300, "the first frame lays the page and its rows out: {counts:?}");
    counts
}

fn open_list_page_again() -> Counts {
    let mut program: Program<AppState> = started();
    open(&mut program, FIRST.open_list, 1.0);
    close(&mut program);
    let counts: Counts = counted(|| program.tap(FIRST.open_list, 2.0));
    assert!(program.page::<ListPage>().is_some(), "the tap opened the list page");
    counts
}

fn first_frame_of_list_page_again() -> Counts {
    let mut program: Program<AppState> = started();
    open(&mut program, FIRST.open_list, 1.0);
    close(&mut program);
    program.tap(FIRST.open_list, 2.0);
    let counts: Counts = counted(|| frame(&mut program));
    assert!(counts.nodes_placed > 300, "the first frame lays the page and its rows out: {counts:?}");
    counts
}

fn draw_list_of_list_page() -> Counts {
    let mut program: Program<AppState> = started();
    open(&mut program, FIRST.open_list, 1.0);
    let mut command_count: usize = 0;
    let counts: Counts = counted(|| command_count = gogo::draw(&program.ui().page, program.ui().kept.frame()).len());
    assert!(command_count > 50, "the walk gives the page's draw commands: {command_count}");
    counts
}

fn still_frame() -> Counts {
    let mut program: Program<AppState> = started();
    open(&mut program, FIRST.open_controls, 1.0);
    counted(|| frame(&mut program))
}

fn mouse_move() -> Counts {
    /// From one row of the controls page to the next, in epx.
    const ROW_STEP_EPX: f32 = 52.0;
    let mut program: Program<AppState> = started();
    open(&mut program, FIRST.open_controls, 1.0);
    let slider: Rect = program.ui().rect(CONTROLS.first_slider);
    let (x, y): (f32, f32) = (slider.x + slider.w / 2.0, slider.y + slider.h / 2.0);
    // Two moves ahead of the counted one: first ones make gogo's list of
    // state changes long enough, once.
    for row in 0..2 {
        assert!(program.hover(x, y + ROW_STEP_EPX * row as f32), "the mouse is over the page");
        frame(&mut program);
        frame(&mut program);
    }
    counted(|| {
        program.hover(x, y + ROW_STEP_EPX * 2.0);
        frame(&mut program);
    })
}

fn scroll_step() -> Counts {
    /// How far the finger moves each step, in epx.
    const PAN_STEP_EPX: f32 = 12.0;
    let mut program: Program<AppState> = started();
    open(&mut program, FIRST.open_list, 1.0);
    let list: Rect = program.ui().rect(LIST.row_list);
    let (x, bottom): (f32, f32) = (list.x + list.w / 2.0, list.y + list.h - 40.0);
    let mut t: f64 = 3.0;
    program.touch(Phase::Down, x, bottom, t);
    let mut step = |program: &mut Program<AppState>, index: u32| {
        t += FRAME_SECONDS as f64;
        program.touch(Phase::Move, x, bottom - PAN_STEP_EPX * index as f32, t);
        frame(program);
    };
    for index in 1..=5 {
        step(&mut program, index);
    }
    let before: f32 = program.ui().scroll_of(LIST.row_list)[1];
    let counts: Counts = counted(|| step(&mut program, 6));
    let after: f32 = program.ui().scroll_of(LIST.row_list)[1];
    assert!(after > before, "the counted step moved the rows: {before} to {after}");
    counts
}

fn slider_drag_step() -> Counts {
    /// How far the finger moves each step, in epx.
    const DRAG_STEP_EPX: f32 = 8.0;
    let mut program: Program<AppState> = started();
    open(&mut program, FIRST.open_controls, 1.0);
    let slider: Rect = program.ui().rect(CONTROLS.first_slider);
    let (left, y): (f32, f32) = (slider.x + 20.0, slider.y + slider.h / 2.0);
    let mut t: f64 = 3.0;
    program.touch(Phase::Down, left, y, t);
    let mut step = |program: &mut Program<AppState>, index: u32| {
        t += FRAME_SECONDS as f64;
        program.touch(Phase::Move, left + DRAG_STEP_EPX * index as f32, y, t);
        frame(program);
    };
    for index in 1..=5 {
        step(&mut program, index);
    }
    let before: u32 = program.app().slider_change_count;
    let counts: Counts = counted(|| step(&mut program, 6));
    assert_eq!(program.app().slider_change_count, before + 1, "the counted step moved the slider");
    counts
}

fn text_change() -> Counts {
    let mut program: Program<AppState> = started();
    open(&mut program, FIRST.open_controls, 1.0);
    program.ui_mut().set_text(CONTROLS.first_value, "41");
    frame(&mut program);
    let counts: Counts = counted(|| {
        program.ui_mut().set_text(CONTROLS.first_value, "42");
        frame(&mut program);
    });
    assert_eq!(program.ui().text_of(CONTROLS.first_value), Some("42"));
    counts
}

fn slide_frame() -> Counts {
    let mut program: Program<AppState> = started();
    program.tap(FIRST.open_controls, 1.0);
    frame(&mut program);
    frame(&mut program);
    assert!(program.is_page_arriving_or_leaving(), "the page is on its way in");
    let counts: Counts = counted(|| frame(&mut program));
    assert!(program.is_page_arriving_or_leaving(), "and still is after the counted frame");
    counts
}

fn leave_and_come_back() -> Counts {
    let mut program: Program<AppState> = started();
    // Two rounds ahead of the counted one: first ones make the engine's
    // and gogo's lists long enough, once.
    for round in 0..3 {
        open(&mut program, FIRST.open_controls, 1.0 + round as f64);
        if round < 2 {
            close(&mut program);
        }
    }
    let counts: Counts = counted(|| close(&mut program));
    assert_eq!(program.depth(), 1, "the first page is back");
    counts
}
