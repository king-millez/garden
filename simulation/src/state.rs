use std::{iter, sync::Arc};
use winit::window::Window;

pub struct State {
  surface: wgpu::Surface<'static>,
  device: wgpu::Device,
  queue: wgpu::Queue,
  config: wgpu::SurfaceConfiguration,
  is_surface_configured: bool,
  render_pipeline: wgpu::RenderPipeline,
  pub window: Arc<Window>,
}

impl State {
  pub async fn new(window: Arc<Window>) -> anyhow::Result<State> {
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

    let shader = device.create_shader_module(wgpu::include_wgsl!("shaders/shader.wgsl"));

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

    let render_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
      label: Some("Render Pipeline Layout"),
      bind_group_layouts: &[],
      immediate_size: 0,
    });

    let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
      label: Some("Render Pipeline"),
      layout: Some(&render_pipeline_layout),
      vertex: wgpu::VertexState {
        module: &shader,
        entry_point: Some("vs_main"),
        buffers: &[],
        compilation_options: wgpu::PipelineCompilationOptions::default(),
      },
      fragment: Some(wgpu::FragmentState {
        module: &shader,
        entry_point: Some("fs_main"),
        targets: &[Some(wgpu::ColorTargetState {
          format: config.format,
          blend: Some(wgpu::BlendState::REPLACE),
          write_mask: wgpu::ColorWrites::ALL,
        })],
        compilation_options: wgpu::PipelineCompilationOptions::default(),
      }),
      primitive: wgpu::PrimitiveState {
        topology: wgpu::PrimitiveTopology::TriangleList,
        strip_index_format: None,
        front_face: wgpu::FrontFace::Ccw,
        cull_mode: Some(wgpu::Face::Back),
        polygon_mode: wgpu::PolygonMode::Fill,
        unclipped_depth: false,
        conservative: false,
      },
      depth_stencil: None,
      multisample: wgpu::MultisampleState {
        count: 1,
        mask: !0,
        alpha_to_coverage_enabled: false,
      },
      multiview_mask: None,
      cache: None,
    });

    Ok(Self {
      surface,
      device,
      queue,
      config,
      is_surface_configured: false,
      window,
      render_pipeline,
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

  pub fn update(&mut self) {}

  pub fn render(&mut self, (mouse_x, mouse_y): (f64, f64)) -> anyhow::Result<()> {
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

    render_pass.set_pipeline(&self.render_pipeline);
    render_pass.draw(0..3, 0..1);

    drop(render_pass);

    self.queue.submit(iter::once(encoder.finish()));
    output.present();

    Ok(())
  }
}
