//! Storvald: an AI-first creative environment for games and apps.
//!
//! Storvald is in development (<https://github.com/nokola/storvald>). This
//! version is one program, the `storvald` command, with two pages: the
//! system skills it carries inside it, and what this version is.
//!
//! - [`skills`]: the skills, read from the `SKILL.md` files under
//!   `skills/` when the program is built.
//! - [`skills_page`]: the page that lists them (`ui/skills_page.gogo`).
//! - [`about_page`]: the page that says what this version is
//!   (`ui/about_page.gogo`).
//! - [`call`]: what a call of the command asks for.
//!
//! Pages run on TurboDot: [`program`] makes them with no window, the
//! command shows that program in one.

use std::path::PathBuf;

use turbodot::Program;

pub mod about_page;
pub mod call;
pub mod skills;
pub mod skills_page;

turbodot::built_pages!();

use skills::Skill;
use skills_page::SkillsPage;

/// Size window opens at, in epx.
pub const WINDOW_SIZE_EPX: (f32, f32) = (1100.0, 720.0);

/// Program-wide state, reached by every page.
pub struct AppState {
    /// System skills this program carries, in list order.
    pub skills: Vec<Skill>,
}

impl AppState {
    /// Line with this version and count of skills carried.
    pub fn carried_line(&self) -> String {
        let count: usize = self.skills.len();
        format!("storvald {} carries {count} {}", env!("CARGO_PKG_VERSION"), if count == 1 { "skill" } else { "skills" })
    }
}

/// Makes Storvald's pages, open on skills page, with no window.
///
/// `does_follow_page_files`: true = debug build with its source folder
/// follows the `.gogo` files, a save shows in running window. Test
/// passes false: no page changes under it.
///
/// Returns why pages cannot show.
pub fn program(does_follow_page_files: bool) -> Result<Program<AppState>, String> {
    let app: AppState = AppState { skills: skills::bundled() };
    let mut program: Program<AppState> = if does_follow_page_files { Program::following_page_files(&ui::BUILT, app, SkillsPage::default())? } else { Program::new(&ui::BUILT, app, SkillsPage::default())? };
    for file in symbol_font_files() {
        if program.add_fallback_font_file(&file) {
            break;
        }
    }
    Ok(program)
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
