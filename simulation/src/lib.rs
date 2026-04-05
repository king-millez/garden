use std::{iter, sync::Arc};
use wasm_bindgen::{prelude::*, JsCast};
use winit::{
  application::ApplicationHandler,
  event::*,
  event_loop::{ActiveEventLoop, EventLoop},
  keyboard::{KeyCode, PhysicalKey},
  platform::web::{EventLoopExtWebSys, WindowAttributesExtWebSys},
  window::Window,
};

const CANVAS_ID: &str = "canvas";

pub struct State {
  surface: wgpu::Surface<'static>,
  device: wgpu::Device,
  queue: wgpu::Queue,
  config: wgpu::SurfaceConfiguration,
  is_surface_configured: bool,
  window: Arc<Window>,
}

impl State {
  async fn new(window: Arc<Window>) -> anyhow::Result<State> {
    let size = window.inner_size();

    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
      backends: wgpu::Backends::GL,
      flags: Default::default(),
      memory_budget_thresholds: Default::default(),
      backend_options: Default::default(),
      display: None,
    });

    let surface = instance.create_surface(window.clone()).unwrap();

    let adapter = instance
      .request_adapter(&wgpu::RequestAdapterOptions {
        power_preference: wgpu::PowerPreference::default(),
        compatible_surface: Some(&surface),
        force_fallback_adapter: false,
      })
      .await?;

    let (device, queue) = adapter
      .request_device(&wgpu::DeviceDescriptor {
        label: None,
        required_features: wgpu::Features::empty(),
        experimental_features: wgpu::ExperimentalFeatures::disabled(),
        required_limits: wgpu::Limits::downlevel_webgl2_defaults(),
        memory_hints: Default::default(),
        trace: wgpu::Trace::Off,
      })
      .await?;

    let surface_caps = surface.get_capabilities(&adapter);

    let surface_format = surface_caps
      .formats
      .iter()
      .copied()
      .find(|f| f.is_srgb())
      .unwrap_or(surface_caps.formats[0]);

    let config = wgpu::SurfaceConfiguration {
      usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
      format: surface_format,
      width: size.width,
      height: size.height,
      present_mode: surface_caps.present_modes[0],
      alpha_mode: surface_caps.alpha_modes[0],
      desired_maximum_frame_latency: 2,
      view_formats: vec![],
    };

    Ok(Self {
      surface,
      device,
      queue,
      config,
      is_surface_configured: false,
      window,
    })
  }

  pub fn resize(&mut self, width: u32, height: u32) {
    if width > 0 && height > 0 {
      let max_dimension = self.device.limits().max_texture_dimension_2d;

      self.config.width = width.min(max_dimension);
      self.config.height = height.min(max_dimension);

      self.surface.configure(&self.device, &self.config);

      self.is_surface_configured = true;
    }
  }

  fn update(&mut self) {}

  fn render(&mut self, (mouse_x, mouse_y): (f64, f64)) -> anyhow::Result<()> {
    self.window.request_redraw();

    if !self.is_surface_configured {
      return Ok(());
    }

    let output = match self.surface.get_current_texture() {
      wgpu::CurrentSurfaceTexture::Success(surface_texture) => surface_texture,
      wgpu::CurrentSurfaceTexture::Suboptimal(surface_texture) => {
        self.surface.configure(&self.device, &self.config);
        surface_texture
      }
      wgpu::CurrentSurfaceTexture::Timeout
      | wgpu::CurrentSurfaceTexture::Occluded
      | wgpu::CurrentSurfaceTexture::Validation => {
        // Skip this frame
        return Ok(());
      }
      wgpu::CurrentSurfaceTexture::Outdated => {
        self.surface.configure(&self.device, &self.config);
        return Ok(());
      }
      wgpu::CurrentSurfaceTexture::Lost => {
        anyhow::bail!("Lost device");
      }
    };

    let view = output
      .texture
      .create_view(&wgpu::TextureViewDescriptor::default());

    let mut encoder = self
      .device
      .create_command_encoder(&wgpu::CommandEncoderDescriptor {
        label: Some("Render Encoder"),
      });

    {
      let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
        label: Some("Render Pass"),
        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
          view: &view,
          resolve_target: None,
          ops: wgpu::Operations {
            load: wgpu::LoadOp::Clear(wgpu::Color {
              r: mouse_y / self.config.height as f64,
              g: mouse_x / self.config.width as f64,
              b: (mouse_x + mouse_y) / (self.config.width + self.config.height) as f64,
              a: 1.0,
            }),
            store: wgpu::StoreOp::Store,
          },
          depth_slice: None,
        })],
        depth_stencil_attachment: None,
        occlusion_query_set: None,
        timestamp_writes: None,
        multiview_mask: None,
      });

      render_pass.set_viewport(
        0.0,
        0.0,
        self.config.width as f32,
        self.config.height as f32,
        0.0,
        1.0,
      );
    }

    self.queue.submit(iter::once(encoder.finish()));
    output.present();

    Ok(())
  }

  pub fn handle_key(&mut self, event_loop: &ActiveEventLoop, code: KeyCode, is_pressed: bool) {
    match (code, is_pressed) {
      (KeyCode::Escape, true) => event_loop.exit(),
      _ => {}
    }
  }
}

pub struct App {
  proxy: winit::event_loop::EventLoopProxy<State>,
  state: Option<State>,
  mouse_position: (f64, f64),
}

impl App {
  pub fn new(event_loop: &EventLoop<State>) -> Self {
    Self {
      state: None,
      proxy: event_loop.create_proxy(),
      mouse_position: (0.0, 0.0),
    }
  }
}

impl ApplicationHandler<State> for App {
  fn resumed(&mut self, event_loop: &ActiveEventLoop) {
    let window = wgpu::web_sys::window().unwrap_throw();
    let document = window.document().unwrap_throw();
    let canvas = document.get_element_by_id(CANVAS_ID).unwrap_throw();

    let html_canvas_element = canvas.unchecked_into();
    let window_attributes = Window::default_attributes().with_canvas(Some(html_canvas_element));
    let window = Arc::new(event_loop.create_window(window_attributes).unwrap());

    let proxy = self.proxy.clone();

    wasm_bindgen_futures::spawn_local(async move {
      assert!(proxy
        .send_event(State::new(window).await.expect("Unable to create state."))
        .is_ok())
    });
  }

  fn user_event(&mut self, _event_loop: &ActiveEventLoop, mut event: State) {
    event.window.request_redraw();

    event.resize(
      event.window.inner_size().width,
      event.window.inner_size().height,
    );

    self.state = Some(event);
  }

  fn window_event(
    &mut self,
    event_loop: &ActiveEventLoop,
    _window_id: winit::window::WindowId,
    event: WindowEvent,
  ) {
    let state = match &mut self.state {
      Some(canvas) => canvas,
      None => return,
    };

    match event {
      WindowEvent::CursorMoved { position, .. } => {
        self.mouse_position = (position.x, position.y);
      }
      WindowEvent::CloseRequested => event_loop.exit(),
      WindowEvent::Resized(size) => state.resize(size.width, size.height),
      WindowEvent::RedrawRequested => {
        state.update();

        match state.render(self.mouse_position) {
          Ok(_) => {}
          Err(e) => {
            log::error!("{e}");
            event_loop.exit();
          }
        }
      }
      WindowEvent::KeyboardInput {
        event:
          KeyEvent {
            physical_key: PhysicalKey::Code(code),
            state: key_state,
            ..
          },
        ..
      } => state.handle_key(event_loop, code, key_state.is_pressed()),
      _ => {}
    }
  }
}

#[wasm_bindgen(start)]
pub fn run() {
  std::panic::set_hook(Box::new(console_error_panic_hook::hook));

  let event_loop = EventLoop::with_user_event().build().unwrap_throw();

  let app = App::new(&event_loop);
  event_loop.spawn_app(app);
}
