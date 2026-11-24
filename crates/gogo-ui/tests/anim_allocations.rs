//! Count of heap allocations an anim costs when it starts, while it
//! plays and when it ends.
//!
//! One test in this file, on purpose: its allocator belongs to the whole
//! test program.

use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::Cell;

use gogo_ui::anim::Finish;
use gogo_ui::layout::{FixedAdvance, Rect};
use gogo_ui::page::{NodeId, Page, StrId};
use gogo_ui::runtime::{Hook, Runtime};
use gogo_ui::{Draw, KeptFrame, compile, has_errors};

thread_local! {
    /// Allocations of this thread since a count was started; `None`
    /// outside a count.
    static COUNT: Cell<Option<usize>> = const { Cell::new(None) };
}

/// The system allocator, counting the allocations of a thread that asked.
///
/// Needed because "an anim allocates nothing" is a statement about
/// allocations, and nothing else in a test can see one.
struct Counting;

unsafe impl GlobalAlloc for Counting {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let _ = COUNT.try_with(|c| {
            if let Some(n) = c.get() {
                c.set(Some(n + 1));
            }
        });
        unsafe { System.alloc(layout) }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        unsafe { System.dealloc(ptr, layout) }
    }

    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        let _ = COUNT.try_with(|c| {
            if let Some(n) = c.get() {
                c.set(Some(n + 1));
            }
        });
        unsafe { System.realloc(ptr, layout, new_size) }
    }
}

#[global_allocator]
static ALLOCATOR: Counting = Counting;

/// How many allocations `work` makes on this thread.
fn allocations(work: impl FnOnce()) -> usize {
    COUNT.with(|c| c.set(Some(0)));
    work();
    COUNT.with(|c| c.replace(None)).expect("the count was started above")
}

/// A page whose root slides in and out by its hooks, as a page of a
/// program does, and whose panel has an anim of its own.
const PAGE: &str = r#"
anim slideIn {
    v.translate.x from=60, to=0, duration=0.3, easing=cubic_out
    and v.opacity from=0, to=1, duration=0.24
}
anim slideOut {
    v.translate.x to=60, duration=0.2, easing=cubic_in
    and v.opacity to=0, duration=0.2
}
anim comeBack {
    v.translate.x from=-30, to=0, duration=0.2, easing=cubic_out
    and v.opacity from=0, to=1, duration=0.16
}
anim grow {
    p.scale from=0.9, to=1, duration=0.1
}
View v on:show=play(slideIn), on:hide=play(slideOut), on:uncover=play(comeBack) {
    Panel p width=10, height=10, background=#E51400
}
"#;

const VIEW: Rect = Rect { x: 0.0, y: 0.0, w: 200.0, h: 100.0 };
const FRAME_SECONDS: f32 = 1.0 / 60.0;

/// A page with its runtime and what a host keeps for it between frames.
struct Shown {
    page: Page,
    rt: Runtime,
    kept: KeptFrame,
    draws: Vec<Draw>,
}

impl Shown {
    fn new(is_root_posed_by_host: bool) -> Shown {
        let compiled = compile(PAGE, &|_| None);
        assert!(!has_errors(&compiled.diags), "{:?}", compiled.diags);
        let page: Page = compiled.page;
        let mut rt: Runtime = Runtime::new(&page);
        rt.is_root_posed_by_host = is_root_posed_by_host;
        Shown { page, rt, kept: KeptFrame::default(), draws: Vec::new() }
    }

    /// One frame as a host runs it: anims move, and page is laid out and
    /// drawn again only when the runtime asks.
    fn frame(&mut self) {
        self.rt.tick(&mut self.page, FRAME_SECONDS);
        if self.rt.dirty {
            self.kept.frame_retained(&mut self.rt, &self.page, VIEW, &FixedAdvance, &mut self.draws, None);
        }
    }

    /// Frames for half a second: longer than any anim of the page.
    fn frames_until_still(&mut self) {
        for _ in 0..30 {
            self.frame();
        }
        assert!(!self.rt.is_moving(), "every anim of the page ended");
    }

    /// What a program does with a page over its life, twice over: page
    /// comes in, a node of it plays an anim, page is uncovered, page
    /// leaves.
    fn comes_and_goes(&mut self, root: NodeId, grow: StrId) {
        let root_pose_read: gogo_ui::DrawnShift = self.rt.root_pose();
        let _ = root_pose_read;
        assert!(self.rt.play_hook(&self.page, root, Hook::Show));
        assert!(self.rt.is_hook_playing(&self.page, root, Hook::Show));
        self.frames_until_still();
        self.rt.play_id(&self.page, grow, Finish::Nothing);
        self.frames_until_still();
        self.rt.play_uncovers(&self.page);
        self.frames_until_still();
        assert!(self.rt.play_hook(&self.page, root, Hook::Hide));
        self.frames_until_still();
        // Opened again while it leaves: the show stops the hide.
        assert!(self.rt.play_hook(&self.page, root, Hook::Hide));
        self.frame();
        assert!(self.rt.play_hook(&self.page, root, Hook::Show));
        self.frames_until_still();
    }
}

/// An anim allocates nothing once its page has played it before: not
/// when it starts, not on a frame of it, not when it ends. So for a hook
/// a host plays, for a play by name, and for a show that stops a hide.
///
/// Needed because a page of a program slides in and out all day, and an
/// allocation at every start is a pause a slow phone shows in the first
/// frame of the slide. Before, a start built four lists and copied every
/// node's pose, and an end copied the lists again.
#[test]
fn an_anim_that_played_before_allocates_nothing() {
    for is_root_posed_by_host in [true, false] {
        let mut shown: Shown = Shown::new(is_root_posed_by_host);
        let root: NodeId = shown.page.find("v").expect("the root");
        let grow: StrId = shown.page.intern_lookup("grow");
        shown.frame();
        shown.comes_and_goes(root, grow);
        shown.comes_and_goes(root, grow);
        let count: usize = allocations(|| shown.comes_and_goes(root, grow));
        assert_eq!(count, 0, "allocations of a page that comes and goes a third time (host draws the root's slide: {is_root_posed_by_host})");
    }
}
