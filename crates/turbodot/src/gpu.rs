//! A GPU device with no window, and a program's pages drawn to a picture.

use std::fs::File;
use std::io::BufWriter;
use std::path::Path;
use std::time::Duration;

use gogo_wgpu::UiRenderer;

use crate::program::Program;

/// 8-bit sRGB pixels: straight alpha, 4 bytes per pixel, RGBA, row by row
/// from the top. The same bytes a decoded PNG holds.
///
/// Needed because a plain `Vec<u8>` does not say whether its colors are
/// multiplied by alpha. The renderer's tile is, a PNG is not, and a mix-up
/// darkens every half-transparent edge with nothing failing.
#[allow(non_camel_case_types, non_snake_case)]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Buf_Straight_Srgb_4xU8_RGBA {
    pub Bytes: Box<[u8]>,
}

/// A page as it was drawn, in device pixels.
///
/// Needed so a test or an AI agent sees what a page looks like without a
/// window on the user's desktop: `at` reads one pixel, `save_png` writes a
/// file to look at.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Picture {
    pub width: u32,
    pub height: u32,
    pub pixels: Buf_Straight_Srgb_4xU8_RGBA,
}

impl Picture {
    /// The pixel at (`x`, `y`) from the top left, as [red, green, blue,
    /// alpha].
    ///
    /// WARNING: panics for a point outside the picture.
    pub fn at(&self, x: u32, y: u32) -> [u8; 4] {
        assert!(x < self.width && y < self.height, "({x}, {y}) is outside a picture of {} x {}", self.width, self.height);
        let i: usize = ((y * self.width + x) * 4) as usize;
        let b: &[u8] = &self.pixels.Bytes;
        [b[i], b[i + 1], b[i + 2], b[i + 3]]
    }

    /// Writes the picture as a PNG file, making the folders above it.
    ///
    /// Returns why the file could not be written.
    pub fn save_png(&self, path: &Path) -> Result<(), String> {
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
        }
        let file: File = File::create(path).map_err(|e| format!("{}: {e}", path.display()))?;
        let mut encoder = png::Encoder::new(BufWriter::new(file), self.width, self.height);
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);
        let mut writer = encoder.write_header().map_err(|e| format!("{}: {e}", path.display()))?;
        writer.write_image_data(&self.pixels.Bytes).map_err(|e| format!("{}: {e}", path.display()))?;
        writer.finish().map_err(|e| format!("{}: {e}", path.display()))
    }
}

/// A program's draw lists, then a host's own pages' above them.
struct ProgramUnderHostPages<'p, A> {
    program: &'p Program<A>,
    host_pages: Option<&'p crate::pages::PageStack>,
}

impl<A> gogo_wgpu::DrawLists for ProgramUnderHostPages<'_, A> {
    fn for_each_list<'a>(&'a self, f: &mut dyn FnMut(&[gogo_ui::Draw], &'a gogo_ui::Page)) {
        gogo_wgpu::DrawLists::for_each_list(self.program, f);
        if let Some(host_pages) = self.host_pages {
            host_pages.for_each_draws(&mut |draws, page, _pose| f(draws, page));
        }
    }
}

/// A GPU device that belongs to no window.
///
/// Needed for work that draws with nobody watching: a test that checks a
/// page's pixels, a command that writes a page to a picture file.
pub struct Gpu {
    pub instance: wgpu::Instance,
    pub adapter: wgpu::Adapter,
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
}

impl Gpu {
    /// Starts a device on any backend built in. The `WGPU_BACKEND` variable
    /// picks one (`vulkan`, `dx12`, `gl`, `metal`), as in wgpu's own tools.
    ///
    /// Returns `None` when this machine has no GPU adapter, as a hosted
    /// test machine may not.
    pub fn new() -> Option<Gpu> {
        let mut desc = wgpu::InstanceDescriptor::new_without_display_handle();
        if let Some(backends) = wgpu::Backends::from_env() {
            desc.backends = backends;
        }
        let instance = wgpu::Instance::new(desc);
        let adapter: wgpu::Adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions::default())).ok()?;
        let (device, queue) = pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor::default())).ok()?;
        Some(Gpu { instance, adapter, device, queue })
    }

    /// Draws the pages of `program` as its last frame laid them out, to
    /// a picture: every page that shows, bottom first, a page on its
    /// way where it is.
    ///
    /// The same renderer and shader a window uses draw it, so the picture
    /// is what a user would see. Where no page paints, the picture is
    /// transparent. Pages' own GPU drawing runs first, as in a window's
    /// frame (`Page::gpu_draw_textures`).
    ///
    /// WARNING: builds a renderer on every call, about 70 ms on Direct3D 12
    /// (it compiles the shader each time). Fine for a test, too slow for a
    /// picture per frame.
    ///
    /// Returns why the GPU did not hand the pixels back, or that
    /// `program` has had no frame yet.
    pub fn draw_program<A: 'static>(&self, program: &mut Program<A>) -> Result<Picture, String> {
        self.draw_program_and(program, None)
    }

    /// `draw_program`, with the pages of `host_pages` drawn above the
    /// program's, each where its own last layout put it.
    ///
    /// Needed by a host that lays out pages of its own beside a
    /// program's (`turbodot::pages`): its picture shows both, as its
    /// window does. A sliding page of `host_pages` is drawn in place.
    ///
    /// Returns what `draw_program` returns.
    pub fn draw_program_with_host_pages<A: 'static>(&self, program: &mut Program<A>, host_pages: &crate::pages::PageStack) -> Result<Picture, String> {
        self.draw_program_and(program, Some(host_pages))
    }

    fn draw_program_and<A: 'static>(&self, program: &mut Program<A>, host_pages: Option<&crate::pages::PageStack>) -> Result<Picture, String> {
        if program.size() == (0.0, 0.0) {
            return Err("the program has had no frame yet: call `Program::frame` first".to_string());
        }
        let epx: f32 = program.epx();
        let size: (u32, u32) = (((program.size().0 * epx).round() as u32).max(1), ((program.size().1 * epx).round() as u32).max(1));
        // A format that stores colors as written: the renderer writes
        // sRGB-encoded values.
        let format: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8Unorm;
        let mut renderer: UiRenderer = UiRenderer::new(&self.device, format);
        // Renderer is new: it has no picture yet (the letters it uploads
        // by itself, `TextEngine::atlas_version`). And a picture of the
        // pages shows the lazy ones too.
        program.forget_uploads();
        program.gpu_draw_as(&mut renderer, self, crate::pictures::LazyPictures::Now, host_pages.map(|pages| pages as &dyn crate::pages::HostPages));
        let lists: ProgramUnderHostPages<'_, A> = ProgramUnderHostPages { program: &*program, host_pages };
        renderer.prepare_pages(&self.device, &self.queue, &lists, epx, size, &mut program.assets.text.borrow_mut(), None);
        let texture: wgpu::Texture = self.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("a program's picture"),
            size: wgpu::Extent3d { width: size.0, height: size.1, depth_or_array_layers: 1 },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });
        let view = texture.create_view(&Default::default());
        let mut encoder = self.device.create_command_encoder(&Default::default());
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("a program's picture"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment { view: &view, depth_slice: None, resolve_target: None, ops: wgpu::Operations { load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT), store: wgpu::StoreOp::Store } })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
            // A `draw=gpu` Surface of a page of `host_pages` is not
            // drawn here: the host makes those calls in its own frame.
            renderer.render_with(&mut pass, &mut |place, pass| {
                program.draw_gpu_surface(place, pass);
            });
        }
        self.queue.submit([encoder.finish()]);
        let mut bytes: Vec<u8> = self.read_back(&texture, size.0, size.1)?;
        straight_from_premul(&mut bytes);
        Ok(Picture { width: size.0, height: size.1, pixels: Buf_Straight_Srgb_4xU8_RGBA { Bytes: bytes.into_boxed_slice() } })
    }

    /// The bytes of a texture of `w` x `h` pixels, row by row, still
    /// multiplied by alpha.
    fn read_back(&self, texture: &wgpu::Texture, w: u32, h: u32) -> Result<Vec<u8>, String> {
        // The GPU copies rows at a fixed byte step, wider than the row.
        let padded: u32 = (w * 4).div_ceil(wgpu::COPY_BYTES_PER_ROW_ALIGNMENT) * wgpu::COPY_BYTES_PER_ROW_ALIGNMENT;
        let buffer = self.device.create_buffer(&wgpu::BufferDescriptor { label: Some("a page's picture"), size: (padded * h) as u64, usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ, mapped_at_creation: false });
        let mut encoder = self.device.create_command_encoder(&Default::default());
        encoder.copy_texture_to_buffer(
            wgpu::TexelCopyTextureInfo { texture, mip_level: 0, origin: wgpu::Origin3d::ZERO, aspect: wgpu::TextureAspect::All },
            wgpu::TexelCopyBufferInfo { buffer: &buffer, layout: wgpu::TexelCopyBufferLayout { offset: 0, bytes_per_row: Some(padded), rows_per_image: Some(h) } },
            wgpu::Extent3d { width: w, height: h, depth_or_array_layers: 1 },
        );
        self.queue.submit([encoder.finish()]);
        let slice = buffer.slice(..);
        slice.map_async(wgpu::MapMode::Read, |_| {});
        self.device.poll(wgpu::PollType::Wait { submission_index: None, timeout: Some(Duration::from_secs(10)) }).map_err(|e| format!("the GPU did not answer within ten seconds: {e}"))?;
        let mut bytes: Vec<u8> = Vec::with_capacity((w * h * 4) as usize);
        {
            let data = slice.get_mapped_range().map_err(|e| format!("the picture's bytes cannot be read: {e}"))?;
            for y in 0..h as usize {
                let start: usize = y * padded as usize;
                bytes.extend_from_slice(&data[start..start + (w * 4) as usize]);
            }
        }
        buffer.unmap();
        Ok(bytes)
    }
}

/// Divides each pixel's color by its alpha, in place.
///
/// Needed because the renderer's tile holds colors multiplied by alpha and
/// a PNG holds them as they are: saved unchanged, every half-transparent
/// edge would come out too dark.
fn straight_from_premul(bytes: &mut [u8]) {
    for pixel in bytes.chunks_exact_mut(4) {
        let alpha: u32 = pixel[3] as u32;
        if alpha == 0 || alpha == 255 {
            continue;
        }
        for channel in &mut pixel[..3] {
            *channel = ((*channel as u32 * 255 + alpha / 2) / alpha).min(255) as u8;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A half-transparent white pixel comes back white, an opaque and a
    /// clear pixel stay as they are.
    #[test]
    fn a_premultiplied_color_is_divided_by_its_alpha() {
        let mut bytes: Vec<u8> = vec![128, 128, 128, 128, 10, 20, 30, 255, 0, 0, 0, 0, 64, 0, 32, 128];
        straight_from_premul(&mut bytes);
        assert_eq!(bytes, [255, 255, 255, 128, 10, 20, 30, 255, 0, 0, 0, 0, 128, 0, 64, 128]);
    }
}
