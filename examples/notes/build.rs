//! Compiles every page in `ui/` when the program is built.

fn main() {
    turbodot::build::pages("ui");
}
