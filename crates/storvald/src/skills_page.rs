//! First page: system skills on left, picked one on right. Picture:
//! `ui/skills_page.gogo`.

use turbodot::prelude::*;

use crate::AppState;
use crate::about_page::AboutPage;
use crate::skills::Skill;
use crate::ui::skills_page::{Handler, NAMES, PAGE};

/// Skills page: which skill is picked.
#[derive(Default)]
pub struct SkillsPage {
    /// Picked skill's place in `AppState::skills`. Stays 0 for empty list.
    picked: usize,
}

impl SkillsPage {
    /// Picked skill, `None` when there are no skills.
    pub fn picked<'a>(&self, app: &'a AppState) -> Option<&'a Skill> {
        app.skills.get(self.picked)
    }

    /// Writes picked skill into right side, scrolls its text back to top.
    fn show_picked(&self, host: &mut Host<AppState>) {
        let (name, about, text): (&str, &str, &str) = match self.picked(host.app) {
            Some(skill) => (&skill.name, &skill.description, &skill.text),
            None => ("", "", ""),
        };
        host.ui.set_text(NAMES.skill_name, name);
        host.ui.set_text(NAMES.skill_about, about);
        host.ui.set_text(NAMES.skill_text, text);
        host.ui.set_scroll(NAMES.skill_scroll, [0.0, 0.0]);
    }
}

impl Page<AppState> for SkillsPage {
    const PAGE: &'static BuiltPage = &PAGE;
    type Handler = Handler;

    /// Writes list and first pick, once: return from another page finds
    /// pick and text's scroll place as left.
    fn loaded(&mut self, host: &mut Host<AppState>) {
        let rows: Vec<Item> = host.app.skills.iter().map(|skill| Item::Record(vec![("name".to_string(), ItemValue::Str(skill.name.clone()))])).collect();
        host.ui.set_items(NAMES.skills, &rows).expect("every skill has a name of its own");
        if let Some(skill) = self.picked(host.app) {
            host.ui.set_selected(NAMES.skill_list, &[ItemKey::Str(skill.name.clone())]).expect("the picked skill is a row");
        }
        self.show_picked(host);
    }

    fn show(&mut self, host: &mut Host<AppState>) {
        host.ui.set_text(NAMES.carried, &host.app.carried_line());
    }

    fn handle(&mut self, host: &mut Host<AppState>, event: PageEvent<Handler>) {
        match event.handler {
            Handler::PickSkill => {
                let Some(ItemKey::Str(name)) = host.ui.selected_keys(NAMES.skill_list).into_iter().next() else { return };
                if let Some(i) = host.app.skills.iter().position(|skill| skill.name == name) {
                    self.picked = i;
                    self.show_picked(host);
                }
            }
            Handler::OpenAbout => host.nav.go(AboutPage),
        }
    }
}
