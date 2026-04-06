use std::sync::Arc;

use crate::state::State;

use wasm_bindgen::prelude::*;
use winit::{
  application::ApplicationHandler,
  event::*,
  event_loop::{ActiveEventLoop, EventLoop},
  keyboard::{KeyCode, PhysicalKey},
  platform::web::WindowAttributesExtWebSys,
  window::Window,
};

pub struct App {
  proxy: winit::event_loop::EventLoopProxy<State>,
  state: Option<State>,
  mouse_position: (f64, f64),
}

const CANVAS_ID: &str = "canvas";

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
      } => match (code, key_state.is_pressed()) {
        (KeyCode::Escape, true) => event_loop.exit(),
        _ => {}
      },
      _ => {}
    }
  }
}
