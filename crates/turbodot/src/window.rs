//! What a window needs: its options, its GPU device, its keys. `start`
//! runs a `Program` in one.

use std::sync::Arc;

use gogo_ui::Key;
use gogo_wgpu::UiRenderer;
use winit::keyboard::{Key as WinitKey, NamedKey};
use winit::window::Window;

use crate::gpu::Gpu;

/// How many device pixels of a touchpad's scroll count as one notch of a
/// mouse wheel, at one device pixel per epx.
pub(crate) const TOUCHPAD_PIXELS_PER_NOTCH: f32 = 48.0;

/// What `start` needs to know to open a window.
pub struct WindowOptions<'a> {
    pub title: &'a str,
    /// The size of the page inside the window, in epx.
    pub width: f32,
    pub height: f32,
}

/// Which wgpu backends a start asks for; `None` = every backend built in.
///
/// Needed because a start with every backend costs about 400 ms more on a
/// Windows PC than Direct3D 12 alone (measured on FanRust, its
/// docs/DECISIONS.md D126): wgpu then starts Vulkan and OpenGL too, and
/// draws with one.
///
/// Returns the starts to try in order. A backend named by `WGPU_BACKEND`
/// is the only one tried. Windows tries Direct3D 12 alone and then every
/// backend, for a PC whose GPU has no Direct3D 12 driver.
fn backends_to_try(named: Option<wgpu::Backends>, is_windows: bool) -> Vec<Option<wgpu::Backends>> {
    match named {
        Some(backends) => vec![Some(backends)],
        None if is_windows => vec![Some(wgpu::Backends::DX12), None],
        None => vec![None],
    }
}

/// A window and the GPU objects that draw into it.
pub(crate) struct Gfx {
    pub(crate) window: Arc<Window>,
    pub(crate) surface: wgpu::Surface<'static>,
    pub(crate) gpu: Gpu,
    pub(crate) config: wgpu::SurfaceConfiguration,
    pub(crate) renderer: UiRenderer,
    /// Depth buffer of the window's size, for a program with a `Surface
    /// depth=true`; `None` for every other program.
    pub(crate) depth: Option<wgpu::TextureView>,
}

impl Gfx {
    /// Brings up a device that can draw into `window`. `depth_format`:
    /// `Program::depth_format`.
    ///
    /// Returns why the last start that was tried gave no device.
    pub(crate) fn new(window: Arc<Window>, depth_format: Option<wgpu::TextureFormat>) -> Result<Gfx, String> {
        let tries: Vec<Option<wgpu::Backends>> = backends_to_try(wgpu::Backends::from_env(), cfg!(windows));
        let mut why: String = String::new();
        for (i, backends) in tries.iter().enumerate() {
            let is_last: bool = i + 1 == tries.len();
            match Gfx::start(window.clone(), *backends, is_last, depth_format) {
                Ok(gfx) => return Ok(gfx),
                Err(e) => why = e,
            }
        }
        Err(why)
    }

    /// One start on `backends`. `takes_a_software_adapter`: whether an
    /// adapter that draws on the CPU is good enough, which it is only
    /// when no later start can find a real one.
    fn start(window: Arc<Window>, backends: Option<wgpu::Backends>, takes_a_software_adapter: bool, depth_format: Option<wgpu::TextureFormat>) -> Result<Gfx, String> {
        let mut desc = wgpu::InstanceDescriptor::new_without_display_handle();
        if let Some(backends) = backends {
            desc.backends = backends;
        }
        let instance = wgpu::Instance::new(desc);
        let surface: wgpu::Surface<'static> = instance.create_surface(window.clone()).map_err(|e| format!("no surface for the window: {e}"))?;
        let adapter: wgpu::Adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions { compatible_surface: Some(&surface), ..Default::default() })).map_err(|e| format!("no GPU adapter: {e}"))?;
        if !takes_a_software_adapter && adapter.get_info().device_type == wgpu::DeviceType::Cpu {
            return Err("only a software adapter".to_string());
        }
        let (device, queue) = pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor::default())).map_err(|e| format!("no GPU device: {e}"))?;
        let caps = surface.get_capabilities(&adapter);
        if caps.formats.is_empty() {
            return Err("the GPU adapter cannot draw into the window".to_string());
        }
        // A format that stores colors as written: the UI renderer writes
        // sRGB-encoded values, and an `*Srgb` format would encode them a
        // second time (everything too bright).
        let format: wgpu::TextureFormat = caps.formats.iter().copied().find(|f| !f.is_srgb()).unwrap_or(caps.formats[0]);
        let size = window.inner_size();
        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format,
            width: size.width.max(1),
            height: size.height.max(1),
            present_mode: wgpu::PresentMode::Fifo,
            alpha_mode: caps.alpha_modes[0],
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
            color_space: Default::default(),
        };
        surface.configure(&device, &config);
        let renderer: UiRenderer = UiRenderer::new(&device, format, depth_format);
        let depth: Option<wgpu::TextureView> = depth_format.map(|depth_format| crate::gpu::depth_buffer(&device, depth_format, config.width, config.height));
        Ok(Gfx { window, surface, gpu: Gpu { instance, adapter, device, queue }, config, renderer, depth })
    }

    /// Window has `width` x `height` device pixels now: the surface and
    /// the depth buffer are made for that size.
    pub(crate) fn resize(&mut self, width: u32, height: u32) {
        self.config.width = width.max(1);
        self.config.height = height.max(1);
        self.surface.configure(&self.gpu.device, &self.config);
        if let Some(depth_format) = self.renderer.depth_format() {
            self.depth = Some(crate::gpu::depth_buffer(&self.gpu.device, depth_format, self.config.width, self.config.height));
        }
    }
}

/// The gogo key a key of the keyboard means, if gogo has one for it.
pub(crate) fn key_from_winit(key: &WinitKey, is_shift_held: bool) -> Option<Key> {
    match key {
        WinitKey::Named(NamedKey::ArrowUp) => Some(Key::Up),
        WinitKey::Named(NamedKey::ArrowDown) => Some(Key::Down),
        WinitKey::Named(NamedKey::ArrowLeft) => Some(Key::Left),
        WinitKey::Named(NamedKey::ArrowRight) => Some(Key::Right),
        WinitKey::Named(NamedKey::Home) => Some(Key::Home),
        WinitKey::Named(NamedKey::End) => Some(Key::End),
        WinitKey::Named(NamedKey::Enter) => Some(Key::Enter),
        WinitKey::Named(NamedKey::Backspace) => Some(Key::Backspace),
        WinitKey::Named(NamedKey::Delete) => Some(Key::Delete),
        WinitKey::Named(NamedKey::Tab) => Some(if is_shift_held { Key::TabBack } else { Key::Tab }),
        WinitKey::Named(NamedKey::Space) => Some(Key::Char(' ')),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Windows starts on Direct3D 12 alone and keeps every backend for a
    /// second start; a named backend is used as named, with no second
    /// start; elsewhere wgpu picks.
    #[test]
    fn windows_starts_on_dx12_alone_unless_a_backend_is_named() {
        assert_eq!(backends_to_try(None, true), [Some(wgpu::Backends::DX12), None]);
        assert_eq!(backends_to_try(Some(wgpu::Backends::VULKAN), true), [Some(wgpu::Backends::VULKAN)]);
        assert_eq!(backends_to_try(None, false), [None]);
        assert_eq!(backends_to_try(Some(wgpu::Backends::GL), false), [Some(wgpu::Backends::GL)]);
    }

    /// Shift turns Tab into the key that walks focus backward, and a key
    /// gogo has no word for maps to none.
    #[test]
    fn keys_map_to_gogos_words() {
        assert_eq!(key_from_winit(&WinitKey::Named(NamedKey::Tab), false), Some(Key::Tab));
        assert_eq!(key_from_winit(&WinitKey::Named(NamedKey::Tab), true), Some(Key::TabBack));
        assert_eq!(key_from_winit(&WinitKey::Named(NamedKey::ArrowDown), false), Some(Key::Down));
        assert_eq!(key_from_winit(&WinitKey::Named(NamedKey::F5), false), None);
    }
}
