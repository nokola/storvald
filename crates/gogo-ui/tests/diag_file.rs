//! A diagnostic names the file its line is in: a line of a `uses` file
//! is reported with that file (`Diagnostic::file`), a line of the page
//! with none.
//!
//! Needed because a page's compile reads its widgets' and its theme's
//! files too, and a diagnostic held a line number alone: the build
//! printed an error in a widget's file as that line of the page.
use gogo_ui::{Diagnostic, compile};

/// The files of one test: the page, and what its `uses` lines name.
fn diags(page: &str, files: &[(&str, &str)]) -> Vec<Diagnostic> {
    let files: Vec<(String, String)> = files.iter().map(|(name, text)| (name.to_string(), text.to_string())).collect();
    compile(page, &move |path: &str| files.iter().find(|(name, _)| name == path).map(|(_, text)| text.clone())).diags
}

/// The one diagnostic with `code`, as (file, line).
fn place_of(diags: &[Diagnostic], code: &str) -> (String, u32) {
    let found: Vec<&Diagnostic> = diags.iter().filter(|d| d.code == code).collect();
    assert_eq!(found.len(), 1, "one {code} expected: {diags:#?}");
    (found[0].file.clone(), found[0].line)
}

const PAGE: &str = "uses \"widgets/card.gogo\"

View page {
    Card first
}
";

#[test]
fn a_line_of_the_page_names_no_file() {
    let found = diags("View page {\n    Label title colour=red\n}\n", &[]);
    assert_eq!(place_of(&found, "G2001"), (String::new(), 2));
}

#[test]
fn a_wrong_word_in_a_widgets_body_names_the_widgets_file() {
    let card = "widget Card {\n    Panel plate {\n        Label caption colour=red\n    }\n}\n";
    let found = diags(PAGE, &[("widgets/card.gogo", card)]);
    assert_eq!(place_of(&found, "G2001"), ("widgets/card.gogo".to_string(), 3));
}

#[test]
fn a_wrong_word_on_the_widgets_root_line_names_the_widgets_file() {
    let card = "widget Card {\n    Panel plate colour=red {\n        Label caption text=\"x\"\n    }\n}\n";
    let found = diags(PAGE, &[("widgets/card.gogo", card)]);
    assert_eq!(place_of(&found, "G2001"), ("widgets/card.gogo".to_string(), 2));
}

#[test]
fn a_wrong_word_on_the_line_that_uses_a_widget_names_the_page() {
    let card = "widget Card {\n    Panel plate {\n        Label caption text=\"x\"\n    }\n}\n";
    let page = "uses \"widgets/card.gogo\"\n\nView page {\n    Card first colour=red\n}\n";
    let found = diags(page, &[("widgets/card.gogo", card)]);
    assert_eq!(place_of(&found, "G2001"), (String::new(), 4));
}

#[test]
fn what_the_page_puts_into_a_widgets_slot_names_the_page() {
    // The slot sits under a node of the widget's file, two lines deep.
    let card = "widget Card {\n    Panel plate {\n        Column inner {\n            slot\n        }\n    }\n}\n";
    let page = "uses \"widgets/card.gogo\"\n\nView page {\n    Card first {\n        Label inside colour=red\n    }\n}\n";
    let found = diags(page, &[("widgets/card.gogo", card)]);
    assert_eq!(place_of(&found, "G2001"), (String::new(), 5));
}

/// The case that was met: a widget moves an item into a seat that is
/// not there, in its own `when screen.…` block.
#[test]
fn a_widgets_own_screen_block_names_the_widgets_file() {
    let card = "widget Card {\n    when screen.landscape {\n        caption.seat = nowhere\n    }\n    Panel plate {\n        Label caption text=\"x\"\n    }\n}\n";
    let found = diags(PAGE, &[("widgets/card.gogo", card)]);
    assert_eq!(place_of(&found, "G1016"), ("widgets/card.gogo".to_string(), 3));
}

#[test]
fn a_style_from_another_file_names_that_file() {
    let theme = "style Loud {\n    when held colour=red\n}\n";
    let page = "uses \"styles/theme.gogo\"\n\nView page {\n    Button go .Loud text=\"go\"\n}\n";
    let found = diags(page, &[("styles/theme.gogo", theme)]);
    assert_eq!(place_of(&found, "G2001"), ("styles/theme.gogo".to_string(), 2));
}

/// A state word no control raises, in a style's `when`: the `when`
/// block refuses it as it is read, and a second check refuses it once
/// the tree stands. Both name the style's file.
#[test]
fn a_wrong_state_word_in_a_style_names_the_styles_file_both_times() {
    let theme = "style Loud {\n    when nosuchstate background=#FF0000FF\n}\n";
    let page = "uses \"styles/theme.gogo\"\n\nView page {\n    Button go .Loud text=\"go\"\n}\n";
    let found = diags(page, &[("styles/theme.gogo", theme)]);
    let refused: Vec<&Diagnostic> = found.iter().filter(|d| d.code == "G0036").collect();
    assert_eq!(refused.len(), 2, "{found:#?}");
    assert!(refused.iter().all(|d| d.file == "styles/theme.gogo" && d.line == 2), "{found:#?}");
}

/// A style written for one kind of control (`for=Button`) has its state
/// words checked once the tree stands, with no wearer in sight.
#[test]
fn a_wrong_state_word_in_a_style_for_one_kind_names_the_styles_file() {
    let theme = "style Loud for=Button {\n    when nosuchstate background=#FF0000FF\n}\n";
    let page = "uses \"styles/theme.gogo\"\n\nView page {\n    Label title text=\"x\"\n}\n";
    let found = diags(page, &[("styles/theme.gogo", theme)]);
    let refused: Vec<&Diagnostic> = found.iter().filter(|d| d.code == "G0036").collect();
    assert!(!refused.is_empty() && refused.iter().all(|d| d.file == "styles/theme.gogo" && d.line == 2), "{found:#?}");
}

#[test]
fn a_line_that_does_not_parse_names_its_file() {
    let card = "widget Card {\n    Panel plate {\n";
    let found = diags(PAGE, &[("widgets/card.gogo", card)]);
    let unclosed: Vec<&Diagnostic> = found.iter().filter(|d| d.code == "G0002").collect();
    assert!(!unclosed.is_empty() && unclosed.iter().all(|d| d.file == "widgets/card.gogo"), "{found:#?}");
    assert!(!found.iter().any(|d| d.message.starts_with("widgets/card.gogo:")), "the file is said once, not in the words too: {found:#?}");
}

#[test]
fn a_file_used_by_a_used_file_names_itself() {
    let card = "uses \"inner.gogo\"\nwidget Card {\n    Panel plate {\n        Inner part\n    }\n}\n";
    let inner = "widget Inner {\n    Label word colour=red\n}\n";
    let found = diags(PAGE, &[("widgets/card.gogo", card), ("widgets/inner.gogo", inner)]);
    assert_eq!(place_of(&found, "G2001"), ("widgets/inner.gogo".to_string(), 2));
}

/// A check that runs once the whole tree stands (a margin on a child of
/// a Column) names the file the child was written in.
#[test]
fn a_check_of_the_built_tree_names_the_file_the_node_is_in() {
    let card = "widget Card {\n    Column plate {\n        Label caption text=\"x\", margin_left=4\n    }\n}\n";
    let found = diags(PAGE, &[("widgets/card.gogo", card)]);
    assert_eq!(place_of(&found, "G1014"), ("widgets/card.gogo".to_string(), 3));
}

#[test]
fn the_words_of_a_diagnostic_say_the_file_before_the_line() {
    let card = "widget Card {\n    Panel plate {\n        Label caption colour=red\n    }\n}\n";
    let found = diags(PAGE, &[("widgets/card.gogo", card)]);
    let shown: String = found.iter().find(|d| d.code == "G2001").map(|d| d.to_string()).unwrap_or_default();
    assert!(shown.starts_with("error G2001 widgets/card.gogo line 3: "), "{shown}");
}
