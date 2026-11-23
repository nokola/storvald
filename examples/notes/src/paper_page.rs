//! Page with a GPU picture of its own: ruled paper under its nodes.
//! Picture: `ui/paper_page.gogo`.

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

#[derive(Default)]
pub struct PaperPage {
    /// Paper texture, made at page's first GPU frame.
    paper: Option<wgpu::Texture>,
}

impl Page<AppState> for PaperPage {
    const PAGE: &'static BuiltPage = &PAGE;
    type Handler = Handler;
    /// Root is see-through, and page's own picture covers the screen.
    const DRAWS_GPU_PICTURE: bool = true;
    /// Touch on the paper, beside the button, goes on to the host: a
    /// painting program would draw there with it.
    const TAKES_EVERY_TOUCH: bool = false;

    fn handle(&mut self, host: &mut Host<AppState>, event: PageEvent<Handler>) {
        match event.handler {
            Handler::GoBack => host.nav.back(),
        }
    }

    /// Each frame before screen is drawn, while page is drawn: makes paper
    /// texture once. A painting would draw its new strokes here.
    fn gpu_draw_textures(&mut self, host: &mut Host<AppState>, gpu: &Gpu) {
        host.app.paper_frames_drawn += 1;
        if self.paper.is_some() {
            return;
        }
        let texels: Vec<u8> = (0..PAPER_SIDE_TEXELS).flat_map(|y| (0..PAPER_SIDE_TEXELS).flat_map(move |_| if y % LINE_EVERY_TEXELS == LINE_EVERY_TEXELS - 1 { LINE } else { PAPER })).collect();
        let size: wgpu::Extent3d = wgpu::Extent3d { width: PAPER_SIDE_TEXELS, height: PAPER_SIDE_TEXELS, depth_or_array_layers: 1 };
        let paper: wgpu::Texture = gpu.device.create_texture(&wgpu::TextureDescriptor { label: Some("paper"), size, mip_level_count: 1, sample_count: 1, dimension: wgpu::TextureDimension::D2, format: wgpu::TextureFormat::Rgba8Unorm, usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST, view_formats: &[] });
        gpu.queue.write_texture(wgpu::TexelCopyTextureInfo { texture: &paper, mip_level: 0, origin: wgpu::Origin3d::ZERO, aspect: wgpu::TextureAspect::All }, &texels, wgpu::TexelCopyBufferLayout { offset: 0, bytes_per_row: Some(PAPER_SIDE_TEXELS * 4), rows_per_image: Some(PAPER_SIDE_TEXELS) }, size);
        self.paper = Some(paper);
    }

    /// Paper goes under page's nodes, over the whole page, in the pass
    /// that draws every page.
    fn gpu_draw_screen(&self, host: &Host<AppState>, layers: &mut Layers) {
        if let Some(paper) = &self.paper {
            layers.picture(Tex_Image_Straight_Srgb_4xU8_RGBA::new(paper), host.ui.rect(NAMES.paper_page));
        }
    }
}
