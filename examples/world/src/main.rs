//! A 3D world under a page's nodes, on TurboDot: one page whose own GPU
//! calls draw a spinning cube in a `Surface draw=gpu, depth=true`, with a
//! label and a button over it.
//!
//! One `Surface` that says `depth=true` gives every frame of the program
//! a depth buffer. A program with none never makes one.
//!
//! Run it with `cargo run -p turbodot-example-world`.

mod world_page;

turbodot::built_pages!();

use turbodot::{Program, WindowOptions};
use world_page::WorldPage;

/// Cube's tip toward the camera when the program starts.
const START_TILT_RAD: f32 = 0.5;

/// Program-wide state. The page reaches it as `host.app`.
pub struct AppState {
    /// How far the cube is turned about its upright axis.
    pub spin_rad: f32,
    /// How far the cube is tipped toward the camera.
    pub tilt_rad: f32,
    /// True while the cube turns by itself.
    pub is_spinning: bool,
}

impl Default for AppState {
    fn default() -> AppState {
        AppState { spin_rad: 0.0, tilt_rad: START_TILT_RAD, is_spinning: true }
    }
}

/// Opens window on the world page, stays until window is closed. Debug
/// build: the page follows its `.gogo` file.
///
/// Returns why page or window could not show.
fn main() -> Result<(), String> {
    let program: Program<AppState> = Program::following_page_files(&ui::BUILT, AppState::default(), WorldPage::default())?;
    turbodot::start(WindowOptions { title: "World", width: 960.0, height: 640.0 }, program)
}

#[cfg(test)]
mod tests {
    use turbodot::Program;
    use turbodot::gogo::Rect;

    use super::*;
    use crate::ui::world_page::NAMES;
    use crate::world_page::{FACE_COLORS, SPIN_RAD_PER_SEC};

    /// Size tests lay the page out for, in epx. One device pixel per epx.
    const SIZE_EPX: (f32, f32) = (640.0, 300.0);
    /// The page's `background=`.
    const BACK: [u8; 4] = [0x10, 0x12, 0x18, 0xFF];
    /// The button's `background=`.
    const BUTTON: [u8; 4] = [0x2C, 0x2F, 0x38, 0xFF];
    /// Color of the face toward the camera when the cube is not turned.
    const NEAR_FACE: [u8; 4] = FACE_COLORS[0];
    /// Color of the face away from the camera when the cube is not turned.
    const FAR_FACE: [u8; 4] = FACE_COLORS[5];

    /// Program whose cube stands still, not turned and not tipped, after
    /// its first frame: its near face is square to the camera.
    fn started_still() -> Program<AppState> {
        let mut program: Program<AppState> = Program::new(&crate::ui::BUILT, AppState { spin_rad: 0.0, tilt_rad: 0.0, is_spinning: false }, WorldPage::default()).expect("the page can be shown");
        program.frame(SIZE_EPX.0, SIZE_EPX.1, 0.0);
        program
    }

    /// The build knows that a Surface of the program says `depth=true`,
    /// and the program has a depth format by it.
    #[test]
    fn the_build_says_the_program_has_a_depth_surface() {
        assert!(crate::ui::HAS_DEPTH_SURFACE);
        assert!(crate::ui::BUILT.has_depth_surface);
        let program: Program<AppState> = started_still();
        assert_eq!(program.depth_format(), Some(turbodot::DEPTH_FORMAT));
    }

    /// The cube turns while it spins and the page asks for frames; the
    /// button stops it, and then the page asks for none.
    #[test]
    fn the_button_stops_the_cube() {
        let mut program: Program<AppState> = Program::new(&crate::ui::BUILT, AppState::default(), WorldPage::default()).expect("the page can be shown");
        assert!(program.frame(SIZE_EPX.0, SIZE_EPX.1, 0.5), "a spinning cube needs the next frame");
        assert!((program.app().spin_rad - SPIN_RAD_PER_SEC * 0.5).abs() < 1e-6);
        assert_eq!(program.ui().text_of(NAMES.spin), Some("Stop"));
        program.tap(NAMES.spin, 1.0);
        assert!(!program.app().is_spinning);
        assert_eq!(program.ui().text_of(NAMES.spin), Some("Spin"));
        let spin_rad: f32 = program.app().spin_rad;
        // The tap's own look may still play; the cube does not turn.
        program.frame(SIZE_EPX.0, SIZE_EPX.1, 0.5);
        program.frame(SIZE_EPX.0, SIZE_EPX.1, 0.5);
        assert_eq!(program.app().spin_rad, spin_rad);
    }

    /// The cube's near face hides its far face, though the far face is
    /// drawn last: the page's draw calls are tested against the depth
    /// buffer. The page's nodes draw over the world, and the world draws
    /// over the page's background.
    ///
    /// Ignored unless asked for: needs a GPU adapter.
    #[test]
    #[ignore = "needs a GPU adapter: run with --include-ignored on a machine that has one"]
    fn the_cube_s_near_face_hides_its_far_face() {
        let mut program: Program<AppState> = started_still();
        let gpu: turbodot::Gpu = turbodot::Gpu::new().expect("this test needs a GPU adapter");
        // Twice: the second picture's renderer and pipeline are new ones.
        for round in 0..2 {
            let picture: turbodot::Picture = gpu.draw_program(&mut program).expect("the page is drawn");
            assert_eq!((picture.width, picture.height), (640, 300));
            // Both faces are in front of this pixel: the near one from
            // 60 to 240 down the picture, the far one from 90 to 210.
            assert_eq!(picture.at(320, 180), NEAR_FACE, "the near face shows where the far face was drawn after it, picture {round}");
            assert_ne!(NEAR_FACE, FAR_FACE);
            // Beside the near face (it ends 90 pixels from the middle).
            assert_eq!(picture.at(60, 150), BACK, "the page's background shows beside the cube: the world clears nothing");
            // The button is over the near face.
            let button: Rect = program.ui().rect(NAMES.spin);
            let middle_y: u32 = (button.y + button.h / 2.0) as u32;
            assert_eq!(picture.at(button.x as u32 + 4, middle_y), BUTTON, "the button is drawn over the world, however near the cube is");
            assert_eq!(picture.at(button.x as u32 - 8, middle_y), NEAR_FACE, "and the cube is right beside it");
        }
    }

    /// The cube turned a quarter shows its left face to the camera: the
    /// page writes where the cube is each frame.
    ///
    /// Ignored unless asked for: needs a GPU adapter.
    #[test]
    #[ignore = "needs a GPU adapter: run with --include-ignored on a machine that has one"]
    fn a_turned_cube_shows_another_face() {
        let mut program: Program<AppState> = started_still();
        let gpu: turbodot::Gpu = turbodot::Gpu::new().expect("this test needs a GPU adapter");
        program.app_mut().spin_rad = std::f32::consts::FRAC_PI_2;
        program.frame(SIZE_EPX.0, SIZE_EPX.1, 0.016);
        let picture: turbodot::Picture = gpu.draw_program(&mut program).expect("the page is drawn");
        assert_eq!(picture.at(320, 180), FACE_COLORS[2], "the left face came round to the camera");
    }
}
