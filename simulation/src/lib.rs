use std::sync::Arc;
use wasm_bindgen::{prelude::*, JsCast};
use web_sys::{CanvasRenderingContext2d, HtmlCanvasElement};
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
  window: Arc<Window>,
  canvas: HtmlCanvasElement,
  ctx: CanvasRenderingContext2d,
  t: f64,
}

impl State {
  pub async fn new(window: Arc<Window>, canvas: HtmlCanvasElement) -> anyhow::Result<Self> {
    let ctx = canvas
      .get_context("2d")
      .unwrap_throw()
      .unwrap_throw()
      .dyn_into::<CanvasRenderingContext2d>()
      .unwrap_throw();

    Ok(Self {
      window,
      canvas,
      ctx,
      t: 0.0,
    })
  }

  pub fn resize(&mut self, width: u32, height: u32) {
    self.canvas.set_width(width);
    self.canvas.set_height(height);
  }

  fn draw_curve(&mut self, steps: i32, cx: f64, cy: f64, amplitude: f64, frequency: f64) {
    for i in 0..=steps {
      let x = i as f64;
      let phase = self.t + (x - cx) * frequency;
      let y = cy + amplitude * phase.sin();

      if i > 0 {
        let prev_x = ((i - 1) as f64 - 0.8) as f64;
        let prev_phase = self.t + (prev_x - cx) * frequency;
        let prev_y = cy + amplitude * prev_phase.sin();

        self.ctx.begin_path();
        self.ctx.set_stroke_style_str("rgb(243, 158, 1)");
        self.ctx.move_to(prev_x, prev_y);
        self.ctx.line_to(x, y);
        self.ctx.stroke();
      }
    }
  }

  pub fn render(&mut self, (mouse_x, mouse_y): (f64, f64)) {
    let width = self.canvas.width() as f64;
    let height = self.canvas.height() as f64;
    let cx = width / 2.0;
    let cy = height / 2.0;

    self.ctx.set_fill_style_str("rgb(0, 0, 0)");
    self.ctx.fill_rect(0.0, 0.0, width, height);

    let amplitude = height * 0.2 * (1.0 - ((mouse_y / height) * 2.0).abs());
    let frequency = 0.02 * (1.0 - ((mouse_x / width) * 2.0).abs());
    let speed = 0.03;

    self.ctx.begin_path();
    self.ctx.set_line_width(2.0);

    let steps = width as i32;
    self.draw_curve(steps, cx, cy, amplitude, frequency);

    self.t += speed;
    self.window.request_redraw();
  }
}

pub struct App {
  proxy: winit::event_loop::EventLoopProxy<State>,
  state: Option<State>,
  cursor_pos: (f64, f64),
}

impl App {
  pub fn new(event_loop: &EventLoop<State>) -> Self {
    Self {
      state: None,
      proxy: event_loop.create_proxy(),
      cursor_pos: (0.0, 0.0),
    }
  }
}

impl ApplicationHandler<State> for App {
  fn resumed(&mut self, event_loop: &ActiveEventLoop) {
    let window = wgpu::web_sys::window().unwrap_throw();
    let document = window.document().unwrap_throw();
    let canvas = document.get_element_by_id(CANVAS_ID).unwrap_throw();

    let canvas_element = canvas.unchecked_ref::<HtmlCanvasElement>().clone();
    let html_canvas_element = canvas.unchecked_into();
    let window_attributes = Window::default_attributes().with_canvas(Some(html_canvas_element));
    let window = Arc::new(event_loop.create_window(window_attributes).unwrap());

    let proxy = self.proxy.clone();

    wasm_bindgen_futures::spawn_local(async move {
      assert!(proxy
        .send_event(
          State::new(window, canvas_element)
            .await
            .expect("Unable to create state.")
        )
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
        self.cursor_pos = (position.x, position.y);
      }
      WindowEvent::CloseRequested => event_loop.exit(),
      WindowEvent::Resized(size) => state.resize(size.width, size.height),
      WindowEvent::RedrawRequested => {
        state.render(self.cursor_pos);
      }
      WindowEvent::KeyboardInput {
        event:
          KeyEvent {
            physical_key: PhysicalKey::Code(code),
            state,
            ..
          },
        ..
      } => match (code, state.is_pressed()) {
        (KeyCode::Escape, true) => event_loop.exit(),
        _ => {}
      },
      _ => {}
    }
  }
}

#[wasm_bindgen(start)]
pub fn run() {
  let event_loop = EventLoop::with_user_event().build().unwrap_throw();

  let app = App::new(&event_loop);
  event_loop.spawn_app(app);
}
