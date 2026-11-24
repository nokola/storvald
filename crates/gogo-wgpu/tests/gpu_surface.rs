//! A `Surface draw=gpu` in the renderer: the program's own draw calls run
//! at the Surface's place in the pass, with the viewport on its rectangle
//! and the scissor on the part of it that shows.
//!
//! Every test here needs a GPU adapter and is ignored unless asked for.

use gogo_text::TextEngine;
use gogo_ui::layout::Rect;
use gogo_ui::{Color_Straight_Srgb_4xU8_RGBA, Draw, GpuSurface, Page, compile, has_errors};
use gogo_wgpu::{SurfacePlace, UiRenderer};

const FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8Unorm;
/// The target, in device pixels.
const TARGET: (u32, u32) = (200, 160);
/// Device pixels per epx in these tests.
const EPX: f32 = 2.0;

/// A device on any backend built in (`WGPU_BACKEND` picks one, as in
/// wgpu's own tools).
fn device() -> (wgpu::Device, wgpu::Queue) {
    let mut desc = wgpu::InstanceDescriptor::new_without_display_handle();
    if let Some(backends) = wgpu::Backends::from_env() {
        desc.backends = backends;
    }
    let instance = wgpu::Instance::new(desc);
    let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions::default())).expect("this test needs a GPU adapter");
    pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor::default())).expect("a device of that adapter")
}

/// A page with one `Surface draw=gpu`, and that Surface.
fn page_with_a_gpu_surface() -> (Page, GpuSurface) {
    let compiled = compile("View v { Surface world draw=gpu, width=10, height=10 }", &|_| None);
    assert!(!has_errors(&compiled.diags), "{:#?}", compiled.diags);
    let surface: GpuSurface = GpuSurface::find(&compiled.page, "world").expect("a Surface draw=gpu");
    (compiled.page, surface)
}

/// Prepares `draws` and draws them into a target of `TARGET`. Returns
/// each place `render_with` handed out, in order.
fn places_of(draws: &[Draw], page: &Page) -> Vec<SurfacePlace> {
    let (device, queue) = device();
    let mut ui: UiRenderer = UiRenderer::new(&device, FORMAT, None);
    let mut text: TextEngine = TextEngine::new(&[]);
    ui.prepare_pages(&device, &queue, &[(draws, page)], EPX, TARGET, &mut text, None);
    let texture = device.create_texture(&wgpu::TextureDescriptor { label: Some("target"), size: wgpu::Extent3d { width: TARGET.0, height: TARGET.1, depth_or_array_layers: 1 }, mip_level_count: 1, sample_count: 1, dimension: wgpu::TextureDimension::D2, format: FORMAT, usage: wgpu::TextureUsages::RENDER_ATTACHMENT, view_formats: &[] });
    let view = texture.create_view(&Default::default());
    let mut encoder = device.create_command_encoder(&Default::default());
    let mut places: Vec<SurfacePlace> = Vec::new();
    {
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("target"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment { view: &view, depth_slice: None, resolve_target: None, ops: wgpu::Operations { load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT), store: wgpu::StoreOp::Store } })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
            multiview_mask: None,
        });
        ui.render_with(&mut pass, &mut |place, _pass| places.push(*place));
    }
    queue.submit([encoder.finish()]);
    places
}

fn quad(rect: Rect) -> Draw {
    Draw::Quad { rect, color: Color_Straight_Srgb_4xU8_RGBA::opaque(0xFFFFFF), corner: 0.0, border: 0.0, border_color: Color_Straight_Srgb_4xU8_RGBA::TRANSPARENT }
}

/// The code is handed the Surface's rectangle in device pixels and the
/// part of it that shows: what clips the Surface cuts it, and so does the
/// target's edge. A Surface of which nothing shows is not drawn.
#[test]
#[ignore = "needs a GPU adapter: run with --include-ignored on a machine that has one"]
fn a_gpu_surface_is_handed_its_rectangle_and_the_part_that_shows() {
    let (page, surface) = page_with_a_gpu_surface();
    let node = surface.node();
    let draws: Vec<Draw> = vec![
        quad(Rect::new(0.0, 0.0, 100.0, 80.0)),
        // Whole inside the target.
        Draw::GpuSurface { rect: Rect::new(10.0, 20.0, 30.0, 40.0), surface: node, alpha: 0.5 },
        // Past the target's left and top edge: a page that slides in.
        Draw::GpuSurface { rect: Rect::new(-20.0, -5.0, 50.0, 30.0), surface: node, alpha: 1.0 },
        // Inside a clip that cuts its right half.
        Draw::ClipPush(Rect::new(40.0, 0.0, 20.0, 80.0)),
        Draw::GpuSurface { rect: Rect::new(50.0, 10.0, 20.0, 20.0), surface: node, alpha: 1.0 },
        // Inside the same clip, and wholly outside it: not drawn.
        Draw::GpuSurface { rect: Rect::new(70.0, 10.0, 20.0, 20.0), surface: node, alpha: 1.0 },
        Draw::ClipPop,
        // Wholly past the target's right edge: not drawn.
        Draw::GpuSurface { rect: Rect::new(120.0, 10.0, 20.0, 20.0), surface: node, alpha: 1.0 },
        quad(Rect::new(0.0, 0.0, 10.0, 10.0)),
    ];
    let places: Vec<SurfacePlace> = places_of(&draws, &page);
    assert_eq!(places.len(), 3, "{places:#?}");
    for place in &places {
        assert!(place.is_of(&page));
        assert_eq!((place.surface, place.epx, place.target_format), (node, EPX, FORMAT));
    }
    assert_eq!((places[0].rect_px, places[0].visible_px, places[0].opacity), ([20.0, 40.0, 60.0, 80.0], [20, 40, 60, 80], 0.5));
    assert_eq!((places[1].rect_px, places[1].visible_px), ([-40.0, -10.0, 100.0, 60.0], [0, 0, 60, 50]), "the viewport reaches past the target, the scissor stops at its edge");
    assert_eq!((places[2].rect_px, places[2].visible_px), ([100.0, 20.0, 40.0, 40.0], [100, 20, 20, 40]), "the clip around the Surface cuts it");
    let (other_page, _) = page_with_a_gpu_surface();
    assert!(!places[0].is_of(&other_page), "a place is of one page");
}

/// `render` draws no `draw=gpu` Surface, so a debug build stops when the
/// frame has one: the user would see an empty place and no reason.
#[test]
#[ignore = "needs a GPU adapter: run with --include-ignored on a machine that has one"]
#[cfg_attr(debug_assertions, should_panic(expected = "call `render_with`"))]
fn render_without_the_surfaces_draw_calls_stops_a_debug_build() {
    let (page, surface) = page_with_a_gpu_surface();
    let draws: Vec<Draw> = vec![Draw::GpuSurface { rect: Rect::new(10.0, 20.0, 30.0, 40.0), surface: surface.node(), alpha: 1.0 }];
    let (device, queue) = device();
    let mut ui: UiRenderer = UiRenderer::new(&device, FORMAT, None);
    let mut text: TextEngine = TextEngine::new(&[]);
    ui.prepare_pages(&device, &queue, &[(draws.as_slice(), &page)], EPX, TARGET, &mut text, None);
    let texture = device.create_texture(&wgpu::TextureDescriptor { label: Some("target"), size: wgpu::Extent3d { width: TARGET.0, height: TARGET.1, depth_or_array_layers: 1 }, mip_level_count: 1, sample_count: 1, dimension: wgpu::TextureDimension::D2, format: FORMAT, usage: wgpu::TextureUsages::RENDER_ATTACHMENT, view_formats: &[] });
    let view = texture.create_view(&Default::default());
    let mut encoder = device.create_command_encoder(&Default::default());
    let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
        label: Some("target"),
        color_attachments: &[Some(wgpu::RenderPassColorAttachment { view: &view, depth_slice: None, resolve_target: None, ops: wgpu::Operations { load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT), store: wgpu::StoreOp::Store } })],
        depth_stencil_attachment: None,
        timestamp_writes: None,
        occlusion_query_set: None,
        multiview_mask: None,
    });
    ui.render(&mut pass);
}

/// A `draw=gpu` Surface that is turned stops a debug build with the
/// Surface's name: a viewport is an upright rectangle, so the drawing
/// would stay upright while everything around it turns.
#[test]
#[ignore = "needs a GPU adapter: run with --include-ignored on a machine that has one"]
#[cfg_attr(debug_assertions, should_panic(expected = "Surface `world` is `draw=gpu` and is turned"))]
fn a_turned_gpu_surface_stops_a_debug_build() {
    let (page, surface) = page_with_a_gpu_surface();
    let draws: Vec<Draw> = vec![Draw::RotatePush { angle: 30.0, cx: 50.0, cy: 40.0 }, Draw::GpuSurface { rect: Rect::new(10.0, 20.0, 30.0, 40.0), surface: surface.node(), alpha: 1.0 }, Draw::RotatePop];
    places_of(&draws, &page);
}

/// Two triangles over the left half of clip space, in one color.
const FILL_SHADER: &str = "
@vertex
fn vs_main(@builtin(vertex_index) index: u32) -> @builtin(position) vec4<f32> {
    let corners = array<vec2<f32>, 6>(vec2<f32>(-1.0, -1.0), vec2<f32>(0.0, -1.0), vec2<f32>(-1.0, 1.0), vec2<f32>(-1.0, 1.0), vec2<f32>(0.0, -1.0), vec2<f32>(0.0, 1.0));
    return vec4<f32>(corners[index], 0.0, 1.0);
}

@fragment
fn fs_main() -> @location(0) vec4<f32> {
    return vec4<f32>(0.0, 1.0, 0.0, 1.0);
}
";

const GREEN: [u8; 4] = [0, 255, 0, 255];
const WHITE: [u8; 4] = [255, 255, 255, 255];
const CLEAR: [u8; 4] = [0, 0, 0, 0];

/// Code that fills the left half of clip space fills the left half of
/// its Surface and nothing else: the viewport is on the Surface's
/// rectangle, and the scissor cuts what a clip around the Surface cuts.
/// What the pages draw after the Surface is drawn over it, in its own
/// place and with the renderer's own pipeline: the renderer set its
/// state again.
#[test]
#[ignore = "needs a GPU adapter: run with --include-ignored on a machine that has one"]
fn code_draws_in_a_viewport_on_its_surface() {
    let (page, surface) = page_with_a_gpu_surface();
    let node = surface.node();
    let draws: Vec<Draw> = vec![
        // A quad of the page before the Surfaces: the renderer's own
        // pipeline is set when the code's calls start.
        quad(Rect::new(0.0, 0.0, 2.0, 2.0)),
        // In epx; a device pixel is half of one.
        Draw::GpuSurface { rect: Rect::new(10.0, 10.0, 30.0, 20.0), surface: node, alpha: 1.0 },
        // A clip that keeps the left quarter of the second Surface.
        Draw::ClipPush(Rect::new(50.0, 40.0, 10.0, 30.0)),
        Draw::GpuSurface { rect: Rect::new(50.0, 40.0, 40.0, 30.0), surface: node, alpha: 1.0 },
        Draw::ClipPop,
        // A quad of the page over the first Surface's top left corner.
        quad(Rect::new(10.0, 10.0, 5.0, 5.0)),
    ];
    let (device, queue) = device();
    let mut ui: UiRenderer = UiRenderer::new(&device, FORMAT, None);
    let mut text: TextEngine = TextEngine::new(&[]);
    ui.prepare_pages(&device, &queue, &[(draws.as_slice(), &page)], EPX, TARGET, &mut text, None);
    let module = device.create_shader_module(wgpu::ShaderModuleDescriptor { label: Some("fill"), source: wgpu::ShaderSource::Wgsl(FILL_SHADER.into()) });
    let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("fill"),
        layout: None,
        vertex: wgpu::VertexState { module: &module, entry_point: Some("vs_main"), compilation_options: Default::default(), buffers: &[] },
        primitive: wgpu::PrimitiveState::default(),
        depth_stencil: None,
        multisample: wgpu::MultisampleState::default(),
        fragment: Some(wgpu::FragmentState { module: &module, entry_point: Some("fs_main"), compilation_options: Default::default(), targets: &[Some(wgpu::ColorTargetState { format: FORMAT, blend: None, write_mask: wgpu::ColorWrites::ALL })] }),
        multiview_mask: None,
        cache: None,
    });
    let texture = device.create_texture(&wgpu::TextureDescriptor { label: Some("target"), size: wgpu::Extent3d { width: TARGET.0, height: TARGET.1, depth_or_array_layers: 1 }, mip_level_count: 1, sample_count: 1, dimension: wgpu::TextureDimension::D2, format: FORMAT, usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC, view_formats: &[] });
    let view = texture.create_view(&Default::default());
    let mut encoder = device.create_command_encoder(&Default::default());
    {
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("target"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment { view: &view, depth_slice: None, resolve_target: None, ops: wgpu::Operations { load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT), store: wgpu::StoreOp::Store } })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
            multiview_mask: None,
        });
        ui.render_with(&mut pass, &mut |_place, pass| {
            pass.set_pipeline(&pipeline);
            pass.draw(0..6, 0..1);
        });
    }
    // The GPU copies rows at a fixed byte step, wider than a row here.
    let row_bytes: u32 = (TARGET.0 * 4).div_ceil(wgpu::COPY_BYTES_PER_ROW_ALIGNMENT) * wgpu::COPY_BYTES_PER_ROW_ALIGNMENT;
    let buffer = device.create_buffer(&wgpu::BufferDescriptor { label: Some("readback"), size: (row_bytes * TARGET.1) as u64, usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ, mapped_at_creation: false });
    encoder.copy_texture_to_buffer(
        wgpu::TexelCopyTextureInfo { texture: &texture, mip_level: 0, origin: wgpu::Origin3d::ZERO, aspect: wgpu::TextureAspect::All },
        wgpu::TexelCopyBufferInfo { buffer: &buffer, layout: wgpu::TexelCopyBufferLayout { offset: 0, bytes_per_row: Some(row_bytes), rows_per_image: Some(TARGET.1) } },
        wgpu::Extent3d { width: TARGET.0, height: TARGET.1, depth_or_array_layers: 1 },
    );
    queue.submit([encoder.finish()]);
    let slice = buffer.slice(..);
    slice.map_async(wgpu::MapMode::Read, |_| {});
    device.poll(wgpu::PollType::Wait { submission_index: None, timeout: Some(std::time::Duration::from_secs(10)) }).expect("the GPU answers within ten seconds");
    let pixels: Vec<u8> = slice.get_mapped_range().expect("the read is mapped").to_vec();
    let at = |x: u32, y: u32| -> [u8; 4] {
        let i: usize = (y * row_bytes + x * 4) as usize;
        [pixels[i], pixels[i + 1], pixels[i + 2], pixels[i + 3]]
    };

    // The first Surface: device pixels 20..80 by 20..60. Its left half
    // is 20..50.
    assert_eq!((at(30, 40), at(49, 59), at(20, 59)), (GREEN, GREEN, GREEN), "the left half of clip space is the left half of the Surface, to its last pixel");
    assert_eq!((at(50, 40), at(79, 59)), (CLEAR, CLEAR), "the right half of the Surface: the viewport is the Surface, not the target");
    for (x, y) in [(19, 30), (30, 19), (30, 60)] {
        assert_eq!(at(x, y), CLEAR, "nothing of the code's drawing at ({x}, {y}), beside the Surface");
    }
    assert_eq!((at(22, 22), at(1, 1)), (WHITE, WHITE), "the page's quads before and after the Surface are drawn, in their own place");
    // The second Surface: device pixels 100..180 by 80..140. Its left
    // half is 100..140, of which the clip keeps 100..120.
    assert_eq!((at(100, 80), at(119, 139)), (GREEN, GREEN), "the part of the Surface inside the clip");
    assert_eq!((at(120, 100), at(139, 100)), (CLEAR, CLEAR), "the clip around the Surface cuts the code's drawing");
}
