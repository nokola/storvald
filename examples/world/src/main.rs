//! A 3D world under a page's nodes, on TurboDot: a page whose own GPU
//! calls draw a spinning cube in a `Surface draw=gpu, depth=true`, with a
//! label and buttons over it.
//!
//! - `world_page`: the cube. A drag turns it, the wheel moves the
//!   camera, "Look around" captures the mouse.
//! - `pause_page`: overlay page. Esc opens it while the mouse is
//!   captured, and the mouse is free while it is on top.
//!
//! One `Surface` that says `depth=true` gives every frame of the program
//! a depth buffer. A program with none never makes one.
//!
//! Run it with `cargo run -p turbodot-example-world`.

mod pause_page;
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
    /// Camera's distance from the cube's center.
    pub camera_distance: f32,
    /// True from "Look around" to "Stop looking": the world's Surface
    /// says `capture_mouse=true`.
    pub is_looking: bool,
    /// True while a button of the mouse is down on the world.
    pub is_mouse_down: bool,
}

impl Default for AppState {
    fn default() -> AppState {
        AppState { spin_rad: 0.0, tilt_rad: START_TILT_RAD, is_spinning: true, camera_distance: world_page::START_CAMERA_DISTANCE, is_looking: false, is_mouse_down: false }
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
    use turbodot::prelude::*;

    use super::*;
    use crate::pause_page::PausePage;
    use crate::ui::world_page::NAMES;
    use crate::world_page::{CAMERA_STEP_PER_NOTCH, FACE_COLORS, SPIN_RAD_PER_SEC, START_CAMERA_DISTANCE, TURN_RAD_PER_MOVE};

    const PAUSE: crate::ui::pause_page::Names = crate::ui::pause_page::NAMES;

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
        let mut program: Program<AppState> = Program::new(&crate::ui::BUILT, AppState { tilt_rad: 0.0, is_spinning: false, ..AppState::default() }, WorldPage::default()).expect("the page can be shown");
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
            assert_eq!(picture.at(button.x as u32 - 4, middle_y), NEAR_FACE, "and the cube shows between it and the button beside it");
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

    /// One frame, a sixtieth of a second after the last.
    fn frame(program: &mut Program<AppState>) {
        program.frame(SIZE_EPX.0, SIZE_EPX.1, 1.0 / 60.0);
    }

    /// Frames until no page is on its way in or out any more.
    fn let_the_pages_arrive(program: &mut Program<AppState>) {
        for _ in 0..120 {
            frame(program);
            if !program.is_page_arriving_or_leaving() {
                return;
            }
        }
        panic!("a page is still on its way after two seconds");
    }

    /// A place on the world beside the buttons, in epx.
    const ON_THE_WORLD: (f32, f32) = (100.0, 200.0);

    /// Program whose cube stands still, with the mouse captured: "Look
    /// around" was tapped and a frame went by.
    fn started_looking() -> Program<AppState> {
        let mut program: Program<AppState> = started_still();
        assert!(!program.is_mouse_captured());
        program.tap(NAMES.look, 1.0);
        assert!(program.app().is_looking);
        assert!(!program.is_mouse_captured(), "the mouse is captured from the next frame on");
        frame(&mut program);
        assert!(program.is_mouse_captured());
        program
    }

    /// "Look around" captures the mouse for the world: every move the
    /// mouse reports turns the cube, in the mouse's own counts, and a
    /// move of the cursor does nothing. A free mouse that passes over the
    /// world turns nothing.
    #[test]
    fn looking_captures_the_mouse_and_its_moves_turn_the_cube() {
        let mut program: Program<AppState> = started_still();
        assert!(program.mouse_moved(ON_THE_WORLD.0, ON_THE_WORLD.1, 0.5, Mods::NONE));
        assert!(program.mouse_moved(ON_THE_WORLD.0 + 30.0, ON_THE_WORLD.1, 0.6, Mods::NONE));
        assert_eq!((program.app().spin_rad, program.app().tilt_rad), (0.0, 0.0), "a free mouse over the world turns nothing");
        assert!(!program.captured_mouse_moved(40.0, 0.0, 0.7, Mods::NONE), "no Surface has the mouse yet");

        let mut program: Program<AppState> = started_looking();
        assert!(program.ui().is_shown() && program.ui().rect(NAMES.hint).h > 0.0, "the hint names the key that pauses");
        assert!(program.captured_mouse_moved(40.0, -20.0, 2.0, Mods::NONE));
        assert!(program.captured_mouse_moved(10.0, 0.0, 2.01, Mods::NONE));
        assert!((program.app().spin_rad - 50.0 * TURN_RAD_PER_MOVE).abs() < 1e-6, "each move turned the cube: {}", program.app().spin_rad);
        assert!((program.app().tilt_rad + 20.0 * TURN_RAD_PER_MOVE).abs() < 1e-6);
        let spin_rad: f32 = program.app().spin_rad;
        // Two moves of the cursor over the world, as a host that still
        // hands them on would: a captured mouse has no place.
        assert!(program.mouse_moved(ON_THE_WORLD.0, ON_THE_WORLD.1, 2.1, Mods::NONE));
        assert!(program.mouse_moved(ON_THE_WORLD.0 + 30.0, ON_THE_WORLD.1, 2.2, Mods::NONE));
        assert_eq!(program.app().spin_rad, spin_rad, "the cursor's moves turn nothing while the mouse is captured");
    }

    /// With a free mouse, a drag on the world turns the cube by its move
    /// in epx, and the wheel over the world moves the camera. Captured,
    /// the buttons and the wheel reach the world wherever the cursor was.
    #[test]
    fn a_drag_turns_the_cube_and_the_wheel_moves_the_camera() {
        let mut program: Program<AppState> = started_still();
        let (x, y) = ON_THE_WORLD;
        assert!(program.mouse_button(Buttons::PRIMARY, true, x, y, 1.0, Mods::NONE));
        assert!(program.app().is_mouse_down);
        assert!(program.mouse_moved(x + 20.0, y, 1.1, Mods::NONE));
        assert!((program.app().spin_rad - 20.0 * TURN_RAD_PER_MOVE).abs() < 1e-6);
        assert!(program.mouse_button(Buttons::PRIMARY, false, x + 20.0, y, 1.2, Mods::NONE));
        assert!(!program.app().is_mouse_down);
        assert!(program.wheel(x, y, 2.0, Mods::NONE));
        assert_eq!(program.app().camera_distance, START_CAMERA_DISTANCE - 2.0 * CAMERA_STEP_PER_NOTCH);

        // Captured while the cursor was on the "Look around" button: a
        // button of the mouse and the wheel go to the world all the same.
        let mut program: Program<AppState> = started_looking();
        let look: Rect = program.ui().rect(NAMES.look);
        assert!(program.mouse_button(Buttons::PRIMARY, true, look.x + 5.0, look.y + 5.0, 3.0, Mods::NONE));
        assert!(program.app().is_mouse_down && program.app().is_looking, "the press went to the world, not to the button under the cursor");
        assert!(program.captured_mouse_moved(8.0, 0.0, 3.1, Mods::NONE));
        assert!((program.app().spin_rad - 8.0 * TURN_RAD_PER_MOVE).abs() < 1e-6, "a drag of a captured mouse");
        assert!(program.mouse_button(Buttons::PRIMARY, false, look.x + 5.0, look.y + 5.0, 3.2, Mods::NONE));
        assert!(!program.app().is_mouse_down);
        assert!(program.wheel(look.x + 5.0, look.y + 5.0, -1.0, Mods::NONE));
        assert_eq!(program.app().camera_distance, START_CAMERA_DISTANCE + CAMERA_STEP_PER_NOTCH);
    }

    /// Esc while the mouse is captured opens the pause page. The mouse is
    /// free while that page is on top, though the world still says
    /// `capture_mouse=true`, and the world has it again when the page
    /// leaves: no page's code frees or captures it.
    #[test]
    fn a_page_over_the_world_frees_the_mouse_until_it_leaves() {
        let mut program: Program<AppState> = started_looking();
        assert!(program.back(), "the world page uses the back request");
        assert!(program.is_drawn::<PausePage>());
        frame(&mut program);
        assert!(!program.is_mouse_captured(), "the pause page is on top: the user has their cursor for its buttons");
        assert!(program.app().is_looking);
        let_the_pages_arrive(&mut program);
        assert!(!program.captured_mouse_moved(10.0, 0.0, 5.0, Mods::NONE));

        program.tap(PAUSE.resume, 6.0);
        frame(&mut program);
        assert!(program.is_mouse_captured(), "the world is the top page again and has the mouse");
        let_the_pages_arrive(&mut program);
        assert!(program.is_mouse_captured());

        // "Stop looking" on the pause page: the world's Surface says
        // false, and the mouse stays free after the page left.
        assert!(program.back());
        let_the_pages_arrive(&mut program);
        program.tap(PAUSE.stop_looking, 8.0);
        let_the_pages_arrive(&mut program);
        assert!(!program.is_mouse_captured() && !program.app().is_looking);
        assert!(!program.back(), "with a free mouse Esc is the plain back request, and the first page has none before it");
    }

    /// A window that loses the keyboard frees the mouse, and has it again
    /// with the keyboard. A button that was down when the mouse was lost
    /// is cancelled for the world.
    #[test]
    fn a_window_without_the_keyboard_frees_the_mouse() {
        let mut program: Program<AppState> = started_looking();
        assert!(program.mouse_button(Buttons::PRIMARY, true, 0.0, 0.0, 2.0, Mods::NONE));
        assert!(program.app().is_mouse_down);
        program.set_window_focused(false);
        assert!(program.is_mouse_captured(), "as the last frame found it");
        frame(&mut program);
        assert!(!program.is_mouse_captured());
        assert!(!program.app().is_mouse_down, "the world heard that its press was cancelled");
        program.set_window_focused(true);
        frame(&mut program);
        assert!(program.is_mouse_captured());
    }

    /// When the world gets the mouse the cursor is gone: the button it
    /// was over shows no hover any more, and a press the free mouse held
    /// is cancelled.
    #[test]
    fn a_captured_mouse_leaves_no_hover_and_no_press_behind() {
        use turbodot::gogo::runtime::HOVERED;
        let mut program: Program<AppState> = started_still();
        let look: Rect = program.ui().rect(NAMES.look);
        let (x, y): (f32, f32) = (look.x + look.w / 2.0, look.y + look.h / 2.0);
        program.mouse_moved(x, y, 0.5, Mods::NONE);
        assert!(program.ui().runtime.has(NAMES.look, HOVERED), "the free mouse is over the button");
        program.mouse_button(Buttons::PRIMARY, true, x, y, 1.0, Mods::NONE);
        program.mouse_button(Buttons::PRIMARY, false, x, y, 1.05, Mods::NONE);
        assert!(program.app().is_looking, "the button was pressed");
        frame(&mut program);
        assert!(program.is_mouse_captured());
        assert!(!program.ui().runtime.has(NAMES.look, HOVERED), "no cursor, no hover");

        // A drag of the free mouse is under way when play starts.
        let mut program: Program<AppState> = started_still();
        program.mouse_button(Buttons::PRIMARY, true, ON_THE_WORLD.0, ON_THE_WORLD.1, 1.0, Mods::NONE);
        assert!(program.app().is_mouse_down);
        assert!(program.tell::<WorldPage>(|world, host| world.set_looking(host, true)));
        frame(&mut program);
        assert!(program.is_mouse_captured());
        assert!(!program.app().is_mouse_down, "the press the free mouse held was cancelled");
    }
}
