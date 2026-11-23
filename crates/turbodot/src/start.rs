//! A `Program` in a window.

use std::sync::Arc;
use std::time::Instant;

use gogo_ui::{Key, Mods, Phase};
use winit::application::ApplicationHandler;
use winit::dpi::LogicalSize;
use winit::event::{ElementState, MouseButton, MouseScrollDelta, WindowEvent};
use winit::event_loop::{ActiveEventLoop, EventLoop};
use winit::keyboard::{Key as WinitKey, NamedKey};
use winit::window::{Window, WindowId};

use crate::program::Program;
use crate::window::{Gfx, TOUCHPAD_PIXELS_PER_NOTCH, WindowOptions, key_from_winit};

/// Window's side of a running program.
struct ProgramWindow<'a, A> {
    options: WindowOptions<'a>,
    program: Program<A>,
    gfx: Option<Gfx>,
    /// The mouse, in device pixels.
    cursor: (f32, f32),
    is_mouse_down: bool,
    mods: Mods,
    started: Instant,
    last_frame: Instant,
    /// Why window could not start, for `start` to return.
    failed: Option<String>,
}

impl<A: 'static> ProgramWindow<'_, A> {
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

    /// One mouse event into page on screen.
    fn pointer(&mut self, phase: Phase) {
        let (x, y) = self.cursor_epx();
        if phase == Phase::Move && !self.is_mouse_down {
            self.program.hover(x, y);
        } else {
            self.program.touch(phase, x, y, self.started.elapsed().as_secs_f64());
        }
        self.redraw();
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
        self.program.set_epx(epx);
        let is_animating: bool = self.program.frame(size.0 as f32 / epx, size.1 as f32 / epx, dt);

        self.program.gpu_draw(&mut gfx.renderer, &gfx.gpu);
        let is_animating: bool = is_animating || self.program.is_loading_pictures();
        gfx.renderer.prepare_pages(&gfx.gpu.device, &gfx.gpu.queue, &self.program, epx, size, &mut self.program.assets.text.borrow_mut(), None);
        let frame = match gfx.surface.get_current_texture() {
            wgpu::CurrentSurfaceTexture::Success(frame) | wgpu::CurrentSurfaceTexture::Suboptimal(frame) => frame,
            // A window that is hidden, being resized or lost its surface:
            // configure again and try on the next frame.
            _ => {
                gfx.surface.configure(&gfx.gpu.device, &gfx.config);
                gfx.window.request_redraw();
                return;
            }
        };
        let view = frame.texture.create_view(&Default::default());
        let mut encoder = gfx.gpu.device.create_command_encoder(&Default::default());
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
        gfx.gpu.queue.submit([encoder.finish()]);
        gfx.gpu.queue.present(frame);
        if is_animating {
            gfx.window.request_redraw();
        }
    }
}

impl<A: 'static> ApplicationHandler for ProgramWindow<'_, A> {
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
                    gfx.surface.configure(&gfx.gpu.device, &gfx.config);
                    gfx.window.request_redraw();
                }
            }
            WindowEvent::ScaleFactorChanged { .. } => self.redraw(),
            WindowEvent::CursorMoved { position, .. } => {
                self.cursor = (position.x as f32, position.y as f32);
                self.pointer(Phase::Move);
            }
            WindowEvent::CursorLeft { .. } => {
                self.program.unhover();
                self.redraw();
            }
            WindowEvent::MouseInput { state, button: MouseButton::Left, .. } => {
                self.is_mouse_down = state == ElementState::Pressed;
                self.pointer(if self.is_mouse_down { Phase::Down } else { Phase::Up });
            }
            // Mouse's own back button: back request, as Esc is.
            WindowEvent::MouseInput { state: ElementState::Pressed, button: MouseButton::Back, .. } => {
                self.program.back();
                self.redraw();
            }
            WindowEvent::MouseWheel { delta, .. } => {
                let notches: f32 = match delta {
                    MouseScrollDelta::LineDelta(_, lines) => lines,
                    MouseScrollDelta::PixelDelta(by) => by.y as f32 / (TOUCHPAD_PIXELS_PER_NOTCH * self.epx()),
                };
                let (x, y) = self.cursor_epx();
                self.program.wheel(x, y, notches);
                self.redraw();
            }
            WindowEvent::ModifiersChanged(mods) => {
                self.mods = Mods { shift: mods.state().shift_key(), ctrl: mods.state().control_key() };
            }
            WindowEvent::KeyboardInput { event, .. } if event.state == ElementState::Pressed => {
                if event.logical_key == WinitKey::Named(NamedKey::Escape) {
                    // PC has no back key: Esc is the back request. Does
                    // nothing on first page.
                    self.program.back();
                } else if let Some(key) = key_from_winit(&event.logical_key, self.mods.shift) {
                    self.program.key(key, self.mods);
                } else if let Some(text) = &event.text {
                    for ch in text.chars().filter(|ch| !ch.is_control()) {
                        self.program.key(Key::Char(ch), self.mods);
                    }
                }
                self.redraw();
            }
            WindowEvent::RedrawRequested => self.frame(),
            _ => {}
        }
    }

    /// Page's file was saved (`Program::on_page_file_saved`). Loop sleeps
    /// between events, so still pages draw nothing and use no CPU: this
    /// brings it back. Next frame shows the save.
    fn user_event(&mut self, _event_loop: &ActiveEventLoop, _event: ()) {
        self.redraw();
    }
}

/// Shows `program` in a window until window is closed. Esc and mouse's
/// back button are back requests.
///
/// On Windows the GPU starts on Direct3D 12 alone, which is the fastest
/// start there, and on every backend when that finds no GPU. The
/// `WGPU_BACKEND` variable names one backend to use instead.
///
/// WARNING: call it on the program's main thread; it does not return
/// before the window is closed.
///
/// Returns why no window or no GPU device could be had.
pub fn start<A: 'static>(options: WindowOptions<'_>, mut program: Program<A>) -> Result<(), String> {
    let event_loop = EventLoop::new().map_err(|e| format!("no event loop: {e}"))?;
    let proxy = event_loop.create_proxy();
    program.on_page_file_saved(move || {
        let _ = proxy.send_event(());
    });
    let mut window = ProgramWindow { options, program, gfx: None, cursor: (0.0, 0.0), is_mouse_down: false, mods: Mods::NONE, started: Instant::now(), last_frame: Instant::now(), failed: None };
    event_loop.run_app(&mut window).map_err(|e| format!("the event loop stopped: {e}"))?;
    match window.failed {
        Some(why) => Err(why),
        None => Ok(()),
    }
}
