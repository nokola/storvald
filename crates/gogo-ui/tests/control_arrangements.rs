//! Arrangements inside a control: a `widget` or `control` declares
//! `arrangement` lines beside its body, and each instance is in one of
//! them on its own.
//!
//! Needed because a page's `arrangement` is one for the whole page: a
//! switch with poses, used twice on one page, flipped both at once, so
//! such a switch had to be a small page of its own hosted by code.
//!
//! After every frame the host's copy of the look must equal the
//! runtime's, the kept rects a plain `layout` of that look, and the kept
//! draw list a plain `draw` of that frame: a flip of one instance names
//! the nodes it changed like any other change.
use gogo_ui::layout::{FixedAdvance, Rect, layout};
use gogo_ui::page::{Action, State};
use gogo_ui::{ArrangementId, BASE, Diagnostic, Draw, EventList, Input, KeptFrame, NodeId, Orientation, Page, Phase, Runtime, ScreenClass, Severity, Touch, compile, draw, has_errors};

const VIEW: Rect = Rect { x: 0.0, y: 0.0, w: 400.0, h: 800.0 };
/// One frame of a 60 Hz screen, in seconds.
const FRAME: f32 = 1.0 / 60.0;

/// A switch with two arrangements. `on` moves the knob to the right end
/// of its well (an alignment: only a box glide can time it) and tints
/// it; `wide` shows a second box, so the switch grows by 30. A tap on
/// the well toggles `on`; a tap on the knob goes back to no arrangement.
const SWITCH: &str = "widget Switch {
    arrangement on
    arrangement wide
    transition any -> on duration=1
    transition any -> wide duration=1
    when on {
        knob.halign = right
        knob.background = #00FF00FF
    }
    when wide { extra.visibility = visible }
    Row box height=40, spacing=0 {
        Button well width=60, height=40, background=#00000000, on:activate=toggle(on) {
            Button knob width=20, height=20, halign=left, valign=top, background=#FF0000FF, on:activate=go(base)
        }
        Rectangle extra width=30, height=40, color=#0000FFFF, visibility=gone
    }
}
";

/// Two switches side by side and a box after them.
fn two_switches() -> String {
    format!("{SWITCH}View page {{\n    Row strip halign=left, valign=top, spacing=0 {{\n        Switch a\n        Switch b\n        Rectangle after width=10, height=10, color=#FFFFFFFF\n    }}\n}}\n")
}

fn diags(src: &str) -> Vec<Diagnostic> {
    compile(src, &|_| None).diags
}

/// The text of every error `src` gets, one per line.
fn errors(src: &str) -> String {
    diags(src).iter().filter(|d| d.severity == Severity::Error).map(|d| format!("{} {}", d.code, d.message)).collect::<Vec<_>>().join("\n")
}

/// One page run the way a host runs it, with the three checks of the
/// file's head after every frame.
struct Fx {
    page: Page,
    rt: Runtime,
    input: Input,
    kept: KeptFrame,
    draws: Vec<Draw>,
    /// The touch clock, in seconds.
    now: f64,
}

impl Fx {
    fn new(src: &str) -> Fx {
        let c = compile(src, &|_| None);
        assert!(!has_errors(&c.diags), "{src}\n{:#?}", c.diags);
        let rt = Runtime::new(&c.page);
        let mut fx = Fx { page: c.page, rt, input: Input::new(), kept: KeptFrame::default(), draws: Vec::new(), now: 0.0 };
        fx.frame(0.0);
        fx
    }

    fn frame(&mut self, dt: f32) {
        self.rt.tick(&mut self.page, dt);
        self.kept.frame_retained(&mut self.rt, &self.page, VIEW, &FixedAdvance, &mut self.draws, None);
        assert!(self.kept.look() == self.rt.resolved(), "the host's copy of the look differs from the runtime's look");
        let plain_rects: Vec<Rect> = layout(&self.page, self.rt.resolved(), VIEW, &FixedAdvance);
        let kept_rects: Vec<Rect> = self.rt.layout_retained(&self.page, VIEW, &FixedAdvance).to_vec();
        if let Some(i) = (0..plain_rects.len()).find(|i| kept_rects[*i] != plain_rects[*i]) {
            panic!("node {i} (`{}`) is laid out at {:?}, a plain layout puts it at {:?}", self.page.str(self.page.nodes[i].name), kept_rects[i], plain_rects[i]);
        }
        let plain_draws: Vec<Draw> = draw(&self.page, self.kept.frame());
        assert_eq!(self.draws, plain_draws, "the kept draw list differs from a plain walk of the same frame");
    }

    /// Frames of a 60 Hz screen until `seconds` have passed.
    fn frames(&mut self, seconds: f32) {
        for _ in 0..(seconds / FRAME).ceil() as u32 {
            self.frame(FRAME);
        }
    }

    fn id(&self, name: &str) -> NodeId {
        self.page.find(name).unwrap_or_else(|| panic!("no node `{name}`"))
    }

    /// Where `node` shows in the last frame.
    fn rect_of(&self, node: NodeId) -> Rect {
        self.kept.rects()[node.index()]
    }

    fn rect(&self, name: &str) -> Rect {
        self.rect_of(self.id(name))
    }

    /// The arrangement `name` of the instance `instance`, as `go_in_id`
    /// takes it.
    fn arrangement(&self, instance: NodeId, name: &str) -> ArrangementId {
        self.page.control_arrangement(instance, name).unwrap_or_else(|| panic!("no arrangement `{name}`"))
    }

    /// Sets `instance` to `name` with no motion, and draws a frame.
    fn set(&mut self, instance: &str, name: &str) {
        let root: NodeId = self.id(instance);
        let to: ArrangementId = self.arrangement(root, name);
        self.rt.go_in_instant_id(root, to);
        self.frame(0.0);
    }

    fn touch(&mut self, phase: Phase, x: f32, y: f32) -> EventList {
        self.now += 0.01;
        self.input.touch(&self.page, &mut self.rt, self.kept.frame(), &FixedAdvance, Touch { phase, x, y, t: self.now, id: 1, ..Touch::PLAIN })
    }

    /// A tap at a point of `node` (`at_x`, `at_y` from its top left), a
    /// second after the touch before it, then a frame.
    fn tap_at(&mut self, node: NodeId, at_x: f32, at_y: f32) {
        let r: Rect = self.rect_of(node);
        self.now += 1.0;
        self.touch(Phase::Down, r.x + at_x, r.y + at_y);
        self.touch(Phase::Up, r.x + at_x, r.y + at_y);
        self.frame(0.0);
    }
}

#[test]
fn two_instances_of_one_widget_flip_one_at_a_time() {
    let mut fx = Fx::new(&two_switches());
    assert_eq!((fx.rect("a/knob").x, fx.rect("b/knob").x), (0.0, 60.0), "both at rest");
    fx.set("a", "on");
    assert_eq!((fx.rect("a/knob").x, fx.rect("b/knob").x), (40.0, 60.0), "`a` is on, `b` is not");
    fx.set("b", "on");
    fx.set("a", "wide");
    assert_eq!((fx.rect("a/knob").x, fx.rect("a/extra").w), (0.0, 30.0), "`a` left `on` for `wide`: one arrangement at a time");
    assert_eq!((fx.rect("b").x, fx.rect("b/knob").x, fx.rect("b/extra").w), (90.0, 130.0, 0.0), "`b` is still on, moved over by the wider `a`");
    let (a, b): (NodeId, NodeId) = (fx.id("a"), fx.id("b"));
    assert_eq!(fx.rt.arrangement_in_id(a), fx.arrangement(a, "wide"));
    assert_eq!(fx.rt.arrangement_in_id(b), fx.arrangement(b, "on"));
    fx.rt.go_in_instant_id(a, BASE);
    fx.frame(0.0);
    assert_eq!((fx.rect("a/knob").x, fx.rect("a/extra").w, fx.rt.arrangement_in_id(a)), (0.0, 0.0, BASE));
}

#[test]
fn a_flip_runs_the_when_blocks_of_that_instance_and_of_no_other_node() {
    let mut fx = Fx::new(&two_switches());
    let a: NodeId = fx.id("a");
    let on: ArrangementId = fx.arrangement(a, "on");
    fx.rt.go_in_instant_id(a, on);
    fx.frame(0.0);
    // `a/knob` (the `when on`) and `a/extra` (the `when wide`) watch
    // `a`'s arrangement; `b`'s two watch `b`'s.
    assert_eq!(fx.rt.debug_laddered(), 2, "a flip of `a` runs the `when` blocks of `a`'s two nodes");
    fx.frame(0.0);
    assert_eq!(fx.rt.debug_laddered(), 0, "a still control runs none");
}

#[test]
fn a_timed_flip_glides_the_boxes_of_the_instance_and_a_box_outside_jumps() {
    let mut fx = Fx::new(&two_switches());
    let a: NodeId = fx.id("a");
    let wide: ArrangementId = fx.arrangement(a, "wide");
    fx.rt.go_in_id(a, wide);
    fx.frame(0.0);
    assert_eq!(fx.rect("a").w, 60.0, "`a`'s own box starts its glide where it showed");
    assert_eq!((fx.rect("b").x, fx.rect("after").x), (90.0, 150.0), "the boxes outside `a` are where the layout puts them at once");
    fx.frames(0.5);
    let midway: f32 = fx.rect("a").w;
    assert!(midway > 62.0 && midway < 88.0, "`a` is {midway} wide half way");
    fx.frames(0.6);
    assert_eq!(fx.rect("a").w, 90.0, "landed");

    // An alignment has no number to slide: the box itself glides.
    let on: ArrangementId = fx.arrangement(a, "on");
    fx.rt.go_in_id(a, on);
    fx.frame(0.0);
    fx.frames(0.5);
    let knob_x: f32 = fx.rect("a/knob").x;
    assert!(knob_x > 2.0 && knob_x < 38.0, "`a/knob` is at {knob_x} half way");
    assert_eq!(fx.rect("b/knob").x, 60.0, "`b`'s knob did not move");
    fx.frames(0.6);
    assert_eq!(fx.rect("a/knob").x, 40.0);
}

#[test]
fn a_flip_with_no_rule_lands_at_once_and_going_back_takes_the_way_in() {
    let src = SWITCH.replace("    transition any -> wide duration=1\n", "") + "View page { Row strip halign=left, valign=top, spacing=0 { Switch a } }\n";
    let mut fx = Fx::new(&src);
    let a: NodeId = fx.id("a");
    let (on, wide): (ArrangementId, ArrangementId) = (fx.arrangement(a, "on"), fx.arrangement(a, "wide"));
    fx.rt.go_in_id(a, wide);
    fx.frame(0.0);
    assert_eq!(fx.rect("a").w, 90.0, "no rule times `wide`: it lands at once");
    fx.rt.go_in_id(a, BASE);
    fx.frame(0.0);
    assert_eq!(fx.rect("a").w, 60.0, "and so does the way back");
    fx.rt.go_in_id(a, on);
    fx.frames(1.2);
    assert_eq!(fx.rect("a/knob").x, 40.0);
    // `any -> on` is the way in; with no rule of its own the way back to
    // no arrangement takes the same time.
    fx.rt.go_in_id(a, BASE);
    fx.frame(0.0);
    fx.frames(0.5);
    let knob_x: f32 = fx.rect("a/knob").x;
    assert!(knob_x > 2.0 && knob_x < 38.0, "`a/knob` is at {knob_x} half way back");
    fx.frames(0.6);
    assert_eq!(fx.rect("a/knob").x, 0.0);
}

#[test]
fn a_node_of_the_body_times_its_own_share_of_a_flip() {
    // The plate takes 2 seconds, the toast jumps, the tile takes the
    // widget's 1 second.
    let src = "widget Plates {
    arrangement open
    transition base -> open duration=1
    when open {
        tile.halign = right
        tile.width = 40
        plate.halign = right
        plate.width = 40
        plate.opacity = 0.5
        tile.opacity = 0.5
        toast.halign = right
    }
    Panel box width=100, height=30 {
        Rectangle plate width=20, height=10, halign=left, valign=top, color=#111111FF {
            transition base -> open duration=2
        }
        Rectangle tile width=20, height=10, halign=left, valign=center, color=#222222FF
        Rectangle toast width=20, height=10, halign=left, valign=bottom, color=#333333FF {
            transition base -> open duration=0
        }
    }
}
View page { Plates t halign=left, valign=top }
";
    let mut fx = Fx::new(src);
    let t: NodeId = fx.id("t");
    let open: ArrangementId = fx.arrangement(t, "open");
    fx.rt.go_in_id(t, open);
    fx.frame(0.0);
    assert_eq!(fx.rect("t/toast").x, 80.0, "the toast's own rule says 0 seconds: it jumps");
    fx.frames(1.1);
    assert_eq!((fx.rect("t/tile").x, fx.rect("t/tile").w), (60.0, 40.0), "the tile landed with the widget's rule");
    let plate: Rect = fx.rect("t/plate");
    assert!(plate.x > 10.0 && plate.x < 55.0, "the plate, on its own 2 seconds, is at {}", plate.x);
    assert!(plate.w > 21.0 && plate.w < 39.0, "and {} wide", plate.w);
    let (plate_id, tile_id): (NodeId, NodeId) = (fx.id("t/plate"), fx.id("t/tile"));
    let plate_opacity: f32 = fx.rt.shown[plate_id].opacity;
    assert!(plate_opacity > 0.55 && plate_opacity < 0.95, "its opacity, a number and no box, is on its own 2 seconds too: {plate_opacity}");
    assert_eq!(fx.rt.shown[tile_id].opacity, 0.5, "the tile's landed with the widget's rule");
    fx.frames(1.0);
    assert_eq!((fx.rect("t/plate").x, fx.rect("t/plate").w), (60.0, 40.0));
}

/// `base` alone in a rule of the widget is the instance with no
/// arrangement on: the rule times every flip away from it.
#[test]
fn base_in_a_rule_of_the_body_is_the_instance_with_no_arrangement_on() {
    let src = SWITCH.replace("    transition any -> on duration=1\n    transition any -> wide duration=1\n", "    transition base -> any duration=1\n") + "View page { Row strip halign=left, valign=top, spacing=0 { Switch a } }\n";
    let mut fx = Fx::new(&src);
    let a: NodeId = fx.id("a");
    let (on, wide): (ArrangementId, ArrangementId) = (fx.arrangement(a, "on"), fx.arrangement(a, "wide"));
    fx.rt.go_in_id(a, on);
    fx.frame(0.0);
    fx.frames(0.5);
    let knob_x: f32 = fx.rect("a/knob").x;
    assert!(knob_x > 2.0 && knob_x < 38.0, "leaving `base` is timed: `a/knob` is at {knob_x} half way");
    fx.frames(0.6);
    fx.rt.go_in_id(a, wide);
    fx.frame(0.0);
    assert_eq!((fx.rect("a/knob").x, fx.rect("a").w), (0.0, 90.0), "`on` to `wide` does not leave `base`: no rule, it lands at once");
}

/// A `when` of the arrangement gone to writes a property an anim left
/// held at its end: the `when` is that property's next writer.
#[test]
fn a_flip_takes_a_property_back_from_an_anim_that_ended_on_it() {
    let src = "widget Lamp {
    arrangement lit
    anim dim { bulb.opacity from=1, to=0, duration=0.1 }
    when lit { bulb.opacity = 0.5 }
    Panel box width=20, height=20 { Rectangle bulb width=20, height=20, color=#FFFFFFFF }
}
View page { Row both halign=left, valign=top { Lamp l1; Lamp l2 } }
";
    let mut fx = Fx::new(src);
    let (l1, bulb1, bulb2): (NodeId, NodeId, NodeId) = (fx.id("l1"), fx.id("l1/bulb"), fx.id("l2/bulb"));
    for anim in ["l1/dim", "l2/dim"] {
        let id = fx.page.intern_lookup(anim);
        fx.rt.play_id(&fx.page, id, gogo_ui::Finish::Nothing);
    }
    fx.frames(0.3);
    assert_eq!((fx.rt.shown[bulb1].opacity, fx.rt.shown[bulb2].opacity), (0.0, 0.0), "both anims ended and hold");
    let lit: ArrangementId = fx.arrangement(l1, "lit");
    fx.rt.go_in_instant_id(l1, lit);
    fx.frame(0.0);
    assert_eq!((fx.rt.shown[bulb1].opacity, fx.rt.shown[bulb2].opacity), (0.5, 0.0), "`l1` shows its `when lit`; `l2`, which did not flip, keeps the anim's end");
}

#[test]
fn a_color_slides_by_the_widgets_rule_and_an_instant_flip_slides_nothing() {
    let mut fx = Fx::new(&two_switches());
    let (a, knob): (NodeId, NodeId) = (fx.id("a"), fx.id("a/knob"));
    let on: ArrangementId = fx.arrangement(a, "on");
    let red_of = |fx: &Fx| match fx.kept.look().prop(&fx.page, knob, gogo_ui::PropKey::Background) {
        Some(gogo_ui::Value::VColor(c)) => c.0[0],
        other => panic!("the knob's plate is {other:?}"),
    };
    fx.rt.go_in_id(a, on);
    fx.frame(0.0);
    fx.frames(0.5);
    let red: u8 = red_of(&fx);
    assert!(red > 20 && red < 235, "half way from red to green the knob holds {red} of red");
    fx.frames(0.6);
    assert_eq!(red_of(&fx), 0);

    fx.rt.go_in_instant_id(a, BASE);
    fx.frame(0.0);
    assert_eq!((red_of(&fx), fx.rect("a/knob").x), (255, 0.0), "an instant flip lands in its frame");
}

#[test]
fn a_box_gliding_from_a_flip_of_the_page_goes_on_while_a_control_flips() {
    let src = format!("arrangement shifted\ntransition base -> shifted duration=1\nwhen shifted {{ mover.halign = right }}\n{SWITCH}View page {{\n    Rectangle mover width=20, height=20, halign=left, valign=bottom, color=#FFFFFFFF\n    Switch a halign=left, valign=top\n}}\n");
    let mut fx = Fx::new(&src);
    fx.rt.go_id(ArrangementId::new(0));
    fx.frame(0.0);
    fx.frames(0.3);
    let before: f32 = fx.rect("mover").x;
    assert!(before > 10.0 && before < 300.0, "the page's box is on its way at {before}");
    let a: NodeId = fx.id("a");
    let on: ArrangementId = fx.arrangement(a, "on");
    fx.rt.go_in_id(a, on);
    fx.frame(FRAME);
    let after: f32 = fx.rect("mover").x;
    assert!(after > before && after < 300.0, "one frame on it is at {after}: still gliding, not at its end (380)");
    fx.frames(1.2);
    assert_eq!((fx.rect("mover").x, fx.rect("a/knob").x), (380.0, 40.0));
}

#[test]
fn go_and_toggle_in_a_body_set_the_instance_the_tapped_node_is_in() {
    let mut fx = Fx::new(&two_switches());
    let (a, b): (NodeId, NodeId) = (fx.id("a"), fx.id("b"));
    let on: ArrangementId = fx.arrangement(b, "on");
    // The right half of `b`'s well, clear of the knob at its left.
    let well: NodeId = fx.id("b/well");
    fx.tap_at(well, 50.0, 30.0);
    assert_eq!((fx.rt.arrangement_in_id(a), fx.rt.arrangement_in_id(b)), (BASE, on), "`toggle(on)` in `b`'s body set `b`");
    fx.tap_at(well, 10.0, 30.0);
    assert_eq!(fx.rt.arrangement_in_id(b), BASE, "a second tap toggles it off");
    fx.set("b", "wide");
    fx.set("a", "on");
    let knob: NodeId = fx.id("b/knob");
    fx.tap_at(knob, 10.0, 10.0);
    assert_eq!((fx.rt.arrangement_in_id(a), fx.rt.arrangement_in_id(b)), (on, BASE), "`go(base)` in `b`'s body set `b` and left `a`");
}

#[test]
fn an_arrangement_composes_with_a_screen_word() {
    let src = "widget Tile {
    arrangement open
    when open { mark.width = 20 }
    when open screen.landscape { mark.width = 50 }
    Panel box width=100, height=30 { Rectangle mark width=10, height=10, halign=left, valign=top, color=#FFFFFFFF }
}
View page { Tile t halign=left, valign=top }
";
    let mut fx = Fx::new(src);
    fx.rt.set_screen(&fx.page, Orientation::Landscape, ScreenClass::LandscapeSmall);
    fx.frame(0.0);
    assert_eq!(fx.rect("t/mark").w, 10.0, "the screen word alone holds nothing");
    fx.set("t", "open");
    assert_eq!(fx.rect("t/mark").w, 50.0, "both hold: the block with two conditions wins");
    fx.rt.set_screen(&fx.page, Orientation::Portrait, ScreenClass::PortraitMedium);
    fx.frame(0.0);
    assert_eq!(fx.rect("t/mark").w, 20.0, "the screen turned: the arrangement's own block");
}

#[test]
fn each_row_of_a_repeat_is_in_an_arrangement_of_its_own() {
    let src = format!("{SWITCH}View page {{\n    Column list halign=left, valign=top, spacing=0 {{\n        repeat rows capacity=2 {{ Switch }}\n    }}\n}}\n");
    let mut fx = Fx::new(&src);
    let repeat: NodeId = fx.id("rows");
    fx.rt.set_count(&mut fx.page, repeat, 3);
    fx.frame(0.0);
    let rows: Vec<NodeId> = fx.rt.items(&fx.page, repeat);
    assert_eq!(rows.len(), 3);
    let template: NodeId = fx.page.repeats[0].template;
    // The knob of a row: the template's knob, moved by the roots' distance.
    let template_knob: NodeId = fx.page.descendants(template).find(|n| fx.page.str(fx.page.nodes[*n].name).ends_with("/knob")).expect("the template's knob");
    let knob_of = |row: NodeId| template_knob.rebase(template, row);
    let on: ArrangementId = fx.arrangement(rows[1], "on");
    fx.rt.go_in_instant_id(rows[1], on);
    fx.frame(0.0);
    let knob_x: Vec<f32> = rows.iter().map(|row| fx.rect_of(knob_of(*row)).x).collect();
    assert_eq!(knob_x, [0.0, 40.0, 0.0], "the middle row is on, its neighbours are not");
    // A tap in the last row's well: the handler is one for every row, and
    // sets the row it was tapped in.
    let template_well: NodeId = fx.page.descendants(template).find(|n| fx.page.str(fx.page.nodes[*n].name).ends_with("/well")).expect("the template's well");
    fx.tap_at(template_well.rebase(template, rows[2]), 50.0, 30.0);
    let now: Vec<ArrangementId> = rows.iter().map(|row| fx.rt.arrangement_in_id(*row)).collect();
    assert_eq!(now, [BASE, on, on]);
}

#[test]
fn a_seat_inside_the_body_takes_the_node_of_that_instance() {
    let src = "widget Dock {
    arrangement parked
    when parked { car.seat = bay }
    Row box height=20, spacing=0 {
        Layers road width=40, height=20 { Rectangle car width=10, height=10, halign=left, valign=top, color=#FFFFFFFF }
        Layers lot width=40, height=20 { seat bay }
    }
}
View page { Column both halign=left, valign=top, spacing=0 { Dock d1; Dock d2 } }
";
    let mut fx = Fx::new(src);
    fx.set("d2", "parked");
    assert_eq!((fx.rect("d1/car").x, fx.rect("d2/car").x), (0.0, 40.0), "`d2`'s car is in `d2`'s bay, `d1`'s at home");
    assert_eq!(fx.rect("d2/car").y, 20.0);
}

/// A `control` poses its own body on its own arrangement as a `widget`
/// does: the block beside the body names the body's nodes and may seat
/// one.
#[test]
fn a_control_poses_its_body_on_its_own_arrangement() {
    let src = "control Dock {
    state busy
    arrangement parked
    when parked { car.seat = bay }
    Row box height=20, spacing=0 {
        Layers road width=40, height=20 { Rectangle car width=10, height=10, halign=left, valign=top, color=#FFFFFFFF }
        Layers lot width=40, height=20 { seat bay }
    }
}
View page { Column both halign=left, valign=top, spacing=0 { Dock d1; Dock d2 } }
";
    let mut fx = Fx::new(src);
    fx.set("d1", "parked");
    assert_eq!((fx.rect("d1/car").x, fx.rect("d2/car").x), (40.0, 0.0), "`d1`'s car is in `d1`'s bay");
}

#[test]
fn a_rule_of_the_body_plays_its_anim_for_the_instance_that_flipped() {
    let src = "widget Lamp {
    arrangement lit
    anim glow { bulb.opacity from=0, to=1, duration=1 }
    transition base -> lit anim=glow
    when lit { bulb.color = #FFFF00FF }
    Panel box width=20, height=20 { Rectangle bulb width=20, height=20, color=#333333FF }
}
View page { Row both halign=left, valign=top { Lamp l1; Lamp l2 } }
";
    let mut fx = Fx::new(src);
    let l2: NodeId = fx.id("l2");
    let lit: ArrangementId = fx.arrangement(l2, "lit");
    fx.rt.go_in_id(l2, lit);
    fx.frame(0.0);
    let (glow1, glow2) = (fx.page.intern_lookup("l1/glow"), fx.page.intern_lookup("l2/glow"));
    assert!(fx.rt.is_playing_id(glow2), "`l2`'s own copy of the anim plays");
    assert!(!fx.rt.is_playing_id(glow1), "`l1`'s does not");
}

#[test]
fn the_build_reads_a_name_where_it_is_written() {
    // The page and the widget both declare `open`. A line of the widget's
    // body means the widget's; the instance's own line and what the page
    // puts into the widget's slot are the page's lines and mean the
    // page's.
    let src = "arrangement open
widget Card {
    arrangement open
    when open { face.color = #00FF00FF }
    Button box width=100, height=40, on:activate=toggle(open) {
        Rectangle face width=fill, height=fill, color=#FF0000FF
        slot
    }
}
View page {
    Card card halign=left, valign=top, on:press=go(open) {
        Rectangle filling width=10, height=10, color=#FFFFFFFF { when open color=#0000FFFF }
    }
}
";
    let c = compile(src, &|_| None);
    assert!(!has_errors(&c.diags), "{:#?}", c.diags);
    let page: &Page = &c.page;
    let card: NodeId = page.find("card").unwrap();
    let actions: Vec<Action> = page.handlers[page.nodes[card].handlers.0 as usize..(page.nodes[card].handlers.0 + page.nodes[card].handlers.1) as usize].iter().map(|h| h.action).collect();
    assert!(actions.contains(&Action::ToggleInControl(ArrangementId::new(0))), "the body's `toggle(open)` is the widget's: {actions:?}");
    assert!(actions.contains(&Action::Go(ArrangementId::new(0))), "the instance line's `go(open)` is the page's: {actions:?}");
    let when_of = |name: &str| {
        let node = &page.nodes[page.find(name).unwrap()];
        page.whens[node.whens.0 as usize]
    };
    assert_eq!((when_of("card/face").state, when_of("card/face").of), (State::ControlArrangement(ArrangementId::new(0)), card));
    assert_eq!(when_of("filling").state, State::Arrangement(ArrangementId::new(0)), "the fill is the page's line");
}

/// A widget that declares no such name follows the page's arrangement,
/// as before: every instance flips with the page (a chip that shows in
/// the page's names-only view).
#[test]
fn a_body_that_names_no_arrangement_of_its_own_follows_the_pages() {
    let src = "arrangement names
widget Chip {
    arrangement lit
    when lit { dot.width = 30 }
    Button box width=40, height=20, on:activate=go(names) {
        when names background=#FF0000FF
        Rectangle dot width=10, height=10, halign=left, valign=top, color=#FFFFFFFF { when names visibility=gone }
    }
}
View page { Row both halign=left, valign=top, spacing=0 { Chip c1; Chip c2 } }
";
    let mut fx = Fx::new(src);
    let c1: NodeId = fx.id("c1");
    let actions: Vec<Action> = fx.page.handlers[fx.page.nodes[c1].handlers.0 as usize..(fx.page.nodes[c1].handlers.0 + fx.page.nodes[c1].handlers.1) as usize].iter().map(|h| h.action).collect();
    assert_eq!(actions, [Action::Go(ArrangementId::new(0))], "`go(names)` in the body is the page's: `Chip` declares no `names`");
    fx.set("c1", "lit");
    assert_eq!((fx.rect("c1/dot").w, fx.rect("c2/dot").w), (30.0, 10.0));
    fx.rt.go_instant_id(ArrangementId::new(0));
    fx.frame(0.0);
    assert_eq!((fx.rect("c1/dot").w, fx.rect("c2/dot").w), (0.0, 0.0), "the page's `names` hides the dot of every chip");
    assert_eq!(fx.rt.arrangement_in_id(c1), fx.arrangement(c1, "lit"), "and leaves each chip in its own arrangement");
}

#[test]
fn a_widget_inside_a_widget_keeps_its_own_arrangements() {
    let src = "widget Inner {
    arrangement lit
    when lit { dot.width = 30 }
    Panel box width=40, height=10 { Rectangle dot width=10, height=10, halign=left, valign=top, color=#FFFFFFFF }
}
widget Outer {
    arrangement tall
    when tall { frame.height = 50 }
    Panel frame width=40, height=20 { Inner inner halign=left, valign=top }
}
View page { Row both halign=left, valign=top, spacing=0 { Outer o1; Outer o2 } }
";
    let mut fx = Fx::new(src);
    fx.set("o1", "tall");
    fx.set("o2/inner", "lit");
    assert_eq!((fx.rect("o1").h, fx.rect("o1/inner/dot").w), (50.0, 10.0));
    assert_eq!((fx.rect("o2").h, fx.rect("o2/inner/dot").w), (20.0, 30.0));
    let (o1, inner): (NodeId, NodeId) = (fx.id("o1"), fx.id("o2/inner"));
    assert_eq!(fx.page.control_arrangement(o1, "lit"), None, "`lit` is no arrangement of `Outer`");
    assert_eq!(fx.page.control_arrangement(inner, "tall"), None, "`tall` is none of `Inner`");
}

#[test]
fn a_place_the_widget_does_not_declare_is_refused() {
    let mut fx = Fx::new(&two_switches());
    let (a, strip): (NodeId, NodeId) = (fx.id("a"), fx.id("strip"));
    fx.rt.go_in_id(a, ArrangementId::new(2));
    fx.rt.go_in_id(strip, ArrangementId::new(0));
    fx.frame(0.0);
    assert_eq!((fx.rt.arrangement_in_id(a), fx.rt.arrangement_in_id(strip)), (BASE, BASE), "`Switch` declares two, and `strip` is no instance");
    assert!(!fx.rt.animating());
}

#[test]
fn the_build_refuses_what_could_mean_two_things() {
    let page = "View page { Card card }\n";
    let card = |lines: &str| format!("widget Card {{\n{lines}    Panel box width=10, height=10 {{ Rectangle face width=5, height=5, color=#FFFFFFFF }}\n}}\n");

    // A `when` beside the body of a widget with no host answers the
    // screen or the widget's own arrangements, as before.
    let e = errors(&format!("arrangement wide\n{}{page}", card("    when wide { face.width = 9 }\n")));
    assert!(e.contains("G5001"), "a widget with no arrangement of its own: {e}");
    let e = errors(&format!("arrangement wide\n{}{page}", card("    arrangement open\n    when wide { face.width = 9 }\n")));
    assert!(e.contains("G5001"), "{e}");

    // A widget's arrangement, named in the page's own lines.
    let e = errors(&format!("{}View page {{ Card card }}\nwhen open {{ card.width = 9 }}\n", card("    arrangement open\n")));
    assert!(e.contains("G0036") && e.contains("`open` is an arrangement of `Card`"), "{e}");
    let e = errors(&format!("{}View page {{ Card card {{ when open background=#FF0000FF }} }}\n", card("    arrangement open\n")));
    assert!(e.contains("G0036") && e.contains("`open` is an arrangement of `Card`"), "an instance's own `when` is the page's line: {e}");

    // The declaration.
    assert!(errors(&format!("{}{page}", card("    arrangement open\n    arrangement open\n"))).contains("G0033"));
    assert!(errors(&format!("{}{page}", card("    arrangement held\n"))).contains("G0032"));
    assert!(errors(&format!("{}{page}", card("    arrangement base\n"))).contains("G0032"));
    let e = errors(&format!("control Fader {{\n    state grabbed\n    arrangement grabbed\n    Panel box width=10, height=10\n}}\nView page {{ Fader f }}\n"));
    assert!(e.contains("G0032") && e.contains("a state and as an arrangement"), "{e}");

    // The rules.
    let e = errors(&format!("{}{page}", card("    arrangement open\n    transition open -> held duration=1\n")));
    assert!(e.contains("G5001"), "a widget with no host has no `held`: {e}");
    let e = errors(&format!("widget Card for=Button {{\n    arrangement open\n    transition open -> held duration=1\n    Rectangle face width=5, height=5, color=#FFFFFFFF\n}}\nView page {{ Button b width=10, height=10 {{ Card card }} }}\n"));
    assert!(e.contains("G0034") && e.contains("times nothing"), "{e}");
    let e = errors(&format!("{}{page}", card("    arrangement open\n    when open screen.landscape { face.width = 9 }\n    transition base -> open duration=1\n    Panel extra\n")));
    assert!(e.contains("G0028"), "two roots are still refused: {e}");
    let e = errors(&format!("widget Card {{\n    arrangement open\n    Button box width=10, height=10, on:activate=toggle(base)\n}}\n{page}"));
    assert!(e.contains("G0050"), "{e}");
}

#[test]
fn the_build_writes_one_enum_per_widget_and_ties_it_to_the_widgets_names() {
    let c = compile(&two_switches(), &|_| None);
    assert!(!has_errors(&c.diags), "{:#?}", c.diags);
    let names: String = gogo_ui::emit::emit_widget_names(&[("m", &c.page)]).unwrap();
    assert!(names.contains("pub enum SwitchArrangement {\n    On = 0,\n    Wide = 1,\n    Base = 4294967295,\n}"), "{names}");
    assert!(names.contains("impl gogo_ui::ControlArrangementName for SwitchArrangement {"), "{names}");
    assert!(names.contains("            1 => SwitchArrangement::Wide,\n            _ => SwitchArrangement::Base,"), "{names}");
    assert!(names.contains("impl gogo_ui::HasArrangements for SwitchNames {\n    type Arrangement = SwitchArrangement;"), "{names}");
    let code: String = gogo_ui::emit::emit_rust(&c.page, "m").unwrap();
    let (a, b): (NodeId, NodeId) = (c.page.find("a").unwrap(), c.page.find("b").unwrap());
    assert!(code.contains(&format!("control_arrangements: &[ControlArrangements {{ root: {a:?}, first: 0, count: 2 }}, ControlArrangements {{ root: {b:?}, first: 0, count: 2 }}],")), "{code}");
    assert!(code.contains("control_arrangement_names: &["), "{code}");
    // Two names that spell one variant, and a name that spells `Base`.
    let clash = |lines: &str| {
        let c = compile(&format!("widget Card {{\n{lines}    Panel box width=10, height=10\n}}\nView page {{ Card card }}\n"), &|_| None);
        assert!(!has_errors(&c.diags), "{:#?}", c.diags);
        gogo_ui::emit::emit_widget_names(&[("m", &c.page)]).unwrap_err()
    };
    assert!(clash("    arrangement fadeIn\n    arrangement fade_in\n").contains("both become `CardArrangement::FadeIn`"));
    assert!(clash("    arrangement Base\n").contains("`CardArrangement::Base`"));
}

#[test]
fn a_page_with_no_such_widget_keeps_no_table() {
    let c = compile("widget Plain { Panel box width=10, height=10 }\nView page { Plain p }\n", &|_| None);
    assert!(!has_errors(&c.diags), "{:#?}", c.diags);
    assert!(c.page.control_arrangements.is_empty() && c.page.control_arrangement_names.is_empty());
}

#[test]
fn a_reload_keeps_each_instance_in_its_arrangement() {
    let mut fx = Fx::new(&two_switches());
    fx.set("b", "wide");
    // The same file with a wider second box.
    let c = compile(&two_switches().replace("extra width=30", "extra width=50"), &|_| None);
    assert!(!has_errors(&c.diags), "{:#?}", c.diags);
    let mut new_page: Page = c.page;
    let carried: Runtime = fx.rt.carried_to(&fx.page, &mut new_page).unwrap_or_else(|why| panic!("{why:?}"));
    fx.page = new_page;
    fx.rt = carried;
    fx.kept = KeptFrame::default();
    fx.frame(0.0);
    let (a, b): (NodeId, NodeId) = (fx.id("a"), fx.id("b"));
    assert_eq!((fx.rt.arrangement_in_id(a), fx.rt.arrangement_in_id(b)), (BASE, fx.arrangement(b, "wide")));
    assert_eq!(fx.rect("b/extra").w, 50.0, "`b` shows the new file in the arrangement it was in");
}
