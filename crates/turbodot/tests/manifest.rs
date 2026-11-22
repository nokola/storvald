//! What `turbodot` builds when a crate takes it with
//! `default-features = false`.

/// The dependencies the manifest lists without `optional = true`.
fn always_built(manifest: &str) -> Vec<&str> {
    let dependencies: &str = manifest.split("\n[dependencies]\n").nth(1).expect("a [dependencies] table");
    let dependencies: &str = dependencies.split("\n[").next().expect("the table's lines");
    dependencies.lines().filter(|line| !line.starts_with('#') && line.contains('=') && !line.contains("optional = true")).map(|line| line.split('=').next().expect("a name").trim()).collect()
}

/// With no feature on, `turbodot` builds gogo's language and its text
/// engine and nothing else.
///
/// Needed because a crate that holds an app's own code must build with no
/// window and no GPU crate: it then cannot call either by mistake, and its
/// tests build in seconds. One dependency that is not optional would bring
/// its crates into every such build, and nothing else would fail.
#[test]
fn with_no_feature_only_gogos_language_and_text_engine_are_built() {
    // In a crates.io package cargo writes `Cargo.toml` again in its own
    // shape and keeps the file as it was written beside it.
    let as_written: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/Cargo.toml.orig");
    let in_the_repository: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/Cargo.toml");
    let manifest: String = std::fs::read_to_string(as_written).or_else(|_| std::fs::read_to_string(in_the_repository)).expect("the manifest").replace("\r\n", "\n");
    assert_eq!(always_built(&manifest), ["gogo-text", "gogo-ui"]);
}
