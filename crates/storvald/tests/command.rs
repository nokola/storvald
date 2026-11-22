//! The `storvald` command, run as a program.
//!
//! Needed because a script sees the exit code and the two output streams of
//! the built program, and a call of a function inside it shows neither.
//!
//! A bare `storvald` is not run here: it opens a window, and a test machine
//! may have no screen. `call.rs` checks that a bare call asks for the
//! window, and `skills_screen.rs` runs the screen with no window.

use std::process::{Command, Output};

/// Runs the built `storvald` with `args` and returns what it left behind.
fn storvald(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_storvald")).args(args).output().expect("the built command runs")
}

/// A call for help or for the version says what this version is and
/// succeeds.
#[test]
fn a_call_for_help_or_the_version_says_what_this_version_is() {
    let calls: [&[&str]; 4] = [&["--version"], &["-V"], &["--help"], &["-h"]];
    for args in calls {
        let out: Output = storvald(args);
        assert!(out.status.success(), "{args:?}");
        let said: String = String::from_utf8(out.stdout).expect("text");
        assert!(said.starts_with(concat!("storvald ", env!("CARGO_PKG_VERSION"))), "{args:?}: {said}");
        assert!(said.contains("in development"), "{args:?}: {said}");
    }
}

/// A command this version does not have fails, and says which one was asked
/// for, so a script that calls it does not take a call that did nothing for
/// a pass.
#[test]
fn a_command_that_does_not_exist_yet_fails() {
    let out: Output = storvald(&["go"]);
    assert_eq!(out.status.code(), Some(2));
    assert!(out.stdout.is_empty(), "nothing on the stream a script reads results from");
    let said: String = String::from_utf8(out.stderr).expect("text");
    assert!(said.contains("`go` is not a command yet"), "{said}");
}

/// The Windows program carries the manifest line that tells Windows to give
/// it no console of its own.
///
/// Needed because `storvald` is a console program, so that a terminal
/// waits for it and reads its exit code, and Windows opens a console
/// window beside the app's window for a console program started by a
/// double click. With the line, a start from a terminal still uses that
/// terminal, and a double click opens the app's window alone.
///
/// The line works on Windows 11 24H2 and later; an older Windows does not
/// read it and opens the console as before. Whether a console opens is
/// checked by hand (docs/todo-ai-dev.md/AD30): this test reads the built
/// program's bytes, which hold the manifest as plain text.
#[cfg(all(windows, target_env = "msvc"))]
#[test]
fn the_windows_program_asks_for_no_console_of_its_own() {
    let program: Vec<u8> = std::fs::read(env!("CARGO_BIN_EXE_storvald")).expect("the built command");
    let has = |text: &str| program.windows(text.len()).any(|bytes| bytes == text.as_bytes());
    assert!(has("consoleAllocationPolicy"), "the program carries no console allocation policy");
    assert!(has(">detached</"), "the policy is not `detached`");
}
