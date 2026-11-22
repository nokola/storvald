//! A `Ui` in a window: the window, the GPU device and the frame loop.

use std::sync::Arc;
use std::time::Instant;

use gogo_ui::{Event, Key, Mods, Phase};
use gogo_wgpu::UiRenderer;
use winit::application::ApplicationHandler;
use winit::dpi::LogicalSize;
use winit::event::{ElementState, MouseButton, MouseScrollDelta, WindowEvent};
use winit::event_loop::{ActiveEventLoop, EventLoop};
use winit::keyboard::{Key as WinitKey, NamedKey};
use winit::window::{Window, WindowId};

use crate::app::App;
use crate::ui::Ui;

/// How many device pixels of a touchpad's scroll count as one notch of a
/// mouse wheel, at one device pixel per epx.
pub(crate) const TOUCHPAD_PIXELS_PER_NOTCH: f32 = 48.0;

/// What `run` needs to know to open a window.
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
    pub(crate) device: wgpu::Device,
    pub(crate) queue: wgpu::Queue,
    pub(crate) config: wgpu::SurfaceConfiguration,
    pub(crate) renderer: UiRenderer,
}

impl Gfx {
    /// Brings up a device that can draw into `window`.
    ///
    /// Returns why the last start that was tried gave no device.
    pub(crate) fn new(window: Arc<Window>) -> Result<Gfx, String> {
        let tries: Vec<Option<wgpu::Backends>> = backends_to_try(wgpu::Backends::from_env(), cfg!(windows));
        let mut why: String = String::new();
        for (i, backends) in tries.iter().enumerate() {
            let is_last: bool = i + 1 == tries.len();
            match Gfx::start(window.clone(), *backends, is_last) {
                Ok(gfx) => return Ok(gfx),
                Err(e) => why = e,
            }
        }
        Err(why)
    }

    /// One start on `backends`. `takes_a_software_adapter`: whether an
    /// adapter that draws on the CPU is good enough, which it is only
    /// when no later start can find a real one.
    fn start(window: Arc<Window>, backends: Option<wgpu::Backends>, takes_a_software_adapter: bool) -> Result<Gfx, String> {
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
        let renderer: UiRenderer = UiRenderer::new(&device, format);
        Ok(Gfx { window, surface, device, queue, config, renderer })
    }
}

/// The window's side of a running app.
struct Host<'a, A: App> {
    options: WindowOptions<'a>,
    ui: Ui,
    app: A,
    gfx: Option<Gfx>,
    /// The mouse, in device pixels.
    cursor: (f32, f32),
    is_mouse_down: bool,
    mods: Mods,
    started: Instant,
    last_frame: Instant,
    /// Why the window could not start, for `run` to return.
    failed: Option<String>,
}

impl<A: App> Host<'_, A> {
    fn redraw(&self) {
        if let Some(gfx) = &self.gfx {
            gfx.window.request_redraw();
        }
    }

    /// Device pixels per epx: the screen's own scale, so a size written
    /// in a `.gogo` file is the same finger-sized thing on every screen.
    fn epx(&self) -> f32 {
        self.gfx.as_ref().map_or(1.0, |g| g.window.scale_factor() as f32)
    }

    /// The mouse in epx.
    fn cursor_epx(&self) -> (f32, f32) {
        let epx: f32 = self.epx();
        (self.cursor.0 / epx, self.cursor.1 / epx)
    }

    /// Hands the page's events to the app and asks for a frame.
    fn handle(&mut self, events: &[Event]) {
        if !events.is_empty() {
            self.app.handle(&mut self.ui, events);
        }
        self.redraw();
    }

    /// One mouse event into the page.
    fn pointer(&mut self, phase: Phase) {
        let (x, y) = self.cursor_epx();
        if phase == Phase::Move && !self.is_mouse_down {
            self.ui.hover(x, y);
            self.redraw();
            return;
        }
        let events: Vec<Event> = self.ui.touch(phase, x, y, self.started.elapsed().as_secs_f64());
        self.handle(&events);
    }

    /// One frame: advance, lay out, draw.
    fn frame(&mut self) {
        let Some(gfx) = self.gfx.as_mut() else { return };
        let now: Instant = Instant::now();
        // A window that was hidden for a minute does not play a minute
        // of animation in one step.
        let dt: f32 = now.duration_since(self.last_frame).as_secs_f32().min(0.1);
        self.last_frame = now;
        let epx: f32 = gfx.window.scale_factor() as f32;
        let size: (u32, u32) = (gfx.config.width, gfx.config.height);
        self.ui.set_epx(epx);
        let is_animating: bool = self.ui.frame(size.0 as f32 / epx, size.1 as f32 / epx, dt);

        gfx.renderer.prepare_pages(&gfx.device, &gfx.queue, &[(self.ui.draws.as_slice(), &self.ui.page)], epx, size, &mut self.ui.text.borrow_mut(), None);
        let frame = match gfx.surface.get_current_texture() {
            wgpu::CurrentSurfaceTexture::Success(frame) | wgpu::CurrentSurfaceTexture::Suboptimal(frame) => frame,
            // A window that is hidden, being resized or lost its surface:
            // configure again and try on the next frame.
            _ => {
                gfx.surface.configure(&gfx.device, &gfx.config);
                gfx.window.request_redraw();
                return;
            }
        };
        let view = frame.texture.create_view(&Default::default());
        let mut encoder = gfx.device.create_command_encoder(&Default::default());
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("the window"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment { view: &view, depth_slice: None, resolve_target: None, ops: wgpu::Operations { load: wgpu::LoadOp::Clear(wgpu::Color::BLACK), store: wgpu::StoreOp::Store } })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
            gfx.renderer.render(&mut pass);
        }
        gfx.queue.submit([encoder.finish()]);
        gfx.queue.present(frame);
        if is_animating {
            gfx.window.request_redraw();
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

impl<A: App> ApplicationHandler for Host<'_, A> {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.gfx.is_some() {
            return;
        }
        let attributes = Window::default_attributes().with_title(self.options.title).with_inner_size(LogicalSize::new(self.options.width as f64, self.options.height as f64));
        let started: Result<Gfx, String> = event_loop.create_window(attributes).map_err(|e| format!("no window: {e}")).and_then(|window| Gfx::new(Arc::new(window)));
        match started {
            Ok(gfx) => {
                gfx.window.request_redraw();
                self.gfx = Some(gfx);
            }
            Err(why) => {
                self.failed = Some(why);
                event_loop.exit();
            }
        }
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::Resized(size) => {
                if let Some(gfx) = self.gfx.as_mut() {
                    gfx.config.width = size.width.max(1);
                    gfx.config.height = size.height.max(1);
                    gfx.surface.configure(&gfx.device, &gfx.config);
                    gfx.window.request_redraw();
                }
            }
            WindowEvent::ScaleFactorChanged { .. } => self.redraw(),
            WindowEvent::CursorMoved { position, .. } => {
                self.cursor = (position.x as f32, position.y as f32);
                self.pointer(Phase::Move);
            }
            WindowEvent::CursorLeft { .. } => {
                self.ui.unhover();
                self.redraw();
            }
            WindowEvent::MouseInput { state, button: MouseButton::Left, .. } => {
                self.is_mouse_down = state == ElementState::Pressed;
                self.pointer(if self.is_mouse_down { Phase::Down } else { Phase::Up });
            }
            WindowEvent::MouseWheel { delta, .. } => {
                let notches: f32 = match delta {
                    MouseScrollDelta::LineDelta(_, lines) => lines,
                    MouseScrollDelta::PixelDelta(by) => by.y as f32 / (TOUCHPAD_PIXELS_PER_NOTCH * self.epx()),
                };
                let (x, y) = self.cursor_epx();
                let events: Vec<Event> = self.ui.wheel(x, y, notches);
                self.handle(&events);
            }
            WindowEvent::ModifiersChanged(mods) => {
                self.mods = Mods { shift: mods.state().shift_key(), ctrl: mods.state().control_key() };
            }
            WindowEvent::KeyboardInput { event, .. } if event.state == ElementState::Pressed => {
                let mut events: Vec<Event> = Vec::new();
                if let Some(key) = key_from_winit(&event.logical_key, self.mods.shift) {
                    events.extend(self.ui.key(key, self.mods));
                } else if let Some(text) = &event.text {
                    for ch in text.chars().filter(|ch| !ch.is_control()) {
                        events.extend(self.ui.key(Key::Char(ch), self.mods));
                    }
                }
                self.handle(&events);
            }
            WindowEvent::RedrawRequested => self.frame(),
            _ => {}
        }
    }

    /// The reloader's wake (`run`): the page's file was saved. The loop
    /// sleeps between events, so a still page draws nothing and uses no
    /// CPU; this is what brings it back for a saved file.
    fn user_event(&mut self, _event_loop: &ActiveEventLoop, _event: ()) {
        match self.ui.reload_if_saved() {
            Ok(true) => {
                self.app.show(&mut self.ui);
                self.redraw();
            }
            Ok(false) => {}
            Err(why) => eprintln!("the saved page cannot be shown, keeping the last one:\n{why}"),
        }
    }
}

/// Shows `ui` in a window and runs `app` on it until the window is closed.
///
/// On Windows the GPU starts on Direct3D 12 alone, which is the fastest
/// start there, and on every backend when that finds no GPU. The
/// `WGPU_BACKEND` variable names one backend to use instead.
///
/// WARNING: call it on the program's main thread; it does not return
/// before the window is closed.
///
/// Returns why no window or no GPU device could be had.
pub fn run<A: App>(options: WindowOptions<'_>, mut ui: Ui, mut app: A) -> Result<(), String> {
    let event_loop = EventLoop::new().map_err(|e| format!("no event loop: {e}"))?;
    let proxy = event_loop.create_proxy();
    ui.on_file_saved(move || {
        let _ = proxy.send_event(());
    });
    app.show(&mut ui);
    let mut host = Host { options, ui, app, gfx: None, cursor: (0.0, 0.0), is_mouse_down: false, mods: Mods::NONE, started: Instant::now(), last_frame: Instant::now(), failed: None };
    event_loop.run_app(&mut host).map_err(|e| format!("the event loop stopped: {e}"))?;
    match host.failed {
        Some(why) => Err(why),
        None => Ok(()),
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
