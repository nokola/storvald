//! Page with three parts its own code draws, under its nodes: ruled paper
//! (a texture), its margin line (draw commands) and a stamp (GPU draw
//! calls of the page's own).
//! Picture: `ui/paper_page.gogo`.

use turbodot::gogo::{Color_Straight_Srgb_4xU8_RGBA, Draw, Rect};
use turbodot::prelude::*;
use turbodot::{Tex_Image_Straight_Srgb_4xU8_RGBA, wgpu};

use crate::AppState;
use crate::ui::paper_page::{Handler, NAMES, PAGE};

/// Side of paper texture, in texels. Stretched over the window.
pub const PAPER_SIDE_TEXELS: u32 = 64;
/// One ruled line every this many texel rows, on the last row of each.
pub const LINE_EVERY_TEXELS: u32 = 16;
/// Paper's color: sRGB bytes, RGBA.
pub const PAPER: [u8; 4] = [0xF4, 0xF0, 0xE0, 0xFF];
/// Ruled line's color: sRGB bytes, RGBA.
pub const LINE: [u8; 4] = [0x9C, 0xB4, 0xD8, 0xFF];
/// Margin line's color: sRGB bytes, RGBA.
pub const MARGIN: [u8; 4] = [0xD0, 0x40, 0x40, 0xFF];
/// Margin line's distance from the page's right edge, in epx.
pub const MARGIN_FROM_RIGHT_EPX: f32 = 40.0;
/// Margin line's width, in epx.
pub const MARGIN_WIDTH_EPX: f32 = 2.0;
/// Stamp's color: sRGB bytes, RGBA.
pub const STAMP: [u8; 4] = [0x20, 0x80, 0x40, 0xFF];

/// Stamp's shader: one triangle over all of clip space, in `STAMP`.
/// The viewport is on the stamp's Surface, so it fills that and no more.
fn stamp_shader() -> String {
    let [red, green, blue, alpha] = STAMP.map(|byte| byte as f32 / 255.0);
    format!(
        "
@vertex
fn vs_main(@builtin(vertex_index) index: u32) -> @builtin(position) vec4<f32> {{
    let corners = array<vec2<f32>, 3>(vec2<f32>(-1.0, -1.0), vec2<f32>(3.0, -1.0), vec2<f32>(-1.0, 3.0));
    return vec4<f32>(corners[index], 0.0, 1.0);
}}

@fragment
fn fs_main() -> @location(0) vec4<f32> {{
    return vec4<f32>({red:?}, {green:?}, {blue:?}, {alpha:?});
}}
"
    )
}

#[derive(Default)]
pub struct PaperPage {
    /// Paper texture, made at page's first GPU frame.
    paper: Option<wgpu::Texture>,
    /// Stamp's pipeline, with the format of the pass it was made for.
    stamp: Option<(wgpu::TextureFormat, wgpu::RenderPipeline)>,
}

impl Page<AppState> for PaperPage {
    const PAGE: &'static BuiltPage = &PAGE;
    type Handler = Handler;
    /// Touch on the paper, beside the button, goes on to the host: a
    /// painting program would draw there with it.
    const TAKES_EVERY_TOUCH: bool = false;

    fn handle(&mut self, host: &mut Host<AppState>, event: PageEvent<Handler>) {
        match event.handler {
            Handler::GoBack => host.nav.back(),
        }
    }

    /// Each frame before screen is drawn, while page is drawn: makes paper
    /// texture once and hands it to the `paper` Surface. A painting would
    /// draw its new strokes into its texture here.
    fn gpu_prepare(&mut self, host: &mut Host<AppState>, gpu: &Gpu) {
        host.app.paper_frames_drawn += 1;
        // Stamp's pipeline, for the format of the pass page is drawn in:
        // a window and a picture of the program may have two.
        if let Some(format) = host.ui.target_format() {
            if self.stamp.as_ref().is_none_or(|(made_for, _)| *made_for != format) {
                self.stamp = Some((format, stamp_pipeline(&gpu.device, format)));
            }
        }
        if self.paper.is_some() {
            return;
        }
        let texels: Vec<u8> = (0..PAPER_SIDE_TEXELS).flat_map(|y| (0..PAPER_SIDE_TEXELS).flat_map(move |_| if y % LINE_EVERY_TEXELS == LINE_EVERY_TEXELS - 1 { LINE } else { PAPER })).collect();
        let size: wgpu::Extent3d = wgpu::Extent3d { width: PAPER_SIDE_TEXELS, height: PAPER_SIDE_TEXELS, depth_or_array_layers: 1 };
        let paper: wgpu::Texture = gpu.device.create_texture(&wgpu::TextureDescriptor { label: Some("paper"), size, mip_level_count: 1, sample_count: 1, dimension: wgpu::TextureDimension::D2, format: wgpu::TextureFormat::Rgba8Unorm, usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST, view_formats: &[] });
        gpu.queue.write_texture(wgpu::TexelCopyTextureInfo { texture: &paper, mip_level: 0, origin: wgpu::Origin3d::ZERO, aspect: wgpu::TextureAspect::All }, &texels, wgpu::TexelCopyBufferLayout { offset: 0, bytes_per_row: Some(PAPER_SIDE_TEXELS * 4), rows_per_image: Some(PAPER_SIDE_TEXELS) }, size);
        host.ui.set_picture_texture(NAMES.paper, Tex_Image_Straight_Srgb_4xU8_RGBA::new(&paper));
        self.paper = Some(paper);
    }

    /// Margin line, from the Surface's top to its bottom. Asked for when
    /// the Surface first shows, when the window's size changes, and when
    /// who moved the line said so (`PageUi::surface_changed`).
    fn draw_surface(&self, app: &AppState, surface: CommandSurface, out: &mut SurfaceCommands) {
        let _ = surface;
        let line: Rect = Rect::new(out.width() - MARGIN_FROM_RIGHT_EPX - app.paper_margin_shift_epx, 0.0, MARGIN_WIDTH_EPX, out.height());
        out.draw(Draw::Quad { rect: line, color: Color_Straight_Srgb_4xU8_RGBA(MARGIN), corner: 0.0, border: 0.0, border_color: Color_Straight_Srgb_4xU8_RGBA::TRANSPARENT });
    }

    /// Stamp: one triangle over all of clip space. The engine put the
    /// viewport on the stamp's Surface, so the stamp moves with page and
    /// is cut where the Surface is cut, with nothing done here for it.
    fn draw_surface_gpu(&self, app: &AppState, surface: GpuSurface, pass: &mut wgpu::RenderPass<'_>, place: &turbodot::SurfacePlace) {
        let _ = (app, surface);
        let Some((made_for, pipeline)) = &self.stamp else { return };
        if *made_for != place.target_format {
            return;
        }
        pass.set_pipeline(pipeline);
        pass.draw(0..3, 0..1);
    }
}

/// Pipeline that draws the stamp into a pass whose color target has
/// `format`.
fn stamp_pipeline(device: &wgpu::Device, format: wgpu::TextureFormat) -> wgpu::RenderPipeline {
    let module: wgpu::ShaderModule = device.create_shader_module(wgpu::ShaderModuleDescriptor { label: Some("stamp"), source: wgpu::ShaderSource::Wgsl(stamp_shader().into()) });
    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("stamp"),
        layout: None,
        vertex: wgpu::VertexState { module: &module, entry_point: Some("vs_main"), compilation_options: Default::default(), buffers: &[] },
        primitive: wgpu::PrimitiveState::default(),
        depth_stencil: None,
        multisample: wgpu::MultisampleState::default(),
        fragment: Some(wgpu::FragmentState { module: &module, entry_point: Some("fs_main"), compilation_options: Default::default(), targets: &[Some(wgpu::ColorTargetState { format, blend: None, write_mask: wgpu::ColorWrites::ALL })] }),
        multiview_mask: None,
        cache: None,
    })
}
