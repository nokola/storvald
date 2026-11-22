//! The skills an agent reads in this repository against the ones the
//! program carries.
//!
//! Not in the crates.io package (`exclude` in Cargo.toml): it reads
//! `.claude/skills` at the top of the repository, which a package does not
//! carry.

use std::path::PathBuf;

use storvald::skills::{self, Skill};

/// Each bundled skill has a copy under `.claude/skills`, the same byte for
/// byte.
///
/// Needed because the same file is in two places for now: the program
/// carries `crates/storvald/skills`, and an agent working in this
/// repository reads `.claude/skills`. Edited in one place only, the agent
/// would follow other instructions than the program gives out, and nothing
/// else would say so.
#[test]
fn the_agents_copies_match_the_bundled_skills() {
    let bundled: Vec<Skill> = skills::bundled();
    assert!(!bundled.is_empty(), "the program carries skills");
    for skill in bundled {
        let ours: PathBuf = PathBuf::from(format!("{}/skills/{}/SKILL.md", env!("CARGO_MANIFEST_DIR"), skill.name));
        let agents: PathBuf = PathBuf::from(format!("{}/../../.claude/skills/{}/SKILL.md", env!("CARGO_MANIFEST_DIR"), skill.name));
        let ours_bytes: Vec<u8> = std::fs::read(&ours).unwrap_or_else(|e| panic!("{}: {e}", ours.display()));
        let agents_bytes: Vec<u8> = std::fs::read(&agents).unwrap_or_else(|e| panic!("{}: {e}", agents.display()));
        assert!(ours_bytes == agents_bytes, "`{}`: {} and {} differ; copy the first over the second", skill.name, ours.display(), agents.display());
    }
}
