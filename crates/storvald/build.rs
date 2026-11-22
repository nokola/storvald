//! Compiles the pages in `ui/`, and puts `storvald.manifest` into the
//! Windows program.
//!
//! Needed because Windows reads a program's manifest from inside the
//! program file, and Storvald is one file with nothing beside it. What the
//! manifest says, and why, is in the manifest.
//!
//! Only Microsoft's linker is told. A build for another system has no
//! manifest, and a Windows build with the GNU linker keeps the console
//! window: nothing here teaches that linker to take the file.

use std::path::PathBuf;

fn main() {
    turbodot::build::pages("ui");
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=storvald.manifest");
    let target_os: String = std::env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();
    let target_env: String = std::env::var("CARGO_CFG_TARGET_ENV").unwrap_or_default();
    if target_os != "windows" || target_env != "msvc" {
        return;
    }
    let manifest: PathBuf = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").expect("cargo names the crate's folder")).join("storvald.manifest");
    // For the `storvald` program only: the tests' own programs get no
    // manifest.
    println!("cargo:rustc-link-arg-bins=/MANIFEST:EMBED");
    println!("cargo:rustc-link-arg-bins=/MANIFESTINPUT:{}", manifest.display());
}
