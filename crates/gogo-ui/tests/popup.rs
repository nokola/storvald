//! `Popup`: what a control opens beside itself.
//!
//! Written inside the node it belongs to; drawn above the whole page,
//! sized by what it holds, placed against that node's rectangle on
//! `side=` and kept inside the page. It shows and hides with
//! `visibility=`, and a back request hides the topmost one.
use gogo_ui::draw::Draw;
use gogo_ui::input::{EventKind, EventList, Input, Phase, Touch};
use gogo_ui::layout::{FixedAdvance, Rect};
use gogo_ui::page::{NodeId, PropKey, Value};
use gogo_ui::runtime::{Runtime, SELECTED};
use gogo_ui::{Color_Straight_Srgb_4xU8_RGBA, KeptFrame, Page, compile, hit};

const W: f32 = 320.0;
const H: f32 = 680.0;

/// A bar at the bottom with a Check at 200,600 60x50 that opens a menu
/// above itself, and a red cover over the top of the page, written
/// after the bar: it paints over everything the bar holds.
const MENU_PAGE: &str = "View v width=fill, height=fill {
    Layers bar halign=left, valign=top, width=320, height=60, margin_top=600 {
        Check more halign=left, valign=top, width=60, height=50, margin_left=200, on:change=moreChanged {
            Rectangle face width=fill, height=fill, color=#202020FF
            Popup menu side=top, visibility=gone {
                Panel plate width=70, height=300, background=#FFFFFFFF, interactive=true {
                    Button pick halign=left, valign=top, width=70, height=40, on:press=picked
                }
            }
            when selected menu.visibility = visible
        }
    }
    Rectangle cover halign=left, valign=top, width=320, height=500, color=#FF0000FF
}";

/// Codes of every diagnostic `src` gets.
fn codes(src: &str) -> Vec<String> {
    compile(src, &|_| None).diags.iter().map(|d| d.code.clone()).collect()
}

/// The message of the first diagnostic of `src` with `code`.
fn message(src: &str, code: &str) -> String {
    let c = compile(src, &|_| None);
    c.diags.iter().find(|d| d.code == code).unwrap_or_else(|| panic!("no {code} in {:#?}", c.diags)).message.clone()
}

/// A page, its runtime, and the frame road a host runs.
struct Fx {
    page: Page,
    rt: Runtime,
    input: Input,
    kept: KeptFrame,
    draws: Vec<Draw>,
}

impl Fx {
    fn new(src: &str) -> Fx {
        let c = compile(src, &|_| None);
        assert!(!gogo_ui::has_errors(&c.diags), "{:#?}", c.diags);
        let rt = Runtime::new(&c.page);
        let mut fx = Fx { page: c.page, rt, input: Input::default(), kept: KeptFrame::default(), draws: Vec::new() };
        fx.frame();
        fx
    }

    /// One frame of the 320 x 680 window.
    fn frame(&mut self) {
        self.rt.tick(&mut self.page, 0.0);
        self.kept.frame_retained(&mut self.rt, &self.page, Rect::new(0.0, 0.0, W, H), &FixedAdvance, &mut self.draws, None);
    }

    fn id(&self, name: &str) -> NodeId {
        self.page.find(name).unwrap_or_else(|| panic!("no node `{name}`"))
    }

    fn rect(&self, name: &str) -> Rect {
        self.kept.rects()[self.id(name).index()]
    }

    fn shows(&self, name: &str) -> bool {
        !self.kept.look().gone(&self.page, self.id(name))
    }

    /// The control's `selected`, written as a tap would leave it.
    fn select(&mut self, name: &str, is_selected: bool) {
        let node: NodeId = self.id(name);
        self.rt.set_state(node, SELECTED, is_selected);
        self.frame();
    }

    /// A finger down and up at one place, a frame after each.
    fn tap(&mut self, x: f32, y: f32) -> EventList {
        let mut events: EventList = self.input.touch(&self.page, &mut self.rt, self.kept.frame(), &FixedAdvance, Touch { phase: Phase::Down, x, y, t: 0.0, id: 1, ..Touch::PLAIN });
        self.frame();
        events.extend(self.input.touch(&self.page, &mut self.rt, self.kept.frame(), &FixedAdvance, Touch { phase: Phase::Up, x, y, t: 0.05, id: 1, ..Touch::PLAIN }));
        self.frame();
        events
    }

    /// Place in the draw list of the first rectangle drawn in `color`.
    fn drawn_at(&self, color: [u8; 4]) -> Option<usize> {
        self.draws.iter().position(|draw| matches!(draw, Draw::Quad { color: c, .. } if *c == Color_Straight_Srgb_4xU8_RGBA(color)))
    }
}

const WHITE: [u8; 4] = [0xFF, 0xFF, 0xFF, 0xFF];
const RED: [u8; 4] = [0xFF, 0, 0, 0xFF];

// ---------------------------------------------------------------- the file

/// A Popup says the side of its parent it opens on; there is no default,
/// so nobody gets a menu over the wrong side by leaving a word out.
#[test]
fn a_popup_says_the_side_it_opens_on() {
    assert!(codes("View v { Button b { Popup p side=top { Label text=\"x\" } } }").is_empty());
    for side in ["top", "bottom", "left", "right"] {
        assert!(codes(&format!("View v {{ Button b {{ Popup p side={side} {{ Label text=\"x\" }} }} }}")).is_empty(), "{side}");
    }
    assert!(codes("View v { Button b { Popup p { Label text=\"x\" } } }").contains(&"G7020".to_string()));
    assert!(message("View v { Button b { Popup p { Label text=\"x\" } } }", "G7020").contains("there is no default"));
    assert!(message("View v { Button b { Popup p side=above { Label text=\"x\" } } }", "G7020").contains("`top`, `bottom`, `left` or `right`"));
}

/// `align=` is where the Popup sits along its side: three words,
/// centered when unwritten.
#[test]
fn align_takes_three_words() {
    for align in ["start", "center", "end"] {
        assert!(codes(&format!("View v {{ Button b {{ Popup p side=top, align={align} {{ Label text=\"x\" }} }} }}")).is_empty(), "{align}");
    }
    assert!(message("View v { Button b { Popup p side=top, align=middle { Label text=\"x\" } } }", "G7020").contains("`start`, `center` or `end`"));
}

/// A Popup opens beside the node it is written in: as a file's root it
/// has none. Its two words are a Popup's alone.
#[test]
fn a_popup_is_written_inside_the_node_it_opens_beside() {
    assert!(codes("Popup p side=top { Label text=\"x\" }").contains(&"G7019".to_string()));
    assert!(message("View v { Button b side=top }", "G2002").contains("where a Popup opens"));
    assert!(message("View v { Label l text=\"x\", align=center }", "G2002").contains("where a Popup opens"));
}

/// A Popup written in a Check is what the Check opens, no part of its
/// look: a Check with nothing else in it keeps its native box and text.
#[test]
fn a_check_with_only_a_popup_in_it_keeps_its_own_look() {
    let plain: Rect = Fx::new("View v { Check c halign=left, valign=top, text=\"More\" }").rect("c");
    let with_popup: Rect = Fx::new("View v { Check c halign=left, valign=top, text=\"More\" { Popup p side=top, visibility=gone { Label text=\"x\" } } }").rect("c");
    assert!(plain.w > 0.0 && plain.h > 0.0, "{plain:?}");
    assert_eq!(with_popup, plain, "as big as the Check with no Popup: its box and its text");
    let with_body: Rect = Fx::new("View v { Check c halign=left, valign=top { Rectangle 5x5 } }").rect("c");
    assert_ne!(with_body, plain, "a body of its own replaces that look");
}

// ---------------------------------------------------------------- showing

/// A Popup shows and hides with `visibility=`, like any node. Here a
/// `when selected` of the Check it is written in shows it: no code.
/// Hidden, it and what it holds take no part in the page.
#[test]
fn a_popup_shows_while_its_check_is_selected() {
    let mut fx = Fx::new(MENU_PAGE);
    assert!(!fx.shows("menu"), "the file says `visibility=gone`");
    assert_eq!(fx.rect("plate"), Rect::new(0.0, 0.0, 0.0, 0.0), "what a hidden Popup holds has no place");
    assert_eq!(fx.drawn_at(WHITE), None, "and is not drawn");
    fx.select("more", true);
    assert!(fx.shows("menu"));
    assert!(fx.drawn_at(WHITE).is_some());
    fx.select("more", false);
    assert!(!fx.shows("menu"));
    assert_eq!(fx.drawn_at(WHITE), None);
}

/// A Popup is as big as what it holds, and sits against the rectangle of
/// the node it is written in: here above the Check, centered on it, its
/// bottom edge on the Check's top edge. It does not size that node.
#[test]
fn a_popup_sits_against_its_parent_and_is_as_big_as_what_it_holds() {
    let mut fx = Fx::new(MENU_PAGE);
    let closed: Rect = fx.rect("more");
    assert_eq!(closed, Rect::new(200.0, 600.0, 60.0, 50.0));
    fx.select("more", true);
    assert_eq!(fx.rect("more"), closed, "the Check is no bigger for the Popup in it");
    assert_eq!(fx.rect("plate"), Rect::new(195.0, 300.0, 70.0, 300.0), "70 wide over a 60 wide Check: 5 past each side; 300 tall, ending where the Check starts");
    assert_eq!(fx.rect("menu"), fx.rect("plate"), "the Popup hugs what it holds");
    assert_eq!(fx.rect("pick"), Rect::new(195.0, 300.0, 70.0, 40.0), "what it holds moved with it");
}

/// The words `side=` and `align=` place it; a `when` may write them.
#[test]
fn side_and_align_place_a_popup() {
    let page = |words: &str| MENU_PAGE.replace("side=top, visibility=gone", &format!("{words}, visibility=gone"));
    let plate_of = |words: &str| -> Rect {
        let mut fx = Fx::new(&page(words));
        fx.select("more", true);
        fx.rect("plate")
    };
    assert_eq!(plate_of("side=top, align=start"), Rect::new(200.0, 300.0, 70.0, 300.0), "left edges in line");
    assert_eq!(plate_of("side=top, align=end"), Rect::new(190.0, 300.0, 70.0, 300.0), "right edges in line");
    assert_eq!(plate_of("side=left"), Rect::new(130.0, 380.0, 70.0, 300.0), "left of the Check; centered it would end under the page, so it ends on the page's bottom edge");
    assert_eq!(plate_of("side=left, align=end"), Rect::new(130.0, 350.0, 70.0, 300.0), "bottom edges in line");

    // A wide screen opens it to the left.
    let mut fx = Fx::new(&MENU_PAGE.replace("when selected menu.visibility = visible", "when selected menu.visibility = visible\n            when screen.landscape menu.side = left"));
    fx.select("more", true);
    assert_eq!(fx.rect("plate").x, 195.0, "an upright screen: above");
    fx.rt.set_screen(&fx.page, gogo_ui::Orientation::from_view(800.0, 400.0), gogo_ui::ScreenClass::from_view(800.0, 400.0));
    fx.frame();
    assert_eq!(fx.rect("plate").x, 130.0, "a wide screen: to the left");
}

/// A Popup that would reach past the page is moved back inside it by the
/// least that does.
#[test]
fn a_popup_stays_inside_the_page() {
    let mut fx = Fx::new(&MENU_PAGE.replace("margin_left=200", "margin_left=258").replace("width=70, height=300, background", "width=200, height=300, background"));
    fx.select("more", true);
    assert_eq!(fx.rect("more").x, 258.0);
    assert_eq!(fx.rect("plate"), Rect::new(120.0, 300.0, 200.0, 300.0), "200 wide, centered on a Check at the page's right edge: its right edge on the page's");

    // The safe area is the room: a notch at the top pushes it down.
    let mut fx = Fx::new(&MENU_PAGE.replace("height=300, background", "height=590, background"));
    fx.rt.safe = [0.0, 40.0, 0.0, 0.0];
    fx.select("more", true);
    assert_eq!(fx.rect("plate").y, 40.0, "590 tall above a Check at 600 would start at 10: it starts under the notch");
}

// ---------------------------------------------------------------- above the page

/// A Popup draws above the whole page: here above the red cover, which
/// is written after the bar and paints over everything the bar holds.
#[test]
fn a_popup_draws_above_the_whole_page() {
    let mut fx = Fx::new(MENU_PAGE);
    fx.select("more", true);
    let (cover, plate): (usize, usize) = (fx.drawn_at(RED).expect("the cover"), fx.drawn_at(WHITE).expect("the plate"));
    assert!(plate > cover, "the plate is drawn after the cover: {plate} and {cover}");
}

/// A finger on a Popup goes to what the Popup holds, also where it lies
/// outside the node it is written in, and under nothing of the page. A
/// finger beside it goes to the page as before.
#[test]
fn a_popup_takes_the_fingers_on_it() {
    let mut fx = Fx::new(MENU_PAGE);
    assert_eq!(hit(&fx.page, fx.kept.frame(), 230.0, 320.0, false), None, "closed: nothing takes a finger there");
    fx.select("more", true);
    assert_eq!(hit(&fx.page, fx.kept.frame(), 230.0, 320.0, false), Some(fx.id("pick")), "the Popup's button, 280 above its Check");
    assert_eq!(hit(&fx.page, fx.kept.frame(), 230.0, 500.0, false), Some(fx.id("plate")), "the Popup's plate");
    assert_eq!(hit(&fx.page, fx.kept.frame(), 100.0, 320.0, false), None, "beside the Popup: the page, as before");
    assert_eq!(hit(&fx.page, fx.kept.frame(), 230.0, 620.0, false), Some(fx.id("more")), "the Check is not covered: the Popup ends where it starts");

    let events: EventList = fx.tap(230.0, 320.0);
    assert!(events.iter().any(|event| event.node == fx.id("pick") && event.kind == EventKind::Press), "a tap on the Popup's button presses it: {events:?}");
    assert!(fx.shows("menu"), "a tap on the Popup leaves it open");
}

/// A tap on the Check opens the Popup and a second tap closes it: the
/// file alone ties the two.
#[test]
fn a_tap_on_the_check_opens_and_closes_its_popup() {
    let mut fx = Fx::new(MENU_PAGE);
    fx.tap(230.0, 620.0);
    assert!(fx.shows("menu"), "the Check is selected");
    fx.tap(230.0, 620.0);
    assert!(!fx.shows("menu"));
}

/// A Popup follows the node it is written in: where layout moves that
/// node, and where a Scroll pans it. A pan lays nothing out.
#[test]
fn a_popup_follows_its_parent() {
    let src: &str = "View v width=fill, height=fill {
    Scroll list halign=left, valign=top, width=320, height=400 {
        Column rows {
            gap 300
            Check more halign=left, width=60, height=50 {
                Rectangle face width=fill, height=fill, color=#202020FF
                Popup menu side=right, visibility=gone {
                    Panel plate width=70, height=50, background=#FFFFFFFF
                }
                when selected menu.visibility = visible
            }
            gap 600
        }
    }
}";
    let mut fx = Fx::new(src);
    fx.select("more", true);
    assert_eq!(fx.rect("more"), Rect::new(0.0, 300.0, 60.0, 50.0));
    assert_eq!(fx.rect("plate"), Rect::new(60.0, 300.0, 70.0, 50.0), "right of the Check");
    let list: NodeId = fx.id("list");
    fx.rt.set_scroll(list, [0.0, 120.0]);
    fx.frame();
    assert_eq!(fx.rect("more").y, 180.0, "the Scroll moved the Check up by 120");
    assert_eq!(fx.rect("plate"), Rect::new(60.0, 180.0, 70.0, 50.0), "and the Popup with it");
    assert!(fx.drawn_at(WHITE).is_some(), "drawn past the Scroll's cut: a Popup is cut by no node above it");
}

// ---------------------------------------------------------------- back

/// A back request hides the topmost Popup back can hide, by ending what
/// shows it. Here a Check's `selected` shows it: the Check is
/// unselected and its `on:change=` raised, as a tap would.
#[test]
fn back_hides_a_popup_its_check_shows() {
    let mut fx = Fx::new(MENU_PAGE);
    assert_eq!(fx.input.hide_top_popup(&fx.page, &mut fx.rt), None, "no Popup shows: the back request is the page's");
    fx.select("more", true);
    let events: EventList = fx.input.hide_top_popup(&fx.page, &mut fx.rt).expect("the Popup takes the back request");
    fx.frame();
    assert!(!fx.shows("menu"));
    assert!(!fx.rt.has(fx.id("more"), SELECTED), "the Check is unselected");
    let changed: Vec<&gogo_ui::Event> = events.iter().filter(|event| event.node == fx.id("more")).collect();
    assert_eq!(changed.len(), 1, "{events:?}");
    assert_eq!(changed[0].kind, EventKind::Change(Value::VBool(false)));
    assert!(changed[0].action.is_some(), "with the Check's `on:change=`");
    assert_eq!(fx.input.hide_top_popup(&fx.page, &mut fx.rt), None, "one back request, one Popup");
}

/// A Popup that code showed is hidden the way code would hide it. One
/// that the file's own line shows is not back's to hide.
#[test]
fn back_hides_a_popup_code_showed_and_leaves_one_the_file_shows() {
    let mut fx = Fx::new(&MENU_PAGE.replace("            when selected menu.visibility = visible\n", ""));
    let menu: NodeId = fx.id("menu");
    fx.rt.set_visible(&mut fx.page, menu, true);
    fx.frame();
    assert!(fx.shows("menu"));
    assert_eq!(fx.rect("plate"), Rect::new(195.0, 300.0, 70.0, 300.0));
    assert_eq!(fx.input.hide_top_popup(&fx.page, &mut fx.rt), Some(EventList::new()), "hidden, and no control has an event to raise");
    fx.frame();
    assert!(!fx.shows("menu"));
    assert_eq!(fx.input.hide_top_popup(&fx.page, &mut fx.rt), None, "nothing shows now: the next back request is the page's");

    let mut fx = Fx::new(&MENU_PAGE.replace("side=top, visibility=gone", "side=top").replace("            when selected menu.visibility = visible\n", ""));
    assert!(fx.shows("menu"), "a Popup with no `visibility=` shows like any node");
    assert_eq!(fx.input.hide_top_popup(&fx.page, &mut fx.rt), None, "the file shows it: back is the page's");
    fx.frame();
    assert!(fx.shows("menu"));
}

/// A Popup that a `when` of another state shows (here a wide screen) is
/// not back's to hide: back ends a Check's `selected` and a write from
/// code, nothing else.
#[test]
fn back_leaves_a_popup_that_a_screen_class_shows() {
    let mut fx = Fx::new(&MENU_PAGE.replace("when selected menu.visibility = visible", "when screen.landscape menu.visibility = visible"));
    assert!(!fx.shows("menu"));
    fx.rt.set_screen(&fx.page, gogo_ui::Orientation::from_view(800.0, 400.0), gogo_ui::ScreenClass::from_view(800.0, 400.0));
    fx.frame();
    assert!(fx.shows("menu"), "a wide screen shows it");
    assert_eq!(fx.input.hide_top_popup(&fx.page, &mut fx.rt), None, "the back request is the page's");
    fx.frame();
    assert!(fx.shows("menu"));
}

/// A Popup sizes nothing: a page that hugs what it holds is no bigger
/// while its Popup shows.
#[test]
fn a_popup_sizes_nothing() {
    let src: &str = "Panel v halign=left, valign=top {
    Check more width=60, height=50 {
        Rectangle width=fill, height=fill, color=#202020FF
        Popup menu side=bottom, visibility=gone {
            Panel plate width=200, height=300, background=#FFFFFFFF
        }
        when selected menu.visibility = visible
    }
}";
    let mut fx = Fx::new(src);
    assert_eq!(fx.rect("v"), Rect::new(0.0, 0.0, 60.0, 50.0), "as big as its Check");
    fx.select("more", true);
    assert_eq!(fx.rect("v"), Rect::new(0.0, 0.0, 60.0, 50.0), "and no bigger for the Popup");
    assert_eq!(fx.rect("plate"), Rect::new(0.0, 50.0, 200.0, 300.0), "which shows under the Check, past the page's own box");
}

/// Of two Popups that show, the one later in the file is on top, and a
/// back request hides that one first.
#[test]
fn the_later_popup_is_on_top_and_back_hides_it_first() {
    let src: &str = "View v width=fill, height=fill {
    Check first halign=left, valign=top, width=60, height=50, margin_top=300 {
        Popup firstMenu side=right, visibility=gone {
            Panel firstPlate width=200, height=50, background=#FFFFFFFF, interactive=true
        }
        when selected firstMenu.visibility = visible
    }
    Check second halign=left, valign=top, width=60, height=50, margin_left=100, margin_top=300 {
        Popup secondMenu side=right, visibility=gone {
            Panel secondPlate width=200, height=50, background=#FF0000FF, interactive=true
        }
        when selected secondMenu.visibility = visible
    }
}";
    let mut fx = Fx::new(src);
    fx.select("first", true);
    fx.select("second", true);
    assert_eq!((fx.rect("firstPlate").x, fx.rect("secondPlate").x), (60.0, 120.0), "they overlap from 160 to 260");
    assert!(fx.drawn_at(RED) > fx.drawn_at(WHITE), "the second is drawn after the first");
    assert_eq!(hit(&fx.page, fx.kept.frame(), 200.0, 320.0, false), Some(fx.id("secondPlate")), "and takes the finger where both lie");
    fx.input.hide_top_popup(&fx.page, &mut fx.rt).expect("one hidden");
    fx.frame();
    assert!(fx.shows("firstMenu") && !fx.shows("secondMenu"), "the one on top goes first");
    fx.input.hide_top_popup(&fx.page, &mut fx.rt).expect("one hidden");
    fx.frame();
    assert!(!fx.shows("firstMenu"));
}

/// A Popup written inside what another Popup holds opens against its
/// own parent, where the outer Popup put that parent.
#[test]
fn a_popup_inside_a_popup_opens_beside_its_own_parent() {
    let src: &str = "View v width=fill, height=fill {
    Check more halign=left, valign=top, width=60, height=50, margin_top=300 {
        Rectangle width=fill, height=fill, color=#202020FF
        Popup menu side=right, visibility=gone {
            Panel plate width=100, height=50, background=#FFFFFFFF, interactive=true {
                Check deeper halign=right, valign=top, width=30, height=50 {
                    Rectangle width=fill, height=fill, color=#404040FF
                    Popup submenu side=bottom, align=start, visibility=gone {
                        Panel subplate width=80, height=40, background=#FF0000FF, interactive=true
                    }
                    when selected submenu.visibility = visible
                }
            }
        }
        when selected menu.visibility = visible
    }
}";
    let mut fx = Fx::new(src);
    fx.select("more", true);
    fx.select("deeper", true);
    assert_eq!(fx.rect("plate"), Rect::new(60.0, 300.0, 100.0, 50.0), "right of the first Check");
    assert_eq!(fx.rect("deeper"), Rect::new(130.0, 300.0, 30.0, 50.0), "at the right end of the first Popup's plate");
    assert_eq!(fx.rect("subplate"), Rect::new(130.0, 350.0, 80.0, 40.0), "under the second Check, left edges in line");
    assert!(fx.drawn_at(RED) > fx.drawn_at(WHITE), "the inner one is drawn over the outer one");
    fx.input.hide_top_popup(&fx.page, &mut fx.rt).expect("one hidden");
    fx.frame();
    assert!(fx.shows("menu") && !fx.shows("submenu"), "back hides the inner one first");
}

/// A row of a `repeat` may hold a Popup: each row made from it has a
/// Popup of its own, which opens beside that row's own control.
#[test]
fn each_row_of_a_repeat_has_its_own_popup() {
    use gogo_ui::runtime::{Item, ItemValue};
    let src: &str = "View v width=fill, height=fill {
    Column rows halign=left, valign=top {
        repeat things as $thing by $thing.key {
            Check rowMore halign=left, width=60, height=40 {
                Rectangle width=fill, height=fill, color=#202020FF
                Popup rowMenu side=right, visibility=gone {
                    Panel rowPlate width=50, height=40, background=#FFFFFFFF
                }
                when selected rowMenu.visibility = visible
            }
        }
    }
}";
    let mut fx = Fx::new(src);
    let things: NodeId = fx.id("things");
    let items: Vec<Item> = (0..3).map(|i| Item::Record(vec![("key".to_string(), ItemValue::Str(i.to_string()))])).collect();
    fx.rt.set_items(&mut fx.page, things, &items).expect("three rows");
    fx.frame();
    let rows: Vec<NodeId> = fx.rt.items(&fx.page, things);
    assert_eq!(rows.len(), 3);
    assert!(fx.page.popups.len() >= 3, "a Popup a row: {:?}", fx.page.popups);
    let second: NodeId = rows[1];
    fx.rt.set_state(second, SELECTED, true);
    fx.frame();
    let shown: Vec<NodeId> = fx.page.popups.iter().copied().filter(|popup| !fx.kept.look().gone(&fx.page, *popup)).collect();
    assert_eq!(shown.len(), 1, "the second row's alone: {shown:?}");
    assert!(fx.page.is_under(shown[0], second), "written in the second row");
    let (row, menu): (Rect, Rect) = (fx.kept.rects()[second.index()], fx.kept.rects()[shown[0].index()]);
    assert_eq!((row.x, row.y, row.w), (0.0, 40.0, 60.0), "the second row: {row:?}");
    assert_eq!(menu, Rect::new(60.0, 40.0, 50.0, 40.0), "right of the second row's Check");
}

/// No node between a Popup and the page's root fades or cuts it: a
/// half see-through bar with `clip_contents=true` shows its menu whole
/// and solid.
#[test]
fn a_popup_is_not_faded_or_cut_by_the_nodes_above_it() {
    let mut fx = Fx::new(&MENU_PAGE.replace("Layers bar halign=left, valign=top, width=320, height=60, margin_top=600 {", "Layers bar halign=left, valign=top, width=320, height=60, margin_top=600, opacity=0.5, clip_contents=true {"));
    fx.select("more", true);
    let plate: &Draw = &fx.draws[fx.drawn_at(WHITE).expect("the plate is drawn with its own color, not faded")];
    assert!(matches!(plate, Draw::Quad { rect, .. } if *rect == Rect::new(195.0, 300.0, 70.0, 300.0)), "whole, 300 above the bar that would cut it: {plate:?}");
    let (first_cut, plate_at): (Option<usize>, usize) = (fx.draws.iter().position(|draw| matches!(draw, Draw::ClipPush { .. })), fx.drawn_at(WHITE).unwrap());
    let last_uncut: Option<usize> = fx.draws.iter().rposition(|draw| matches!(draw, Draw::ClipPop));
    assert!(first_cut.is_some() && last_uncut.is_some_and(|at| at < plate_at), "the bar's cut ends before the plate is drawn: cut from {first_cut:?} to {last_uncut:?}, plate at {plate_at}");
}

/// What a node takes from the nodes above it (a font size, a text
/// color) a Popup's content takes from the page's root, not from the
/// node the Popup is written in: it sits on the root.
#[test]
fn what_a_popup_holds_reads_the_page_and_not_the_popups_parent() {
    let fx = Fx::new("View v { Button more font_color=#FF0000FF, font_size=30 { Label inside text=\"a\"
 Popup p side=top { Label inPopup text=\"b\" } } }");
    let look = fx.kept.look();
    assert_eq!(look.font_size(&fx.page, fx.id("inside")), Some(30.0), "the Button's own Label reads the Button");
    assert_eq!((look.font_size(&fx.page, fx.id("inPopup")), look.font_color(&fx.page, fx.id("inPopup"))), (None, None), "the Popup's Label does not");
}

// ---------------------------------------------------------------- other roads

/// The road a test takes with no runtime's frame (`KeptFrame::fresh`)
/// seats a Popup too.
#[test]
fn a_frame_made_fresh_seats_a_popup() {
    let c = compile(MENU_PAGE, &|_| None);
    let mut rt = Runtime::new(&c.page);
    rt.set_state(c.page.find("more").unwrap(), SELECTED, true);
    let kept: KeptFrame = KeptFrame::fresh(&c.page, gogo_ui::resolve(&c.page, &rt), Rect::new(0.0, 0.0, W, H), &FixedAdvance);
    assert_eq!(kept.rects()[c.page.find("plate").unwrap().index()], Rect::new(195.0, 300.0, 70.0, 300.0));
}

/// A page tells the Popups of its file apart from its other nodes, and a
/// row cloned from one with a Popup has a Popup of its own.
#[test]
fn a_page_lists_its_popups() {
    let fx = Fx::new(MENU_PAGE);
    assert_eq!(fx.page.popups, [fx.id("menu")]);
    assert_eq!(fx.page.prop(fx.id("menu"), PropKey::Side), Some(Value::VIdent(fx.page.intern_lookup("top"))));
}
