//! What a client's `build.rs` calls.

use std::fmt::Write as _;
use std::path::PathBuf;

use gogo_ui::BuildOptions;

/// File `pages` writes into build's output folder. `built_pages!` includes
/// it.
pub const BUILT_PAGES_FILE: &str = "turbodot_pages.rs";

/// Compiles every `.gogo` page in `folder` (folder of the crate being
/// built) into that crate's `ui` module. One line in `build.rs`:
///
/// ```text
/// fn main() { turbodot::build::pages("ui"); }
/// ```
///
/// Needed so a page costs nothing at program start, and a mistake in a
/// page stops build with file and line.
///
/// Path a page writes as `res://assets/x.ttf` is read from crate's own
/// folder: `<crate>/assets/x.ttf`.
///
/// WARNING: stops build script with exit code 1 when a page does not
/// compile, after printing gogo's errors.
pub fn pages(folder: &str) {
    if let Err(why) = write_pages(folder) {
        eprintln!("{why}");
        std::process::exit(1);
    }
}

/// Writes built pages, then `BUILT`, which a `Program` starts from.
///
/// Returns gogo's errors, or that cargo named no crate folder or no output
/// folder.
fn write_pages(folder: &str) -> Result<(), String> {
    let crate_dir: PathBuf = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").map_err(|_| "`turbodot::build::pages` is called from a build script: cargo names no crate folder here")?);
    let out: PathBuf = PathBuf::from(std::env::var("OUT_DIR").map_err(|_| "`turbodot::build::pages` is called from a build script: cargo names no output folder here")?).join(BUILT_PAGES_FILE);
    let dir: PathBuf = crate_dir.join(folder);
    // Page added to folder: cargo builds program again.
    println!("cargo:rerun-if-changed={}", dir.display());
    gogo_ui::build_pages(&dir, &out, BuildOptions { gogo_path: "turbodot::gogo", ..BuildOptions::new(&crate_dir) })?;
    let mut source: String = std::fs::read_to_string(&out).map_err(|e| format!("{}: {e}", out.display()))?;
    let folder_as_text: String = dir.display().to_string().replace('\\', "/");
    let _ = write!(source, "/// What `turbodot::Program` starts from: this build's fonts and tables, and the folder its pages came from.\npub static BUILT: turbodot::Built = turbodot::Built {{ fonts: FONTS, tables: &BUILD, release_unused: RELEASE_UNUSED, folder: {folder_as_text:?}, has_depth_surface: HAS_DEPTH_SURFACE }};\n");
    std::fs::write(&out, source).map_err(|e| format!("{}: {e}", out.display()))
}

/// Declares module `ui` with pages built by `turbodot::build::pages`. One
/// line at top of a crate's `lib.rs` or `main.rs`:
///
/// ```text
/// turbodot::built_pages!();
/// ```
///
/// Each page is a module named after its file:
/// `ui::home_page::{PAGE, NAMES, Handler}` for `home_page.gogo`.
#[macro_export]
macro_rules! built_pages {
    () => {
        #[allow(dead_code, unused_imports, clippy::all)]
        pub mod ui {
            include!(concat!(env!("OUT_DIR"), "/turbodot_pages.rs"));
        }
    };
}
