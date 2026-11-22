//! Storvald: an AI-first creative environment for games and apps.
//!
//! Storvald is in development (<https://github.com/nokola/storvald>). This
//! version is one program, the `storvald` command, with one screen: the
//! system skills it carries inside it.
//!
//! - [`skills`]: the skills, read from the `SKILL.md` files under
//!   `skills/` when the program is built.
//! - [`screen`]: the screen that lists them, a gogo page (`ui/skills.gogo`)
//!   run by TurboDot.
//! - [`call`]: what a call of the command asks for.

pub mod call;
pub mod screen;
pub mod skills;
