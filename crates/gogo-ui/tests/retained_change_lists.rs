//! A frame that changes one box costs the same on a page with a hidden
//! part of 30 nodes and on one with a hidden part of 3000: the kept
//! layout, the kept draw list and a host's copy of the look compare the
//! nodes the resolve names (`ResolveChanges`) and no other.
//!
//! Needed because a control that sits hidden in its page (a Popup's
//! content, a pane that opens later) is that page's nodes: when every
//! frame compared every node, a hidden part of 54 nodes cost a crop
//! rectangle's drag 0.04 ms a frame on a 2014 phone.
//!
//! After every step the host's copy of the look must equal the
//! runtime's, the kept rects a plain `layout` of that look, and the kept
//! draw list a plain `draw` of that frame, so the savings cannot change
//! what shows.
use gogo_ui::draw::{Draw, DrawStats};
use gogo_ui::layout::LayoutStats;
use gogo_ui::runtime::{HELD, Runtime, SELECTED};
use gogo_ui::{ArrangementId, Finish, FixedAdvance, KeptFrame, NodeId, Page, PropKey, Rect, Value, compile, draw, has_errors, layout};

const VIEWPORT: Rect = Rect { x: 0.0, y: 0.0, w: 400.0, h: 800.0 };

/// A page with a box code moves, a column that shows, a Scroll, a button
/// with a timed `when held` look, one with an untimed one, a Check, an anim, a seat an arrangement fills,
/// and a part that starts `gone` and holds `hidden_rows` rows of two
/// nodes each.
fn source(hidden_rows: usize) -> String {
    let mut s = String::from(
        "arrangement wide
arrangement loud
when wide { title.seat = spot }
when loud { title.font_size = 30 }
anim pop { handle.translate.x from=-40, to=0, duration=0.5 }
View page {
    Rectangle handle width=40, height=40, halign=left, valign=top, color=#FF0000FF
    Column shownPart valign=bottom {
        Label title text=\"shown\"
        Check tick text=\"tick\"
        Button plain text=\"plain\" { when held background=#0000FFFF }
        Button press text=\"press\" {
            transition any -> held duration=0.2
            when held background=#00FF00FF
        }
        Scroll strip height=40, allow=vertical { Column stripBody { Label s1 text=\"one\"; Label s2 text=\"two\"; Label s3 text=\"three\"; Label s4 text=\"four\" } }
        Layers free height=30 { seat spot }
    }
    Panel hiddenPart visibility=gone, halign=right, valign=top {
        Column hiddenColumn {
",
    );
    for i in 0..hidden_rows {
        s += &format!("            Row h{i} {{ Label hl{i} text=\"row {i}\" }}\n");
    }
    s += "        }\n    }\n}\n";
    s
}

struct Host {
    page: Page,
    rt: Runtime,
    kept: KeptFrame,
    draws: Vec<Draw>,
}

impl Host {
    fn new(hidden_rows: usize) -> Host {
        let c = compile(&source(hidden_rows), &|_| None);
        assert!(!has_errors(&c.diags), "{:#?}", c.diags);
        let rt = Runtime::new(&c.page);
        let mut host = Host { page: c.page, rt, kept: KeptFrame::default(), draws: Vec::new() };
        host.frame("the first frame");
        host
    }

    fn id(&self, name: &str) -> NodeId {
        self.page.find(name).unwrap_or_else(|| panic!("no node `{name}`"))
    }

    /// One frame on the road a host takes, then the three checks of the
    /// file's head. Returns what the kept layout and the kept draw list
    /// did.
    fn frame(&mut self, what: &str) -> (LayoutStats, DrawStats) {
        self.kept.frame_retained(&mut self.rt, &self.page, VIEWPORT, &FixedAdvance, &mut self.draws, None);
        // Before the checks: they lay the page out a second time.
        let stats: (LayoutStats, DrawStats) = (self.rt.debug_layout_stats(), self.rt.debug_draw_stats());
        self.check(what);
        stats
    }

    fn check(&mut self, what: &str) {
        assert!(self.kept.look() == self.rt.resolved(), "{what}: the host's copy of the look differs from the runtime's look");
        let plain_rects: Vec<Rect> = layout(&self.page, self.rt.resolved(), VIEWPORT, &FixedAdvance);
        let kept_rects: Vec<Rect> = self.rt.layout_retained(&self.page, VIEWPORT, &FixedAdvance).to_vec();
        if let Some(i) = (0..plain_rects.len()).find(|i| kept_rects[*i] != plain_rects[*i]) {
            panic!("{what}: node {i} (`{}`) is laid out at {:?}, a plain layout puts it at {:?}", self.page.str(self.page.nodes[i].name), kept_rects[i], plain_rects[i]);
        }
        let plain_draws: Vec<Draw> = draw(&self.page, self.kept.frame());
        assert_eq!(self.draws, plain_draws, "{what}: the kept draw list differs from a plain walk of the same frame");
    }
}

#[test]
fn a_frame_that_moves_one_box_compares_the_same_nodes_whatever_the_hidden_part_holds() {
    let mut compared: Vec<(u32, u32)> = Vec::new();
    for hidden_rows in [15, 1500] {
        let mut host = Host::new(hidden_rows);
        let handle: NodeId = host.id("handle");
        let mut last: (u32, u32) = (0, 0);
        for step in 0..4 {
            host.rt.set_prop(handle, PropKey::MarginLeft, Value::VNum(10.0 + step as f32 * 3.0));
            host.rt.set_prop(handle, PropKey::MarginTop, Value::VNum(20.0 + step as f32));
            let (layout, draw): (LayoutStats, DrawStats) = host.frame("a margin move");
            assert!(!layout.full && !draw.full, "a margin move took the full road: {layout:?} {draw:?}");
            assert_eq!(layout.placed, 1, "a margin move placed {} boxes", layout.placed);
            assert_eq!(draw.emitted, 1, "a margin move drew {} commands again", draw.emitted);
            last = (layout.compared, draw.compared);
        }
        // The handle, and nothing of the hidden part.
        assert!(last.0 <= 2 && last.1 <= 2, "with {hidden_rows} hidden rows a margin move compared {last:?} nodes");
        compared.push(last);
    }
    assert_eq!(compared[0], compared[1], "a hidden part 100 times the size changed how many nodes a frame compares");
}

#[test]
fn a_part_shown_and_hidden_again_shows_what_a_plain_layout_and_walk_show() {
    let mut host = Host::new(12);
    let handle: NodeId = host.id("handle");
    let hidden_part: NodeId = host.id("hiddenPart");
    let press: NodeId = host.id("press");
    let strip: NodeId = host.id("strip");
    let row: NodeId = host.id("hl3");
    host.rt.set_prop(handle, PropKey::MarginLeft, Value::VNum(30.0));
    host.frame("the handle moved while the part is hidden");
    // A write on a node of the hidden part, then the part shows.
    host.rt.set_text(&mut host.page, row, "written while hidden");
    host.frame("a hidden label written");
    host.rt.set_visible(&mut host.page, hidden_part, true);
    host.frame("the part shows");
    host.rt.set_text(&mut host.page, row, "written in place");
    host.frame("a shown label written in place");
    host.rt.set_prop(handle, PropKey::MarginLeft, Value::VNum(60.0));
    host.frame("the handle moved while the part shows");
    host.rt.set_prop(handle, PropKey::Opacity, Value::VNum(0.5));
    host.frame("the handle dims");
    // A `when` with no transition: the button's words change and no
    // write, tween or anim stands on it.
    let plain: NodeId = host.id("plain");
    host.rt.set_state(plain, HELD, true);
    let (_, draw): (LayoutStats, DrawStats) = host.frame("the plain button pressed");
    assert!(!draw.full && draw.emitted > 0, "a press drew nothing again, or everything: {draw:?}");
    host.rt.set_state(plain, HELD, false);
    host.frame("the plain button let go");
    // A written word away from the value it had at the press, and back:
    // a copy of the look that kept the writes of that frame would call
    // the way back no change.
    host.rt.set_prop(handle, PropKey::Opacity, Value::VNum(0.8));
    host.frame("the handle dims less");
    host.rt.set_prop(handle, PropKey::Opacity, Value::VNum(0.5));
    host.frame("the handle as dim as before");
    // A state bit, a scroll offset and a write in one frame.
    host.rt.set_state(press, HELD, true);
    host.rt.scroll[strip.index()] = [0.0, 12.0];
    host.rt.dirty = true;
    host.rt.set_prop(handle, PropKey::MarginTop, Value::VNum(90.0));
    let (_, draw): (LayoutStats, DrawStats) = host.frame("a press, a pan and a move");
    assert!(!draw.full, "a press, a pan and a move walked the whole page: {draw:?}");
    // The press's color on its way: a tween stands on the button.
    host.rt.tick(&mut host.page, 0.1);
    host.frame("the press's color half way");
    host.rt.tick(&mut host.page, 0.2);
    host.frame("the press's color landed");
    host.rt.set_state(press, HELD, false);
    host.frame("the press let go");
    host.rt.tick(&mut host.page, 0.3);
    host.frame("the color back");
    // A state bit no `when` answers: the Check draws its mark from it.
    let tick: NodeId = host.id("tick");
    let circles_and_quads: usize = host.draws.len();
    host.rt.set_state(tick, SELECTED, true);
    let (_, draw): (LayoutStats, DrawStats) = host.frame("the check ticked");
    assert!(!draw.full, "a tick walked the whole page: {draw:?}");
    assert!(host.draws.len() > circles_and_quads, "the tick drew its mark");
    host.rt.set_state(tick, SELECTED, false);
    host.frame("the tick taken back");
    host.rt.set_visible(&mut host.page, hidden_part, false);
    host.frame("the part hides again");
    host.rt.scroll[strip.index()] = [0.0, 0.0];
    host.rt.dirty = true;
    host.rt.clear_prop(handle, PropKey::MarginLeft);
    host.frame("a write taken back and the pan undone");
    host.rt.set_visible(&mut host.page, hidden_part, true);
    host.rt.set_text(&mut host.page, row, "written as it shows");
    host.frame("the part shows a second time");
}

#[test]
fn an_anim_a_pulse_a_raise_and_a_seat_show_what_a_plain_walk_shows() {
    let mut host = Host::new(12);
    let handle: NodeId = host.id("handle");
    let press: NodeId = host.id("press");
    let title: NodeId = host.id("title");
    let pop = host.page.intern_lookup("pop");
    host.rt.play_id(&host.page, pop, Finish::Nothing);
    host.frame("an anim starts");
    for _ in 0..3 {
        host.rt.tick(&mut host.page, 0.1);
        let (_, draw): (LayoutStats, DrawStats) = host.frame("the anim plays");
        assert!(!draw.full, "an anim on one box walked the whole page: {draw:?}");
    }
    host.rt.tick(&mut host.page, 0.4);
    host.frame("the anim ended");
    host.frame("the frame after the anim ended");
    host.rt.tap_pulse(press);
    host.frame("a pulse starts");
    host.rt.tick(&mut host.page, 0.03);
    host.frame("the pulse swings");
    host.rt.tick(&mut host.page, 0.5);
    host.frame("the pulse ended");
    host.rt.set_raise(handle, 1);
    host.frame("the handle raised");
    host.rt.set_prop(handle, PropKey::MarginLeft, Value::VNum(12.0));
    host.frame("the raised handle moved");
    host.rt.set_raise(handle, 0);
    host.frame("the raise taken back");
    host.rt.go_id(ArrangementId::new(0));
    host.frame("the title sits in the seat");
    assert_eq!(host.rt.resolved().seat[title.index()], host.id("spot"), "the arrangement moved the title into the seat");
    host.rt.set_prop(handle, PropKey::MarginLeft, Value::VNum(40.0));
    host.frame("the handle moved while the title sits in the seat");
    host.rt.go_id(gogo_ui::BASE);
    host.frame("the title back home");
    // A pose that writes a word: no state bit flips on the title, no
    // write, tween or anim stands on it, and it sits where it sat.
    host.rt.go_id(ArrangementId::new(1));
    let (layout, _): (LayoutStats, DrawStats) = host.frame("the title in large letters");
    assert!(!layout.full, "a pose that writes one word laid the whole page out: {layout:?}");
    host.rt.go_id(gogo_ui::BASE);
    host.frame("the title in its own letters");
}

/// The pieces of a frame called one by one, with a pulse that swings
/// between the resolve and `compose_motion`.
///
/// Needed because the resolve names the nodes it touched, and this swing
/// lands on a node it did not: `compose_motion` must name it
/// (`ResolveChanges::restamped`), or the kept draw list keeps the button
/// flat.
#[test]
fn a_motion_that_lands_after_the_resolve_is_drawn() {
    let mut host = Host::new(12);
    let press: NodeId = host.id("press");
    host.rt.resolve_retained(&host.page);
    host.rt.tap_pulse(press);
    host.rt.tick(&mut host.page, 0.03);
    let mut rects: Vec<Rect> = host.rt.layout_retained(&host.page, VIEWPORT, &FixedAdvance).to_vec();
    host.rt.glide_in_place(&mut rects);
    host.rt.compose_motion();
    assert!(host.rt.resolved().motion[press.index()].swing > 0.0, "the pulse swings the button");
    let mut draws: Vec<Draw> = std::mem::take(&mut host.draws);
    host.rt.draw_retained(&host.page, &rects, &mut draws);
    let swings = |list: &[Draw]| list.iter().filter(|d| matches!(d, Draw::SwingPush { .. })).count();
    assert_eq!(swings(&draws), 1, "the kept draw list shows the swing");
}

#[test]
fn a_kept_frame_handed_another_runtime_copies_its_look_whole() {
    let mut first = Host::new(12);
    let mut second = Host::new(12);
    let first_handle: NodeId = first.id("handle");
    let second_title: NodeId = second.id("title");
    // The second runtime is at the same count of resolves as the first,
    // on a look that differs from it.
    second.rt.set_prop(second_title, PropKey::Opacity, Value::VNum(0.5));
    second.frame("the second page's title dims");
    first.rt.set_prop(first_handle, PropKey::MarginLeft, Value::VNum(30.0));
    first.frame("the first page's handle moves");
    // The first host's kept frame follows the second runtime's next resolve.
    second.rt.set_prop(second_title, PropKey::Opacity, Value::VNum(0.6));
    first.kept.frame_retained(&mut second.rt, &second.page, VIEWPORT, &FixedAdvance, &mut first.draws, None);
    assert!(first.kept.look() == second.rt.resolved(), "a kept frame followed another runtime's resolve as if it were its own");
}
