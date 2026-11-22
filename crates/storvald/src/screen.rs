//! The first screen: the system skills on the left, the picked one on the
//! right.

use std::path::PathBuf;

use gogo_ui::runtime::{Item, ItemKey, ItemValue};
use gogo_ui::{Event, EventKind, NodeId};
use turbodot::{App, PageSource, Ui};

use crate::skills::Skill;

/// The page's file in the `ui` folder.
const PAGE_FILE: &str = "skills.gogo";

/// The page's text, carried inside the program.
const PAGE: &str = include_str!("../ui/skills.gogo");

/// The one font of the screen, carried inside the program.
const FONT: &[u8] = include_bytes!("../assets/Roboto Regular.ttf");

/// The size the window opens at, in epx.
pub const SIZE: (f32, f32) = (1100.0, 720.0);

/// Makes the page of the skills screen, ready for its first frame.
///
/// A debug build that still has its source folder reads the page from
/// `ui/skills.gogo` and follows the file, so a save shows in the running
/// window. Every other build uses the text it carries, and so runs from
/// any folder with nothing beside it.
///
/// Returns the compiler's errors when the page does not compile.
pub fn ui() -> Result<Ui, String> {
    let fonts: [(&str, Vec<u8>); 1] = [("Roboto Regular", FONT.to_vec())];
    let dir: PathBuf = PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/ui"));
    let source: PageSource<'_> = if cfg!(debug_assertions) && dir.join(PAGE_FILE).is_file() { PageSource::File { dir, name: PAGE_FILE } } else { PageSource::Text(PAGE) };
    let mut ui: Ui = Ui::new(source, &fonts)?;
    for file in symbol_font_files() {
        if ui.add_fallback_font_file(&file) {
            break;
        }
    }
    Ok(ui)
}

/// Font files of the operating system that have arrows and other signs,
/// the likeliest first.
///
/// Needed because the skills' texts use signs Roboto does not have ("→"),
/// and each would show as an empty box. On a computer with none of these
/// files the boxes stay.
fn symbol_font_files() -> Vec<PathBuf> {
    let mut files: Vec<PathBuf> = Vec::new();
    if let Some(windows) = std::env::var_os("SystemRoot") {
        files.push(PathBuf::from(windows).join("Fonts").join("seguisym.ttf"));
    }
    files.push(PathBuf::from("/System/Library/Fonts/Apple Symbols.ttf"));
    files.push(PathBuf::from("/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf"));
    files
}

/// The nodes of `ui/skills.gogo` that the screen's code writes and reads.
///
/// Needed so the code asks the page for a node once, when the page comes
/// in, and not at every event.
#[derive(Clone, Copy)]
struct Nodes {
    skills: NodeId,
    skill_list: NodeId,
    carried: NodeId,
    skill_name: NodeId,
    skill_about: NodeId,
    skill_text: NodeId,
    skill_scroll: NodeId,
}

impl Nodes {
    /// Finds the nodes in `ui`'s page.
    ///
    /// WARNING: panics when the page lacks one of the names (`Ui::node`).
    fn of(ui: &Ui) -> Nodes {
        Nodes { skills: ui.node("skills"), skill_list: ui.node("skillList"), carried: ui.node("carried"), skill_name: ui.node("skillName"), skill_about: ui.node("skillAbout"), skill_text: ui.node("skillText"), skill_scroll: ui.node("skillScroll") }
    }
}

/// The skills screen: what it lists and which skill is picked.
pub struct SkillsScreen {
    skills: Vec<Skill>,
    /// The picked skill's place in `skills`. Stays 0 for an empty list.
    picked: usize,
    nodes: Nodes,
}

impl SkillsScreen {
    /// A screen that lists `skills` on the page of `ui` and opens on the
    /// first.
    pub fn new(skills: Vec<Skill>, ui: &Ui) -> SkillsScreen {
        SkillsScreen { skills, picked: 0, nodes: Nodes::of(ui) }
    }

    /// The picked skill, `None` when there are no skills.
    pub fn picked(&self) -> Option<&Skill> {
        self.skills.get(self.picked)
    }

    /// Writes the picked skill into the right side and scrolls its text
    /// back to the top.
    fn show_picked(&self, ui: &mut Ui) {
        let (name, about, text): (&str, &str, &str) = match self.picked() {
            Some(skill) => (&skill.name, &skill.description, &skill.text),
            None => ("", "", ""),
        };
        ui.set_text(self.nodes.skill_name, name);
        ui.set_text(self.nodes.skill_about, about);
        ui.set_text(self.nodes.skill_text, text);
        ui.set_scroll(self.nodes.skill_scroll, [0.0, 0.0]);
    }
}

impl App for SkillsScreen {
    fn show(&mut self, ui: &mut Ui) {
        // A page that came in by a reload may have its nodes in other places.
        self.nodes = Nodes::of(ui);
        let rows: Vec<Item> = self.skills.iter().map(|skill| Item::Record(vec![("name".to_string(), ItemValue::Str(skill.name.clone()))])).collect();
        ui.set_items(self.nodes.skills, &rows).expect("every skill has a name of its own");
        if let Some(skill) = self.picked() {
            ui.set_selected(self.nodes.skill_list, &[ItemKey::Str(skill.name.clone())]).expect("the picked skill is a row");
        }
        let count: usize = self.skills.len();
        ui.set_text(self.nodes.carried, &format!("storvald {} carries {count} {}", env!("CARGO_PKG_VERSION"), if count == 1 { "skill" } else { "skills" }));
        self.show_picked(ui);
    }

    fn handle(&mut self, ui: &mut Ui, events: &[Event]) {
        for event in events {
            if !matches!(event.kind, EventKind::Change(_)) || event.node != self.nodes.skill_list {
                continue;
            }
            let Some(ItemKey::Str(name)) = ui.selected_keys(self.nodes.skill_list).into_iter().next() else { continue };
            if let Some(i) = self.skills.iter().position(|skill| skill.name == name) {
                self.picked = i;
                self.show_picked(ui);
            }
        }
    }
}
