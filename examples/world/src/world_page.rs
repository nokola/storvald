//! Page with a 3D world under its nodes: a cube the page draws with GPU
//! calls of its own, tested against a depth buffer. The world takes the
//! mouse: a drag turns the cube, the wheel moves the camera, and "Look
//! around" captures the mouse.
//! Picture: `ui/world_page.gogo`.

use turbodot::prelude::*;
use turbodot::wgpu;

use crate::AppState;
use crate::pause_page::PausePage;
use crate::ui::world_page::{Handler, NAMES, PAGE};

/// How fast the cube turns while it spins.
pub const SPIN_RAD_PER_SEC: f32 = 0.8;
/// Camera's angle of view, top to bottom.
const FIELD_OF_VIEW_RAD: f32 = std::f32::consts::FRAC_PI_4;
/// Camera's distance from the cube's center when the program starts. The
/// cube's faces are 1 from its center.
pub const START_CAMERA_DISTANCE: f32 = 5.0;
/// Nearest and farthest the wheel moves the camera.
pub const NEAREST_CAMERA_DISTANCE: f32 = 3.0;
pub const FARTHEST_CAMERA_DISTANCE: f32 = 12.0;
/// How far one notch of the wheel moves the camera.
pub const CAMERA_STEP_PER_NOTCH: f32 = 0.5;
/// How far the cube turns for a move of 1: one epx of a drag, or one
/// count of a captured mouse.
pub const TURN_RAD_PER_MOVE: f32 = 0.005;
/// Nearest and farthest distance from the camera that is drawn.
const NEAR: f32 = 0.5;
const FAR: f32 = 20.0;

/// Cube's faces in the order they are drawn, each with its color (sRGB
/// bytes, RGBA): the face toward the camera first, the face away from it
/// last.
///
/// Needed in this order to show what the depth buffer does: drawn last,
/// the far face would cover the near one. With `depth=true` the near one
/// stays.
pub const FACE_COLORS: [[u8; 4]; 6] = [
    [0xE0, 0x50, 0x40, 0xFF], // toward the camera
    [0x40, 0xA0, 0xE0, 0xFF], // right
    [0x40, 0xC0, 0x80, 0xFF], // left
    [0xE0, 0xC0, 0x40, 0xFF], // top
    [0xA0, 0x60, 0xD0, 0xFF], // bottom
    [0xF0, 0xF0, 0xF0, 0xFF], // away from the camera
];

/// Cube's shader: 36 vertices made from their index, six per face, moved
/// by `world.transform`. The colors are `FACE_COLORS`.
fn cube_shader() -> String {
    let colors: String = FACE_COLORS.iter().map(|[r, g, b, _]| format!("vec3<f32>({:?}, {:?}, {:?})", *r as f32 / 255.0, *g as f32 / 255.0, *b as f32 / 255.0)).collect::<Vec<String>>().join(", ");
    format!(
        "
struct World {{
    transform: mat4x4<f32>,
}}
@group(0) @binding(0) var<uniform> world: World;

struct Out {{
    @builtin(position) position: vec4<f32>,
    @location(0) color: vec3<f32>,
}}

@vertex
fn vs_main(@builtin(vertex_index) index: u32) -> Out {{
    // Each face looks along one of these, in the order of the colors.
    let normals = array<vec3<f32>, 6>(vec3<f32>(0.0, 0.0, 1.0), vec3<f32>(1.0, 0.0, 0.0), vec3<f32>(-1.0, 0.0, 0.0), vec3<f32>(0.0, 1.0, 0.0), vec3<f32>(0.0, -1.0, 0.0), vec3<f32>(0.0, 0.0, -1.0));
    let colors = array<vec3<f32>, 6>({colors});
    // A face's four corners as two triangles.
    let corners = array<vec2<f32>, 6>(vec2<f32>(-1.0, -1.0), vec2<f32>(1.0, -1.0), vec2<f32>(-1.0, 1.0), vec2<f32>(-1.0, 1.0), vec2<f32>(1.0, -1.0), vec2<f32>(1.0, 1.0));
    let face = index / 6u;
    let normal = normals[face];
    let across = vec3<f32>(normal.y, normal.z, normal.x);
    let up = cross(normal, across);
    let corner = corners[index % 6u];
    var out: Out;
    out.position = world.transform * vec4<f32>(normal + across * corner.x + up * corner.y, 1.0);
    out.color = colors[face];
    return out;
}}

@fragment
fn fs_main(in: Out) -> @location(0) vec4<f32> {{
    return vec4<f32>(in.color, 1.0);
}}
"
    )
}

/// What the page keeps on the GPU for its cube.
struct WorldGpu {
    /// Formats of the pass the pipeline was made for: its color target's
    /// and its depth buffer's.
    made_for: (wgpu::TextureFormat, Option<wgpu::TextureFormat>),
    pipeline: wgpu::RenderPipeline,
    /// Cube's transform, written each frame.
    transform: wgpu::Buffer,
    bind_group: wgpu::BindGroup,
}

impl WorldGpu {
    /// Pipeline and buffer for a pass with `made_for`'s formats.
    ///
    /// With a depth format the pipeline keeps what is nearer and writes
    /// its depth. It names the format because the GPU takes a pipeline
    /// only in a pass whose depth buffer has the format the pipeline
    /// names.
    fn new(device: &wgpu::Device, made_for: (wgpu::TextureFormat, Option<wgpu::TextureFormat>)) -> WorldGpu {
        let (target_format, depth_format) = made_for;
        let module: wgpu::ShaderModule = device.create_shader_module(wgpu::ShaderModuleDescriptor { label: Some("cube"), source: wgpu::ShaderSource::Wgsl(cube_shader().into()) });
        let pipeline: wgpu::RenderPipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("cube"),
            layout: None,
            vertex: wgpu::VertexState { module: &module, entry_point: Some("vs_main"), compilation_options: Default::default(), buffers: &[] },
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: depth_format.map(|format| wgpu::DepthStencilState { format, depth_write_enabled: Some(true), depth_compare: Some(wgpu::CompareFunction::Less), stencil: wgpu::StencilState::default(), bias: wgpu::DepthBiasState::default() }),
            multisample: wgpu::MultisampleState::default(),
            fragment: Some(wgpu::FragmentState { module: &module, entry_point: Some("fs_main"), compilation_options: Default::default(), targets: &[Some(wgpu::ColorTargetState { format: target_format, blend: None, write_mask: wgpu::ColorWrites::ALL })] }),
            multiview_mask: None,
            cache: None,
        });
        let transform: wgpu::Buffer = device.create_buffer(&wgpu::BufferDescriptor { label: Some("cube transform"), size: 64, usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST, mapped_at_creation: false });
        let bind_group: wgpu::BindGroup = device.create_bind_group(&wgpu::BindGroupDescriptor { label: Some("cube"), layout: &pipeline.get_bind_group_layout(0), entries: &[wgpu::BindGroupEntry { binding: 0, resource: transform.as_entire_binding() }] });
        WorldGpu { made_for, pipeline, transform, bind_group }
    }
}

#[derive(Default)]
pub struct WorldPage {
    /// Made at page's first GPU frame, and again for a pass with other
    /// formats.
    gpu: Option<WorldGpu>,
}

impl Page<AppState> for WorldPage {
    const PAGE: &'static BuiltPage = &PAGE;
    type Handler = Handler;

    fn show(&mut self, host: &mut Host<AppState>) {
        host.ui.set_text(NAMES.spin, if host.app.is_spinning { "Stop" } else { "Spin" });
    }

    fn handle(&mut self, host: &mut Host<AppState>, event: PageEvent<Handler>) {
        match event.handler {
            Handler::Spin => {
                host.app.is_spinning = !host.app.is_spinning;
                host.ui.set_text(NAMES.spin, if host.app.is_spinning { "Stop" } else { "Spin" });
            }
            Handler::Look => self.set_looking(host, true),
            // The world's `on:pointer=` and `on:hover=`.
            Handler::Turn => match event.kind {
                // A button is down: a drag turns the cube, with a free
                // mouse (the move is in epx) and with a captured one.
                EventKind::Pointer(phase, sample) => {
                    host.app.is_mouse_down = matches!(phase, Phase::Down | Phase::Move);
                    if phase == Phase::Move {
                        turn(host.app, sample.dx, sample.dy);
                    }
                }
                // No button is down. A free mouse only passes over the
                // world; a captured one turns the cube with every move,
                // and its move is in the mouse's own counts.
                EventKind::Hover(HoverPhase::Over, sample) if host.app.is_looking => turn(host.app, sample.dx, sample.dy),
                _ => {}
            },
            // The world's `on:wheel=`: away from the user moves the
            // camera nearer.
            Handler::Zoom => {
                if let EventKind::Wheel { notches, .. } = event.kind {
                    host.app.camera_distance = (host.app.camera_distance - notches * CAMERA_STEP_PER_NOTCH).clamp(NEAREST_CAMERA_DISTANCE, FARTHEST_CAMERA_DISTANCE);
                }
            }
        }
    }

    /// Esc while the mouse is captured: the pause page opens over the
    /// world, and the mouse is free there with nothing done here. With
    /// a free mouse Esc is the plain back request.
    fn back(&mut self, host: &mut Host<AppState>) -> Back {
        if !host.app.is_looking {
            return Back::Leave;
        }
        host.nav.go(PausePage);
        Back::Stay
    }

    /// Turns the cube while it spins, and asks for frames only then.
    fn frame(&mut self, host: &mut Host<AppState>, dt_sec: f32) -> bool {
        if !host.app.is_spinning {
            return false;
        }
        host.app.spin_rad = (host.app.spin_rad + SPIN_RAD_PER_SEC * dt_sec) % std::f32::consts::TAU;
        true
    }

    /// Each frame before the pages are drawn: makes the cube's pipeline
    /// for the pass's formats, and writes where the cube is now. The
    /// draw calls themselves come later, inside the pass
    /// (`draw_surface_gpu`), and only read.
    fn gpu_prepare(&mut self, host: &mut Host<AppState>, gpu: &Gpu) {
        let Some(target_format) = host.ui.target_format() else { return };
        let made_for: (wgpu::TextureFormat, Option<wgpu::TextureFormat>) = (target_format, host.ui.depth_format());
        if self.gpu.as_ref().is_none_or(|world| world.made_for != made_for) {
            self.gpu = Some(WorldGpu::new(&gpu.device, made_for));
        }
        let Some(world) = &self.gpu else { return };
        let rect = host.ui.rect(NAMES.world.node());
        if rect.h <= 0.0 {
            return;
        }
        // The engine puts the viewport on the Surface, so the picture's
        // shape is the Surface's.
        let transform: [f32; 16] = cube_transform(host.app.spin_rad, host.app.tilt_rad, host.app.camera_distance, rect.w / rect.h);
        let mut bytes: [u8; 64] = [0; 64];
        for (number, four_bytes) in transform.iter().zip(bytes.chunks_exact_mut(4)) {
            four_bytes.copy_from_slice(&number.to_ne_bytes());
        }
        gpu.queue.write_buffer(&world.transform, 0, &bytes);
    }

    /// The cube: 36 vertices. The engine put the viewport on the `world`
    /// Surface and cleared its depth, so nothing here says where on the
    /// screen the world is.
    fn draw_surface_gpu(&self, app: &AppState, surface: GpuSurface, pass: &mut wgpu::RenderPass<'_>, place: &turbodot::SurfacePlace) {
        let _ = (app, surface);
        let Some(world) = &self.gpu else { return };
        if world.made_for != (place.target_format, place.depth_format) {
            return;
        }
        pass.set_pipeline(&world.pipeline);
        pass.set_bind_group(0, &world.bind_group, &[]);
        pass.draw(0..36, 0..1);
    }
}

impl WorldPage {
    /// Captures the mouse for the world, or frees it: the cursor goes,
    /// and every move of the mouse comes to the world's `on:hover=`.
    ///
    /// With no cursor no button can be pressed, so the hint names the
    /// key that pauses.
    pub fn set_looking(&mut self, host: &mut Host<AppState>, is_looking: bool) {
        host.app.is_looking = is_looking;
        host.ui.set_capture_mouse(NAMES.world.node(), is_looking);
        host.ui.set_visible(NAMES.hint, is_looking);
    }
}

/// Turns the cube by a move of (`dx`, `dy`): sideways about its upright
/// axis, up and down toward the camera and away, never past flat.
fn turn(app: &mut AppState, dx: f32, dy: f32) {
    app.spin_rad = (app.spin_rad + dx * TURN_RAD_PER_MOVE) % std::f32::consts::TAU;
    app.tilt_rad = (app.tilt_rad + dy * TURN_RAD_PER_MOVE).clamp(-std::f32::consts::FRAC_PI_2, std::f32::consts::FRAC_PI_2);
}

/// A 4 x 4 matrix, column by column.
type Matrix = [f32; 16];

/// `a` after `b`: what `b` does to a point, then what `a` does.
fn times(a: &Matrix, b: &Matrix) -> Matrix {
    let mut product: Matrix = [0.0; 16];
    for column in 0..4 {
        for row in 0..4 {
            product[column * 4 + row] = (0..4).map(|k| a[k * 4 + row] * b[column * 4 + k]).sum();
        }
    }
    product
}

/// From the cube's own space to clip space: turned `spin_rad` about its
/// upright axis, tipped `tilt_rad` toward the camera, seen from
/// `camera_distance` in perspective. `width_over_height`: the shape of
/// the Surface it is drawn in.
///
/// Depth runs from 0 at `NEAR` to 1 at `FAR`, as a depth buffer cleared
/// to 1 and a pipeline that keeps what is `Less` want it.
pub fn cube_transform(spin_rad: f32, tilt_rad: f32, camera_distance: f32, width_over_height: f32) -> Matrix {
    let (spin_sin, spin_cos) = spin_rad.sin_cos();
    let (tilt_sin, tilt_cos) = tilt_rad.sin_cos();
    let spin: Matrix = [spin_cos, 0.0, -spin_sin, 0.0, 0.0, 1.0, 0.0, 0.0, spin_sin, 0.0, spin_cos, 0.0, 0.0, 0.0, 0.0, 1.0];
    let tilt: Matrix = [1.0, 0.0, 0.0, 0.0, 0.0, tilt_cos, tilt_sin, 0.0, 0.0, -tilt_sin, tilt_cos, 0.0, 0.0, 0.0, 0.0, 1.0];
    let away: Matrix = [1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, -camera_distance, 1.0];
    let zoom: f32 = 1.0 / (FIELD_OF_VIEW_RAD / 2.0).tan();
    let lens: Matrix = [zoom / width_over_height, 0.0, 0.0, 0.0, 0.0, zoom, 0.0, 0.0, 0.0, 0.0, FAR / (NEAR - FAR), -1.0, 0.0, 0.0, NEAR * FAR / (NEAR - FAR), 0.0];
    times(&lens, &times(&away, &times(&tilt, &spin)))
}
