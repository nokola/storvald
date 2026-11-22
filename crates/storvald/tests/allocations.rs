//! Count of heap allocations Storvald's pages cost while they run, by
//! `turbodot::alloc_count`.
//!
//! One test in this file, on purpose: the counts are for the whole process,
//! and cargo runs the tests of one file at the same time.

use storvald::ui::skills_page;
use storvald::{AppState, WINDOW_SIZE_EPX};
use turbodot::Program;
use turbodot::alloc_count::{self, CountingAlloc};
use turbodot::gogo::Rect;

#[global_allocator]
static ALLOC: CountingAlloc = CountingAlloc;

const FRAME_SECONDS: f32 = 0.016;

/// Allocations inside gogo when mouse moves onto another list row, and
/// in the frame that draws that row's hovered look.
const ONTO_ANOTHER_ROW: u64 = 3;
const FRAME_AFTER: u64 = 2;

/// Allocations `work` makes.
fn allocations_of(work: impl FnOnce()) -> u64 {
    let before: u64 = alloc_count::allocations();
    work();
    alloc_count::allocations() - before
}

/// One frame at size window opens at.
fn frame(program: &mut Program<AppState>) {
    program.frame(WINDOW_SIZE_EPX.0, WINDOW_SIZE_EPX.1, FRAME_SECONDS);
}

/// Still pages allocate nothing: not a frame, not a mouse move inside one
/// row, not a frame while a page slides in or out.
///
/// Needed because a program does these most of the time: an allocation
/// in one repeats every frame, and user sees stutter on a slow phone.
#[test]
fn still_pages_a_mouse_move_and_a_slide_allocate_nothing() {
    let mut program: Program<AppState> = storvald::program(false).expect("the pages can be shown");
    for _ in 0..3 {
        frame(&mut program);
    }
    assert_eq!(allocations_of(|| frame(&mut program)), 0, "a frame of a still page");

    let list: Rect = program.ui().rect(skills_page::NAMES.skill_list);
    program.hover(list.x + 20.0, list.y + 20.0);
    frame(&mut program);
    assert_eq!(allocations_of(|| program.hover(list.x + 24.0, list.y + 22.0)), 0, "a move of the mouse inside one row");
    assert_eq!(allocations_of(|| frame(&mut program)), 0, "a frame after that move");
    // Move onto another row changes which row looks hovered. gogo
    // allocates for that change today; check holds the count, so one
    // more shows.
    let onto_another_row: u64 = allocations_of(|| program.hover(list.x + 20.0, list.y + 60.0));
    let frame_after: u64 = allocations_of(|| frame(&mut program));
    assert_eq!((onto_another_row, frame_after), (ONTO_ANOTHER_ROW, FRAME_AFTER), "a move of the mouse onto another row, and the frame after it");
    assert_eq!(allocations_of(|| frame(&mut program)), 0, "the next frame");

    // About page slides in: slide frames lay nothing out again, only move the page.
    program.tap(skills_page::NAMES.about_button, 1.0);
    frame(&mut program);
    frame(&mut program);
    assert!(program.is_sliding());
    assert_eq!(allocations_of(|| frame(&mut program)), 0, "a frame while a page slides in");
    while program.is_sliding() {
        frame(&mut program);
    }
    assert_eq!(allocations_of(|| frame(&mut program)), 0, "a frame of the page that slid in");
}
