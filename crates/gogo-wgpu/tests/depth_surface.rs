//! `depth=true` on a `Surface draw=gpu`, in the renderer: the pass has a
//! depth buffer, the program's own draw calls test and write it, and
//! gogo's drawing leaves it alone.
//!
//! Every test here needs a GPU adapter and is ignored unless asked for.

use gogo_text::TextEngine;
use gogo_ui::layout::Rect;
use gogo_ui::{Color_Straight_Srgb_4xU8_RGBA, Draw, NodeId, Page, compile, has_errors};
use gogo_wgpu::{SurfacePlace, UiRenderer};

const FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8Unorm;
const DEPTH_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Depth32Float;
/// The target, in device pixels.
const TARGET: (u32, u32) = (200, 160);
/// Bytes from one row of a read to the next: the GPU copies rows at a
/// fixed byte step, wider than the row.
const ROW_BYTES: u32 = (TARGET.0 * 4).div_ceil(wgpu::COPY_BYTES_PER_ROW_ALIGNMENT) * wgpu::COPY_BYTES_PER_ROW_ALIGNMENT;
/// Device pixels per epx in these tests.
const EPX: f32 = 2.0;

const RED: [u8; 4] = [255, 0, 0, 255];
const GREEN: [u8; 4] = [0, 255, 0, 255];
const BLUE: [u8; 4] = [0, 0, 255, 255];
const WHITE: [u8; 4] = [255, 255, 255, 255];
const CLEAR: [u8; 4] = [0, 0, 0, 0];

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

/// A page with three `Surface draw=gpu`, and their nodes in file order.
fn page_with_gpu_surfaces() -> (Page, [NodeId; 3]) {
    let compiled = compile("View v { Layers l width=fill, height=fill {\n Surface first draw=gpu, depth=true, width=10, height=10\n Surface second draw=gpu, depth=true, width=10, height=10\n Surface third draw=gpu, width=10, height=10\n} }", &|_| None);
    assert!(!has_errors(&compiled.diags), "{:#?}", compiled.diags);
    let node = |name: &str| compiled.page.find(name).expect("a Surface of the page");
    let nodes: [NodeId; 3] = [node("first"), node("second"), node("third")];
    (compiled.page, nodes)
}

fn quad(rect: Rect) -> Draw {
    Draw::Quad { rect, color: Color_Straight_Srgb_4xU8_RGBA::opaque(0xFFFFFF), corner: 0.0, border: 0.0, border_color: Color_Straight_Srgb_4xU8_RGBA::TRANSPARENT }
}

/// A pipeline of the program's own: one triangle over all of clip space,
/// at depth `z`, in `color`, kept where it is nearer than what the depth
/// buffer holds, and written there.
fn fill_at_depth(device: &wgpu::Device, z: f32, color: [u8; 4]) -> wgpu::RenderPipeline {
    let [r, g, b, a] = color.map(|byte| byte as f32 / 255.0);
    let shader: String = format!(
        "
@vertex
fn vs_main(@builtin(vertex_index) index: u32) -> @builtin(position) vec4<f32> {{
    let corners = array<vec2<f32>, 3>(vec2<f32>(-1.0, -1.0), vec2<f32>(3.0, -1.0), vec2<f32>(-1.0, 3.0));
    return vec4<f32>(corners[index], {z:?}, 1.0);
}}

@fragment
fn fs_main() -> @location(0) vec4<f32> {{
    return vec4<f32>({r:?}, {g:?}, {b:?}, {a:?});
}}
"
    );
    let module = device.create_shader_module(wgpu::ShaderModuleDescriptor { label: Some("fill"), source: wgpu::ShaderSource::Wgsl(shader.into()) });
    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("fill"),
        layout: None,
        vertex: wgpu::VertexState { module: &module, entry_point: Some("vs_main"), compilation_options: Default::default(), buffers: &[] },
        primitive: wgpu::PrimitiveState::default(),
        depth_stencil: Some(wgpu::DepthStencilState { format: DEPTH_FORMAT, depth_write_enabled: Some(true), depth_compare: Some(wgpu::CompareFunction::Less), stencil: wgpu::StencilState::default(), bias: wgpu::DepthBiasState::default() }),
        multisample: wgpu::MultisampleState::default(),
        fragment: Some(wgpu::FragmentState { module: &module, entry_point: Some("fs_main"), compilation_options: Default::default(), targets: &[Some(wgpu::ColorTargetState { format: FORMAT, blend: None, write_mask: wgpu::ColorWrites::ALL })] }),
        multiview_mask: None,
        cache: None,
    })
}

/// What one frame gave: its pixels, the places `render_with` handed out
/// and the draw calls the renderer counted.
struct Drawn {
    pixels: Vec<u8>,
    places: Vec<SurfacePlace>,
    draw_calls: u32,
}

impl Drawn {
    /// The pixel at (`x`, `y`) of the target, from its top left.
    fn at(&self, x: u32, y: u32) -> [u8; 4] {
        let i: usize = (y * ROW_BYTES + x * 4) as usize;
        [self.pixels[i], self.pixels[i + 1], self.pixels[i + 2], self.pixels[i + 3]]
    }
}

/// Draws `draws` into a target of `TARGET` in a pass with a depth buffer
/// cleared to 1.0, as a host with a `depth=true` Surface does.
/// `draw_gpu` makes the program's draw calls of each Surface.
fn draw_with_depth(device: &wgpu::Device, queue: &wgpu::Queue, draws: &[Draw], page: &Page, draw_gpu: &mut dyn FnMut(&SurfacePlace, &mut wgpu::RenderPass<'_>)) -> Drawn {
    let mut ui: UiRenderer = UiRenderer::new(device, FORMAT, Some(DEPTH_FORMAT));
    let mut text: TextEngine = TextEngine::new(&[]);
    ui.prepare_pages(device, queue, &[(draws, page)], EPX, TARGET, &mut text, None);
    let size = wgpu::Extent3d { width: TARGET.0, height: TARGET.1, depth_or_array_layers: 1 };
    let texture = device.create_texture(&wgpu::TextureDescriptor { label: Some("target"), size, mip_level_count: 1, sample_count: 1, dimension: wgpu::TextureDimension::D2, format: FORMAT, usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC, view_formats: &[] });
    let view = texture.create_view(&Default::default());
    let depth = device.create_texture(&wgpu::TextureDescriptor { label: Some("depth"), size, mip_level_count: 1, sample_count: 1, dimension: wgpu::TextureDimension::D2, format: DEPTH_FORMAT, usage: wgpu::TextureUsages::RENDER_ATTACHMENT, view_formats: &[] });
    let depth_view = depth.create_view(&Default::default());
    let mut encoder = device.create_command_encoder(&Default::default());
    let mut places: Vec<SurfacePlace> = Vec::new();
    {
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("target"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment { view: &view, depth_slice: None, resolve_target: None, ops: wgpu::Operations { load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT), store: wgpu::StoreOp::Store } })],
            depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment { view: &depth_view, depth_ops: Some(wgpu::Operations { load: wgpu::LoadOp::Clear(1.0), store: wgpu::StoreOp::Discard }), stencil_ops: None }),
            timestamp_writes: None,
            occlusion_query_set: None,
            multiview_mask: None,
        });
        ui.render_with(&mut pass, &mut |place, pass| {
            places.push(*place);
            draw_gpu(place, pass);
        });
    }
    let row_bytes: u32 = ROW_BYTES;
    let buffer = device.create_buffer(&wgpu::BufferDescriptor { label: Some("readback"), size: (row_bytes * TARGET.1) as u64, usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ, mapped_at_creation: false });
    encoder.copy_texture_to_buffer(
        wgpu::TexelCopyTextureInfo { texture: &texture, mip_level: 0, origin: wgpu::Origin3d::ZERO, aspect: wgpu::TextureAspect::All },
        wgpu::TexelCopyBufferInfo { buffer: &buffer, layout: wgpu::TexelCopyBufferLayout { offset: 0, bytes_per_row: Some(row_bytes), rows_per_image: Some(TARGET.1) } },
        size,
    );
    queue.submit([encoder.finish()]);
    let slice = buffer.slice(..);
    slice.map_async(wgpu::MapMode::Read, |_| {});
    device.poll(wgpu::PollType::Wait { submission_index: None, timeout: Some(std::time::Duration::from_secs(10)) }).expect("the GPU answers within ten seconds");
    let pixels: Vec<u8> = slice.get_mapped_range().expect("the read is mapped").to_vec();
    Drawn { pixels, places, draw_calls: ui.draw_calls }
}

/// Inside a `depth=true` Surface the program's draw calls are tested
/// against the depth buffer: what is nearer shows, whatever the order of
/// the calls. gogo's own drawing takes no part in it: a quad under the
/// Surface writes no depth, and a quad after the Surface is drawn over
/// what the code drew, however near that is.
#[test]
#[ignore = "needs a GPU adapter: run with --include-ignored on a machine that has one"]
fn code_tests_depth_in_its_surface_and_gogo_draws_in_paint_order() {
    let (page, [first, _, _]) = page_with_gpu_surfaces();
    let draws: Vec<Draw> = vec![
        // In epx; a device pixel is half of one. Under the Surface's left part.
        quad(Rect::new(10.0, 10.0, 20.0, 20.0)),
        Draw::GpuSurface { rect: Rect::new(10.0, 10.0, 60.0, 40.0), surface: first, alpha: 1.0, has_depth: true },
        // Over the Surface's right part.
        quad(Rect::new(50.0, 10.0, 20.0, 20.0)),
    ];
    let (device, queue) = device();
    // The nearest depth there is, drawn first; then something farther.
    let (near, far) = (fill_at_depth(&device, 0.0, GREEN), fill_at_depth(&device, 0.5, RED));
    let drawn: Drawn = draw_with_depth(&device, &queue, &draws, &page, &mut |_place, pass| {
        pass.set_pipeline(&near);
        pass.draw(0..3, 0..1);
        pass.set_pipeline(&far);
        pass.draw(0..3, 0..1);
    });

    assert_eq!(drawn.places.len(), 1);
    assert_eq!((drawn.places[0].depth_format, drawn.places[0].has_depth), (Some(DEPTH_FORMAT), true));
    // The Surface: device pixels 20..140 by 20..100.
    assert_eq!(drawn.at(80, 80), GREEN, "the nearer drawing shows, though the farther one was drawn after it");
    assert_eq!(drawn.at(40, 40), GREEN, "the page's quad under the Surface wrote no depth: the code draws over it");
    assert_eq!(drawn.at(120, 40), WHITE, "the page's quad after the Surface is drawn over it, though the code drew at the nearest depth");
    assert_eq!(drawn.at(10, 10), CLEAR);
    // One quad batch, the Surface, one quad batch: the first code of a
    // frame finds the depth the pass cleared, and no clear is drawn.
    assert_eq!(drawn.draw_calls, 3);
}

/// Two `depth=true` Surfaces of one frame do not see each other's depth:
/// before the second one's draw calls, the depth in the part of it that
/// shows is 1.0 again, and nowhere else. A `draw=gpu` Surface without
/// `depth=true` gets no such clear.
#[test]
#[ignore = "needs a GPU adapter: run with --include-ignored on a machine that has one"]
fn a_later_depth_surface_finds_its_own_part_of_the_depth_cleared() {
    let (page, [first, second, third]) = page_with_gpu_surfaces();
    let draws: Vec<Draw> = vec![
        // Device pixels 20..140 by 20..100.
        Draw::GpuSurface { rect: Rect::new(10.0, 10.0, 60.0, 40.0), surface: first, alpha: 1.0, has_depth: true },
        // Device pixels 100..140 by 20..100, of which a clip keeps 100..120.
        Draw::ClipPush(Rect::new(50.0, 10.0, 10.0, 40.0)),
        Draw::GpuSurface { rect: Rect::new(50.0, 10.0, 20.0, 40.0), surface: second, alpha: 1.0, has_depth: true },
        Draw::ClipPop,
        // Over the first one again, with no `depth=true`.
        Draw::GpuSurface { rect: Rect::new(10.0, 10.0, 60.0, 40.0), surface: third, alpha: 1.0, has_depth: false },
    ];
    let (device, queue) = device();
    let (near, far, middle) = (fill_at_depth(&device, 0.1, GREEN), fill_at_depth(&device, 0.9, BLUE), fill_at_depth(&device, 0.5, RED));
    let drawn: Drawn = draw_with_depth(&device, &queue, &draws, &page, &mut |place, pass| {
        // The third Surface's code tests depth too: with a depth buffer
        // in the pass, its pipeline names the depth format anyway.
        pass.set_pipeline(if place.surface == first { &near } else if place.surface == second { &far } else { &middle });
        pass.draw(0..3, 0..1);
    });

    assert_eq!(drawn.places.iter().map(|place| (place.surface, place.has_depth, place.depth_format)).collect::<Vec<_>>(), [(first, true, Some(DEPTH_FORMAT)), (second, true, Some(DEPTH_FORMAT)), (third, false, Some(DEPTH_FORMAT))]);
    // The part of the second Surface that shows: its far drawing landed,
    // so the first one's near depth was gone there. The third one's
    // drawing, between the two, then landed over it.
    assert_eq!(drawn.at(110, 60), RED, "the second Surface found its depth cleared: its far drawing was kept, and the middle one drawn over it");
    // The part of the second Surface the clip cut: nothing cleared there.
    assert_eq!(drawn.at(130, 60), GREEN, "the clear stays inside the part of the Surface that shows");
    // The first Surface alone: the third one's drawing is farther than
    // the first one's, and the third one got no clear.
    assert_eq!(drawn.at(50, 60), GREEN, "a Surface without `depth=true` gets no clear: it sees the depth that is there");
    // Three Surfaces and one clear, before the second.
    assert_eq!(drawn.draw_calls, 4);
}

/// The far drawing of a second Surface alone, to see it land: without
/// the clear it would lose against the first one's near depth.
#[test]
#[ignore = "needs a GPU adapter: run with --include-ignored on a machine that has one"]
fn a_second_depth_surface_does_not_see_the_first_one_s_depth() {
    let (page, [first, second, _]) = page_with_gpu_surfaces();
    let draws: Vec<Draw> = vec![
        Draw::GpuSurface { rect: Rect::new(10.0, 10.0, 60.0, 40.0), surface: first, alpha: 1.0, has_depth: true },
        Draw::GpuSurface { rect: Rect::new(50.0, 10.0, 20.0, 40.0), surface: second, alpha: 1.0, has_depth: true },
    ];
    let (device, queue) = device();
    let (near, far) = (fill_at_depth(&device, 0.1, GREEN), fill_at_depth(&device, 0.9, BLUE));
    let drawn: Drawn = draw_with_depth(&device, &queue, &draws, &page, &mut |place, pass| {
        pass.set_pipeline(if place.surface == first { &near } else { &far });
        pass.draw(0..3, 0..1);
    });
    assert_eq!(drawn.at(120, 60), BLUE, "the second Surface's far drawing shows over the first one's near drawing");
    assert_eq!(drawn.at(60, 60), GREEN);
}

/// The clear before a later `depth=true` Surface touches depth alone: a
/// Surface whose code draws nothing leaves what was drawn under it as it
/// was.
#[test]
#[ignore = "needs a GPU adapter: run with --include-ignored on a machine that has one"]
fn the_clear_before_a_later_depth_surface_writes_no_color() {
    let (page, [first, second, _]) = page_with_gpu_surfaces();
    let draws: Vec<Draw> = vec![
        Draw::GpuSurface { rect: Rect::new(10.0, 10.0, 60.0, 40.0), surface: first, alpha: 1.0, has_depth: true },
        Draw::GpuSurface { rect: Rect::new(50.0, 10.0, 20.0, 40.0), surface: second, alpha: 1.0, has_depth: true },
    ];
    let (device, queue) = device();
    let near = fill_at_depth(&device, 0.1, GREEN);
    let drawn: Drawn = draw_with_depth(&device, &queue, &draws, &page, &mut |place, pass| {
        if place.surface == first {
            pass.set_pipeline(&near);
            pass.draw(0..3, 0..1);
        }
    });
    assert_eq!(drawn.at(120, 60), GREEN, "the first Surface's drawing is still there under the second, which drew nothing");
    assert_eq!(drawn.draw_calls, 3, "two Surfaces and the clear before the second");
}

/// A renderer made for a pass with no depth buffer tells the code so: the
/// place names no depth format, and the code's pipelines are made without
/// one.
#[test]
#[ignore = "needs a GPU adapter: run with --include-ignored on a machine that has one"]
fn a_renderer_with_no_depth_format_hands_out_places_with_none() {
    let (page, [_, _, third]) = page_with_gpu_surfaces();
    let draws: Vec<Draw> = vec![Draw::GpuSurface { rect: Rect::new(10.0, 10.0, 60.0, 40.0), surface: third, alpha: 1.0, has_depth: false }];
    let (device, queue) = device();
    let ui: UiRenderer = UiRenderer::new(&device, FORMAT, None);
    assert_eq!(ui.depth_format(), None);
    assert_eq!(UiRenderer::new(&device, FORMAT, Some(DEPTH_FORMAT)).depth_format(), Some(DEPTH_FORMAT));
    assert_eq!(places_without_depth(&device, &queue, &draws, &page), [(None, false)]);
}

/// A Surface that says `depth=true`, drawn by a renderer made for a pass
/// with no depth buffer, stops a debug build with the Surface's name and
/// what to hand `UiRenderer::new`: its draw calls would test a depth
/// buffer that is not there.
#[test]
#[ignore = "needs a GPU adapter: run with --include-ignored on a machine that has one"]
#[cfg_attr(debug_assertions, should_panic(expected = "Surface `first` says `depth=true`, and this renderer was made for a pass with no depth buffer"))]
fn a_depth_surface_in_a_pass_with_no_depth_buffer_stops_a_debug_build() {
    let (page, [first, _, _]) = page_with_gpu_surfaces();
    let draws: Vec<Draw> = vec![Draw::GpuSurface { rect: Rect::new(10.0, 10.0, 60.0, 40.0), surface: first, alpha: 1.0, has_depth: true }];
    let (device, queue) = device();
    // A release build goes on and tells the code that it has no depth.
    assert_eq!(places_without_depth(&device, &queue, &draws, &page), [(None, false)]);
}

/// (`depth_format`, `has_depth`) of each place a renderer made with no
/// depth format hands out for `draws`, in a pass with no depth buffer.
fn places_without_depth(device: &wgpu::Device, queue: &wgpu::Queue, draws: &[Draw], page: &Page) -> Vec<(Option<wgpu::TextureFormat>, bool)> {
    let mut ui: UiRenderer = UiRenderer::new(device, FORMAT, None);
    let mut text: TextEngine = TextEngine::new(&[]);
    ui.prepare_pages(device, queue, &[(draws, page)], EPX, TARGET, &mut text, None);
    let texture = device.create_texture(&wgpu::TextureDescriptor { label: Some("target"), size: wgpu::Extent3d { width: TARGET.0, height: TARGET.1, depth_or_array_layers: 1 }, mip_level_count: 1, sample_count: 1, dimension: wgpu::TextureDimension::D2, format: FORMAT, usage: wgpu::TextureUsages::RENDER_ATTACHMENT, view_formats: &[] });
    let view = texture.create_view(&Default::default());
    let mut encoder = device.create_command_encoder(&Default::default());
    let mut places: Vec<(Option<wgpu::TextureFormat>, bool)> = Vec::new();
    {
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("target"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment { view: &view, depth_slice: None, resolve_target: None, ops: wgpu::Operations { load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT), store: wgpu::StoreOp::Store } })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
            multiview_mask: None,
        });
        ui.render_with(&mut pass, &mut |place, _pass| places.push((place.depth_format, place.has_depth)));
    }
    queue.submit([encoder.finish()]);
    places
}
