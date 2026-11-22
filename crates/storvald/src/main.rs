//! The `storvald` command.
//!
//! Storvald is in development (<https://github.com/nokola/storvald>). A
//! bare `storvald` opens its one screen, the system skills this program
//! carries. It has no other command yet.

use std::process::ExitCode;

use storvald::call::Call;
use storvald::screen::{self, SkillsScreen};
use storvald::skills;
use turbodot::WindowOptions;

/// What the command says when asked for help or for its version.
const NOTE: &str = concat!("storvald ", env!("CARGO_PKG_VERSION"), ": in development. A bare `storvald` opens a window that lists the system skills it carries.\nhttps://github.com/nokola/storvald");

/// Opens the window on the skills screen and stays until it is closed.
///
/// Returns why the screen could not be shown.
fn window() -> Result<(), String> {
    let (width, height): (f32, f32) = screen::SIZE;
    let ui: turbodot::Ui = screen::ui()?;
    let app: SkillsScreen = SkillsScreen::new(skills::bundled(), &ui);
    turbodot::run(WindowOptions { title: "Storvald", width, height }, ui, app)
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match Call::parse(&args) {
        Call::Note => {
            println!("{NOTE}");
            ExitCode::SUCCESS
        }
        Call::Unknown(asked) => {
            eprintln!("{NOTE}");
            eprintln!("storvald: `{asked}` is not a command yet");
            ExitCode::from(2)
        }
        Call::Window => match window() {
            Ok(()) => ExitCode::SUCCESS,
            Err(why) => {
                eprintln!("storvald: the window cannot be shown: {why}");
                ExitCode::FAILURE
            }
        },
    }
}
