//! The system skills this program carries inside it.

/// The `SKILL.md` file of every system skill, by folder name, in the order
/// the list shows them.
///
/// Needed so Storvald is one file that works with nothing beside it: the
/// skills are read when the program is built, not when it runs.
///
/// WARNING: a new folder under `skills/` needs a line here. The test
/// `every_skill_folder_is_bundled` fails for a folder this list leaves out.
const BUNDLED: &[(&str, &str)] = &[
    ("doc", include_str!("../skills/doc/SKILL.md")),
    ("explain-comments", include_str!("../skills/explain-comments/SKILL.md")),
    ("make-todo", include_str!("../skills/make-todo/SKILL.md")),
];

/// One skill: instructions an AI agent follows for one kind of work.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Skill {
    /// The name an agent calls the skill by, which is its folder's name.
    pub name: String,
    /// When to use the skill: the one line an agent reads to decide.
    pub description: String,
    /// The instructions: the file's text after its head.
    pub text: String,
}

impl Skill {
    /// Reads a `SKILL.md` file: a head between two `---` lines that names
    /// the skill and describes it, then the instructions.
    ///
    /// ```text
    /// ---
    /// name: doc
    /// description: Write or improve documentation ...
    /// ---
    ///
    /// # Writing documentation that teaches
    /// ```
    ///
    /// Returns what is wrong with the head when it has no `name:` or no
    /// `description:` line, or is not closed.
    pub fn parse(file: &str) -> Result<Skill, String> {
        let file: String = file.replace("\r\n", "\n");
        let rest: &str = file.strip_prefix("---\n").ok_or("the file does not start with a `---` line")?;
        let (head, text) = rest.split_once("\n---\n").ok_or("the head is not closed by a `---` line")?;
        let mut name: Option<&str> = None;
        let mut description: Option<&str> = None;
        for line in head.lines() {
            match line.split_once(':') {
                Some(("name", value)) => name = Some(value.trim()),
                Some(("description", value)) => description = Some(value.trim()),
                _ => {}
            }
        }
        let name: &str = name.filter(|n| !n.is_empty()).ok_or("the head has no `name:` line")?;
        let description: &str = description.filter(|d| !d.is_empty()).ok_or("the head has no `description:` line")?;
        Ok(Skill { name: name.to_string(), description: description.to_string(), text: text.trim().to_string() })
    }
}

/// The system skills this program carries, in the order the list shows
/// them.
///
/// WARNING: panics on a bundled file whose head cannot be read, or whose
/// `name:` is not its folder's name. Both are mistakes in this repository
/// that the tests catch before a program is given out.
pub fn bundled() -> Vec<Skill> {
    BUNDLED
        .iter()
        .map(|(folder, file)| {
            let skill: Skill = Skill::parse(file).unwrap_or_else(|why| panic!("skills/{folder}/SKILL.md: {why}"));
            assert_eq!(skill.name, *folder, "skills/{folder}/SKILL.md names itself `{}`", skill.name);
            skill
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_skill_file_gives_its_name_its_description_and_its_text() {
        let skill: Skill = Skill::parse("---\nname: doc\ndescription: Write docs: plainly.\n---\n\n# Writing\n\nBody.\n").expect("a whole file");
        assert_eq!(skill, Skill { name: "doc".to_string(), description: "Write docs: plainly.".to_string(), text: "# Writing\n\nBody.".to_string() });
    }

    /// A file saved with Windows line ends reads the same.
    #[test]
    fn windows_line_ends_read_the_same() {
        let skill: Skill = Skill::parse("---\r\nname: doc\r\ndescription: d\r\n---\r\nBody.\r\n").expect("a whole file");
        assert_eq!((skill.name.as_str(), skill.text.as_str()), ("doc", "Body."));
    }

    #[test]
    fn a_head_that_misses_a_line_says_which() {
        assert_eq!(Skill::parse("# no head"), Err("the file does not start with a `---` line".to_string()));
        assert_eq!(Skill::parse("---\nname: doc\n"), Err("the head is not closed by a `---` line".to_string()));
        assert_eq!(Skill::parse("---\ndescription: d\n---\n"), Err("the head has no `name:` line".to_string()));
        assert_eq!(Skill::parse("---\nname: doc\n---\n"), Err("the head has no `description:` line".to_string()));
    }

    /// Every folder under `skills/` is in `BUNDLED`, and nothing else is.
    ///
    /// Needed because the list is written by hand: a skill added as a
    /// folder alone would be in the repository and missing from the
    /// program, and nothing else would say so.
    #[test]
    fn every_skill_folder_is_bundled() {
        let dir: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/skills");
        let mut folders: Vec<String> = std::fs::read_dir(dir).expect("the skills folder").map(|e| e.expect("a folder entry").file_name().to_string_lossy().into_owned()).collect();
        folders.sort();
        let mut bundled: Vec<String> = bundled().into_iter().map(|s| s.name).collect();
        bundled.sort();
        assert_eq!(bundled, folders);
    }
}
