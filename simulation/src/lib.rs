mod state;
mod window;

use wasm_bindgen::prelude::*;
use winit::{event_loop::EventLoop, platform::web::EventLoopExtWebSys};

use crate::window::App;

#[wasm_bindgen(start)]
pub fn run() {
  std::panic::set_hook(Box::new(console_error_panic_hook::hook));

  let event_loop = EventLoop::with_user_event().build().unwrap_throw();

  let app = App::new(&event_loop);
  event_loop.spawn_app(app);
}
