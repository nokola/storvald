//! What a call of the `storvald` command asks for.

/// What one call of the command asks for.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Call {
    /// A bare `storvald`: open the window.
    Window,
    /// `--help`, `-h`, `--version` or `-V`: say what this version is.
    Note,
    /// Anything else: a command this version does not have, as written.
    Unknown(String),
}

impl Call {
    /// Reads the words after the program's name.
    ///
    /// Needed so a script or an AI agent that calls a command this version
    /// does not have gets a failure, not a window, and not a success that
    /// did nothing.
    pub fn parse(args: &[String]) -> Call {
        if args.is_empty() {
            return Call::Window;
        }
        if args.iter().all(|arg| matches!(arg.as_str(), "--help" | "-h" | "--version" | "-V")) {
            return Call::Note;
        }
        Call::Unknown(args.join(" "))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn call(args: &[&str]) -> Call {
        let args: Vec<String> = args.iter().map(|arg| arg.to_string()).collect();
        Call::parse(&args)
    }

    /// A bare call opens the window. The window itself is not opened
    /// here: a test machine may have no screen.
    #[test]
    fn a_bare_call_asks_for_the_window() {
        assert_eq!(call(&[]), Call::Window);
    }

    #[test]
    fn help_and_version_ask_for_the_note() {
        for arg in ["--help", "-h", "--version", "-V"] {
            assert_eq!(call(&[arg]), Call::Note, "{arg}");
        }
    }

    #[test]
    fn any_other_word_is_a_command_this_version_does_not_have() {
        assert_eq!(call(&["go"]), Call::Unknown("go".to_string()));
        assert_eq!(call(&["--version", "go"]), Call::Unknown("--version go".to_string()));
    }
}
