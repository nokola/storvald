//! A widget's own anims by a typed name: a `widget` or `control`
//! declares `anim` blocks beside its body, each instance has its own copy
//! of them, and code plays the copy of one instance
//! (`Runtime::play_in`).
//!
//! Needed because the name of an instance's copy (`l1/dim`) was a string
//! code had to put together: a misspelled one played nothing and said
//! nothing.
use gogo_ui::layout::{FixedAdvance, Rect};
use gogo_ui::{ControlAnimName, Draw, Finish, HasAnims, KeptFrame, NodeId, Page, Runtime, StrId, compile, has_errors};

const VIEW: Rect = Rect { x: 0.0, y: 0.0, w: 400.0, h: 800.0 };
/// One frame of a 60 Hz screen, in seconds.
const FRAME: f32 = 1.0 / 60.0;

/// A lamp with two anims: `dim` fades its bulb out, `glow` fades it in.
const LAMP: &str = "widget Lamp {
    anim dim { bulb.opacity from=1, to=0, duration=0.1 }
    anim glow { bulb.opacity from=0, to=1, duration=0.1 }
    Panel box width=20, height=20 { Rectangle bulb width=20, height=20, color=#FFFFFFFF }
}
";

/// What a build writes for `Lamp`: the enum of its anims, and the tie
/// between the enum and the names of one instance.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u32)]
enum LampAnim {
    Dim = 0,
    Glow = 1,
}

impl ControlAnimName for LampAnim {
    fn place(self) -> u32 {
        self as u32
    }
}

#[derive(Clone, Copy)]
struct LampNames {
    root: NodeId,
}

impl HasAnims for LampNames {
    type Anim = LampAnim;

    fn root(&self) -> NodeId {
        self.root
    }
}

/// One page run the way a host runs it.
struct Fx {
    page: Page,
    rt: Runtime,
    kept: KeptFrame,
    draws: Vec<Draw>,
}

impl Fx {
    fn new(src: &str) -> Fx {
        let c = compile(src, &|_| None);
        assert!(!has_errors(&c.diags), "{src}\n{:#?}", c.diags);
        let rt = Runtime::new(&c.page);
        let mut fx = Fx { page: c.page, rt, kept: KeptFrame::default(), draws: Vec::new() };
        fx.frame(0.0);
        fx
    }

    fn frame(&mut self, dt: f32) {
        self.rt.tick(&mut self.page, dt);
        self.kept.frame_retained(&mut self.rt, &self.page, VIEW, &FixedAdvance, &mut self.draws, None);
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

    fn lamp(&self, name: &str) -> LampNames {
        LampNames { root: self.id(name) }
    }

    fn opacity(&self, node: NodeId) -> f32 {
        self.rt.shown[node].opacity
    }
}

fn two_lamps() -> String {
    format!("{LAMP}View page {{ Row both halign=left, valign=top {{ Lamp l1; Lamp l2 }} }}\n")
}

#[test]
fn each_instance_plays_its_own_copy() {
    let mut fx = Fx::new(&two_lamps());
    let (l1, l2): (LampNames, LampNames) = (fx.lamp("l1"), fx.lamp("l2"));
    let (bulb1, bulb2): (NodeId, NodeId) = (fx.id("l1/bulb"), fx.id("l2/bulb"));
    assert_eq!(fx.page.anim_in(&l1, LampAnim::Dim), Some(fx.page.intern_lookup("l1/dim")));
    assert_eq!(fx.page.anim_in(&l2, LampAnim::Glow), Some(fx.page.intern_lookup("l2/glow")));

    fx.rt.play_in(&fx.page, &l1, LampAnim::Dim, Finish::Nothing);
    assert!(fx.rt.is_playing_in(&fx.page, &l1, LampAnim::Dim), "`l1` plays its `dim`");
    assert!(!fx.rt.is_playing_in(&fx.page, &l2, LampAnim::Dim), "`l2` does not");
    assert!(!fx.rt.is_playing_in(&fx.page, &l1, LampAnim::Glow), "and `l1` plays no `glow`");
    fx.frames(0.3);
    assert_eq!((fx.opacity(bulb1), fx.opacity(bulb2)), (0.0, 1.0), "the bulb of `l1` went out, the bulb of `l2` did not move");
    let finished: Vec<StrId> = fx.rt.take_finished();
    assert_eq!(finished, [fx.page.anim_in(&l1, LampAnim::Dim).unwrap()], "the anim that ended is the copy of `l1`");

    // The other anim of the widget, on the other instance.
    fx.rt.play_in(&fx.page, &l2, LampAnim::Dim, Finish::Nothing);
    fx.frames(0.3);
    fx.rt.play_in(&fx.page, &l2, LampAnim::Glow, Finish::Nothing);
    fx.frames(0.3);
    assert_eq!((fx.opacity(bulb1), fx.opacity(bulb2)), (0.0, 1.0), "`l2` glows again, `l1` stays out");
}

#[test]
fn a_stop_puts_back_the_instance_that_played() {
    let mut fx = Fx::new(&two_lamps());
    let (l1, l2): (LampNames, LampNames) = (fx.lamp("l1"), fx.lamp("l2"));
    let (bulb1, bulb2): (NodeId, NodeId) = (fx.id("l1/bulb"), fx.id("l2/bulb"));
    fx.rt.play_in(&fx.page, &l1, LampAnim::Dim, Finish::Nothing);
    fx.rt.play_in(&fx.page, &l2, LampAnim::Dim, Finish::Nothing);
    fx.frames(0.05);
    assert!(fx.opacity(bulb1) < 0.9 && fx.opacity(bulb2) < 0.9, "both are half way");
    fx.rt.stop_in(&fx.page, &l1, LampAnim::Dim, true);
    fx.frame(0.0);
    assert!(!fx.rt.is_playing_in(&fx.page, &l1, LampAnim::Dim) && fx.rt.is_playing_in(&fx.page, &l2, LampAnim::Dim));
    assert_eq!(fx.opacity(bulb1), 1.0, "the bulb of `l1` is back where the play found it");
    assert!(fx.opacity(bulb2) < 0.9, "the bulb of `l2` plays on");
}

#[test]
fn a_row_of_a_repeat_plays_its_own_copy() {
    let src = format!("{LAMP}View page {{\n    Column list halign=left, valign=top, spacing=0 {{\n        repeat rows capacity=2 {{ Lamp }}\n    }}\n}}\n");
    let mut fx = Fx::new(&src);
    let repeat: NodeId = fx.id("rows");
    fx.rt.set_count(&mut fx.page, repeat, 3);
    fx.frame(0.0);
    let rows: Vec<NodeId> = fx.rt.items(&fx.page, repeat);
    assert_eq!(rows.len(), 3);
    let template: NodeId = fx.page.repeats[0].template;
    // The bulb of a row: the template's bulb, moved by the roots' distance.
    let template_bulb: NodeId = fx.page.descendants(template).find(|n| fx.page.str(fx.page.nodes[*n].name).ends_with("/bulb")).expect("the template's bulb");
    let bulbs: Vec<NodeId> = rows.iter().map(|row| template_bulb.rebase(template, *row)).collect();
    let middle = LampNames { root: rows[1] };
    fx.rt.play_in(&fx.page, &middle, LampAnim::Dim, Finish::Nothing);
    assert!(fx.rt.is_playing_in(&fx.page, &middle, LampAnim::Dim), "the middle row has a `dim` of its own to play");
    fx.frames(0.3);
    let shown: Vec<f32> = bulbs.iter().map(|bulb| fx.opacity(*bulb)).collect();
    assert_eq!(shown, [1.0, 0.0, 1.0], "the middle row's bulb went out, its neighbours' did not move");
    assert_eq!(fx.opacity(template_bulb), 1.0, "nor did the template's");
    // Every row names copies of its own.
    let names: Vec<StrId> = rows.iter().map(|row| fx.page.anim_in(&LampNames { root: *row }, LampAnim::Glow).expect("a row's `glow`")).collect();
    assert!(names[0] != names[1] && names[1] != names[2] && names[0] != names[2]);
}

#[test]
fn a_widget_inside_a_widget_plays_its_own_copy() {
    let src = format!("{LAMP}widget Post {{\n    Column box spacing=0 {{ Lamp top; Lamp bottom }}\n}}\nView page {{ Row both halign=left, valign=top {{ Post p1; Post p2 }} }}\n");
    let mut fx = Fx::new(&src);
    let lamp: LampNames = fx.lamp("p2/top");
    assert_eq!(fx.page.anim_in(&lamp, LampAnim::Dim), Some(fx.page.intern_lookup("p2/top/dim")));
    fx.rt.play_in(&fx.page, &lamp, LampAnim::Dim, Finish::Nothing);
    fx.frames(0.3);
    let shown: Vec<f32> = ["p1/top/bulb", "p1/bottom/bulb", "p2/top/bulb", "p2/bottom/bulb"].iter().map(|name| fx.opacity(fx.id(name))).collect();
    assert_eq!(shown, [1.0, 1.0, 0.0, 1.0]);
}

#[test]
fn a_node_that_is_no_such_instance_plays_nothing() {
    let mut fx = Fx::new(&two_lamps());
    // `both` is the row the lamps sit in: no instance of `Lamp`.
    let no_lamp: LampNames = fx.lamp("both");
    assert_eq!(fx.page.anim_in(&no_lamp, LampAnim::Dim), None);
    fx.rt.play_in(&fx.page, &no_lamp, LampAnim::Dim, Finish::Nothing);
    assert!(!fx.rt.is_playing_in(&fx.page, &no_lamp, LampAnim::Dim));
    fx.rt.stop_in(&fx.page, &no_lamp, LampAnim::Dim, true);
    fx.frames(0.3);
    assert_eq!((fx.opacity(fx.id("l1/bulb")), fx.opacity(fx.id("l2/bulb"))), (1.0, 1.0));

    // A widget with one anim, asked for a second place: the next
    // instance's first anim sits there in the table.
    let src = "widget Lamp {\n    anim dim { bulb.opacity from=1, to=0, duration=0.1 }\n    Panel box width=20, height=20 { Rectangle bulb width=20, height=20, color=#FFFFFFFF }\n}\nView page { Row both halign=left, valign=top { Lamp l1; Lamp l2 } }\n";
    let mut fx = Fx::new(src);
    let l1: LampNames = fx.lamp("l1");
    assert_eq!(fx.page.anim_in(&l1, LampAnim::Glow), None, "`l1` has one anim");
    fx.rt.play_in(&fx.page, &l1, LampAnim::Glow, Finish::Nothing);
    fx.frames(0.3);
    assert_eq!(fx.opacity(fx.id("l2/bulb")), 1.0, "the `dim` of `l2` did not play");
}

#[test]
fn the_build_writes_one_enum_per_widget_and_ties_it_to_the_widgets_names() {
    let c = compile(&two_lamps(), &|_| None);
    assert!(!has_errors(&c.diags), "{:#?}", c.diags);
    let names: String = gogo_ui::emit::emit_widget_names(&[("m", &c.page)]).unwrap();
    assert!(names.contains("pub enum LampAnim {\n    Dim = 0,\n    Glow = 1,\n}"), "{names}");
    assert!(names.contains("impl gogo_ui::ControlAnimName for LampAnim {\n    fn place(self) -> u32 {\n        self as u32\n    }\n}"), "{names}");
    assert!(names.contains("impl gogo_ui::HasAnims for LampNames {\n    type Anim = LampAnim;\n\n    fn root(&self) -> NodeId {\n        self.root\n    }\n}"), "{names}");
    assert!(!names.contains("HasArrangements"), "a widget with no arrangement gets no arrangement enum: {names}");
    let code: String = gogo_ui::emit::emit_rust(&c.page, "m").unwrap();
    let (l1, l2): (NodeId, NodeId) = (c.page.find("l1").unwrap(), c.page.find("l2").unwrap());
    assert!(code.contains(&format!("control_anims: &[ControlAnims {{ root: {l1:?}, first: 0, count: 2 }}, ControlAnims {{ root: {l2:?}, first: 2, count: 2 }}],")), "{code}");
    let copies: Vec<StrId> = ["l1/dim", "l1/glow", "l2/dim", "l2/glow"].iter().map(|name| c.page.intern_lookup(name)).collect();
    assert!(code.contains(&format!("control_anim_names: &{copies:?},")), "{code}");
    // The page's own `Anim` lists no anim of a widget.
    assert!(!code.contains("pub enum Anim"), "{code}");

    // Two names that spell one variant.
    let c = compile("widget Card {\n    anim fadeIn { box.opacity from=0, to=1, duration=0.1 }\n    anim fade_in { box.opacity from=0, to=1, duration=0.1 }\n    Panel box width=10, height=10\n}\nView page { Card card }\n", &|_| None);
    assert!(!has_errors(&c.diags), "{:#?}", c.diags);
    let clash: String = gogo_ui::emit::emit_widget_names(&[("m", &c.page)]).unwrap_err();
    assert!(clash.contains("anims `fadeIn` and `fade_in` both become `CardAnim::FadeIn`"), "{clash}");
}

#[test]
fn a_page_with_no_such_widget_keeps_no_table() {
    let c = compile("widget Plain { Panel box width=10, height=10 }\nanim appear { p.opacity from=0, to=1, duration=0.1 }\nView page { Plain p }\n", &|_| None);
    assert!(!has_errors(&c.diags), "{:#?}", c.diags);
    assert!(c.page.control_anims.is_empty() && c.page.control_anim_names.is_empty(), "the page's own anim is no widget's");
    let names: String = gogo_ui::emit::emit_widget_names(&[("m", &c.page)]).unwrap();
    assert!(!names.contains("Anim"), "{names}");
}
