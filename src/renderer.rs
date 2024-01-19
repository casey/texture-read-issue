use {
  super::*,
  wgpu::{
    include_wgsl, BindGroup, BindGroupDescriptor, BindGroupEntry, BindGroupLayout,
    BindGroupLayoutDescriptor, BindGroupLayoutEntry, BindingResource, BindingType, Buffer,
    BufferBinding, BufferBindingType, BufferDescriptor, BufferUsages, Color,
    CommandEncoderDescriptor, Device, DeviceDescriptor, Extent3d, Features, FragmentState,
    ImageCopyTexture, ImageDataLayout, Instance, Limits, LoadOp, MemoryHints, MultisampleState,
    Operations, Origin3d, PipelineCompilationOptions, PipelineLayoutDescriptor, PowerPreference,
    PrimitiveState, Queue, RenderPassColorAttachment, RenderPassDescriptor, RenderPipeline,
    RenderPipelineDescriptor, RequestAdapterOptions, Sampler, SamplerBindingType,
    SamplerDescriptor, ShaderStages, StoreOp, Surface, SurfaceConfiguration, Texture,
    TextureAspect, TextureDescriptor, TextureDimension, TextureFormat, TextureSampleType,
    TextureUsages, TextureView, TextureViewDescriptor, TextureViewDimension, VertexState,
  },
};

const SAMPLE_UNIFORM_BUFFER_SIZE: u64 = 8;

struct Uniforms {
  i: u32,
  resolution: f32,
}

impl Uniforms {
  fn data(&self) -> Vec<u8> {
    let mut data = Vec::new();
    data.extend(&self.i.to_le_bytes());
    data.extend(&self.resolution.to_le_bytes());
    data
  }
}

struct Target {
  bind_group: BindGroup,
  texture: Texture,
  texture_view: TextureView,
}

pub struct Renderer {
  bind_group_layout: BindGroupLayout,
  config: SurfaceConfiguration,
  device: Device,
  frame: u64,
  queue: Queue,
  render_pipeline: RenderPipeline,
  sampler: Sampler,
  surface: Surface<'static>,
  targets: Vec<Target>,
  texture_format: TextureFormat,
  uniform_buffer: Buffer,
}

impl Renderer {
  fn target(&self) -> Target {
    let texture = self.device.create_texture(&TextureDescriptor {
      label: None,
      size: Extent3d {
        width: self.config.width,
        height: self.config.height,
        depth_or_array_layers: 1,
      },
      mip_level_count: 1,
      sample_count: 1,
      dimension: TextureDimension::D2,
      format: self.texture_format,
      usage: TextureUsages::RENDER_ATTACHMENT
        | TextureUsages::TEXTURE_BINDING
        | TextureUsages::COPY_DST,
      view_formats: &[self.texture_format],
    });

    let texture_view = texture.create_view(&TextureViewDescriptor::default());

    let bind_group = self.device.create_bind_group(&BindGroupDescriptor {
      layout: &self.bind_group_layout,
      entries: &[
        BindGroupEntry {
          binding: 0,
          resource: BindingResource::TextureView(&texture_view),
        },
        BindGroupEntry {
          binding: 1,
          resource: BindingResource::Sampler(&self.sampler),
        },
        BindGroupEntry {
          binding: 2,
          resource: BindingResource::Buffer(BufferBinding {
            buffer: &self.uniform_buffer,
            offset: 0,
            size: None,
          }),
        },
      ],
      label: Some("target bind group"),
    });

    Target {
      bind_group,
      texture,
      texture_view,
    }
  }

  pub async fn new(window: Arc<Window>) -> Result<Self> {
    let mut size = window.inner_size();
    size.width = size.width.max(1);
    size.height = size.height.max(1);

    let instance = Instance::default();

    let surface = instance.create_surface(window)?;

    let adapter = instance
      .request_adapter(&RequestAdapterOptions {
        power_preference: PowerPreference::default(),
        force_fallback_adapter: false,
        compatible_surface: Some(&surface),
      })
      .await
      .context("failed to find an appropriate adapter")?;

    let (device, queue) = adapter
      .request_device(
        &DeviceDescriptor {
          label: Some("device"),
          required_features: Features::empty(),
          required_limits: Limits::default(),
          memory_hints: MemoryHints::Performance,
        },
        None,
      )
      .await
      .context("failed to create device")?;

    let texture_format = surface.get_capabilities(&adapter).formats[0];

    let shader = device.create_shader_module(include_wgsl!("shader.wgsl"));

    let config = surface
      .get_default_config(&adapter, size.width, size.height)
      .context("failed to get default config")?;

    surface.configure(&device, &config);

    let bind_group_layout = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
      entries: &[
        BindGroupLayoutEntry {
          binding: 0,
          count: None,
          ty: BindingType::Texture {
            multisampled: false,
            sample_type: TextureSampleType::Float { filterable: true },
            view_dimension: TextureViewDimension::D2,
          },
          visibility: ShaderStages::FRAGMENT,
        },
        BindGroupLayoutEntry {
          binding: 1,
          count: None,
          ty: BindingType::Sampler(SamplerBindingType::Filtering),
          visibility: ShaderStages::FRAGMENT,
        },
        BindGroupLayoutEntry {
          binding: 2,
          count: None,
          ty: BindingType::Buffer {
            has_dynamic_offset: false,
            min_binding_size: Some(SAMPLE_UNIFORM_BUFFER_SIZE.try_into().unwrap()),
            ty: BufferBindingType::Uniform,
          },
          visibility: ShaderStages::FRAGMENT,
        },
      ],
      label: Some("sample bind group layout"),
    });

    let sampler = device.create_sampler(&SamplerDescriptor::default());

    let uniform_buffer = device.create_buffer(&BufferDescriptor {
      label: Some("sample uniform buffer"),
      mapped_at_creation: false,
      size: SAMPLE_UNIFORM_BUFFER_SIZE,
      usage: BufferUsages::COPY_DST | BufferUsages::UNIFORM,
    });

    let pipeline_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
      bind_group_layouts: &[&bind_group_layout],
      label: Some("sample pipeline layout"),
      push_constant_ranges: &[],
    });

    let render_pipeline = device.create_render_pipeline(&RenderPipelineDescriptor {
      cache: None,
      depth_stencil: None,
      fragment: Some(FragmentState {
        compilation_options: PipelineCompilationOptions::default(),
        entry_point: Some("fragment"),
        module: &shader,
        targets: &[Some(texture_format.into())],
      }),
      label: Some("sample pipeline"),
      layout: Some(&pipeline_layout),
      multisample: MultisampleState::default(),
      multiview: None,
      primitive: PrimitiveState::default(),
      vertex: VertexState {
        buffers: &[],
        compilation_options: PipelineCompilationOptions::default(),
        entry_point: Some("vertex"),
        module: &shader,
      },
    });

    let mut renderer = Renderer {
      bind_group_layout,
      config,
      device,
      frame: 0,
      queue,
      render_pipeline,
      sampler,
      surface,
      targets: Vec::with_capacity(2),
      texture_format,
      uniform_buffer,
    };

    renderer.targets.push(renderer.target());
    renderer.targets.push(renderer.target());

    Ok(renderer)
  }

  pub(crate) fn render(&mut self) -> Result {
    let mut encoder = self
      .device
      .create_command_encoder(&CommandEncoderDescriptor::default());

    let frame = self
      .surface
      .get_current_texture()
      .context("failed to acquire next swap chain texture")?;

    // zero initialize target texture 0
    self.queue.write_texture(
      ImageCopyTexture {
        texture: &self.targets[0].texture,
        mip_level: 0,
        origin: Origin3d::ZERO,
        aspect: TextureAspect::All,
      },
      &vec![
        0x00;
        (self.config.width * self.config.height * 4)
          .try_into()
          .unwrap()
      ],
      ImageDataLayout {
        offset: 0,
        bytes_per_row: Some(self.config.width * 4),
        rows_per_image: Some(self.config.height),
      },
      Extent3d {
        width: self.config.width,
        height: self.config.height,
        depth_or_array_layers: 1,
      },
    );

    // render from target texture 0 to target texture 1
    // pass is 0, so shader should invert the input black color to white
    {
      let uniforms = Uniforms {
        i: 0,
        resolution: self.config.width.max(self.config.height) as f32,
      };

      self
        .queue
        .write_buffer(&self.uniform_buffer, 0, &uniforms.data());

      let mut pass = encoder.begin_render_pass(&RenderPassDescriptor {
        color_attachments: &[Some(RenderPassColorAttachment {
          ops: Operations {
            load: LoadOp::Clear(Color::BLACK),
            store: StoreOp::Store,
          },
          resolve_target: None,
          view: &self.targets[1].texture_view,
        })],
        depth_stencil_attachment: None,
        label: Some("filter render pass"),
        occlusion_query_set: None,
        timestamp_writes: None,
      });

      pass.set_bind_group(0, Some(&self.targets[0].bind_group), &[]);
      pass.set_pipeline(&self.render_pipeline);
      pass.draw(0..3, 0..1);
    }

    // render from target texture 1 to current surface texture
    // pass is 1, so shader should pass through color from target texture 1
    {
      let uniforms = Uniforms {
        i: 1,
        resolution: self.config.width.max(self.config.height) as f32,
      };

      self
        .queue
        .write_buffer(&self.uniform_buffer, 0, &uniforms.data());

      let view = frame.texture.create_view(&TextureViewDescriptor::default());

      let mut pass = encoder.begin_render_pass(&RenderPassDescriptor {
        color_attachments: &[Some(RenderPassColorAttachment {
          ops: Operations {
            load: LoadOp::Clear(Color::BLACK),
            store: StoreOp::Store,
          },
          resolve_target: None,
          view: &view,
        })],
        depth_stencil_attachment: None,
        label: Some("final render pass"),
        occlusion_query_set: None,
        timestamp_writes: None,
      });

      pass.set_bind_group(0, Some(&self.targets[1].bind_group), &[]);
      pass.set_pipeline(&self.render_pipeline);
      pass.draw(0..3, 0..1);
    }

    self.queue.submit([encoder.finish()]);

    frame.present();

    self.frame += 1;

    Ok(())
  }

  pub(crate) fn resize(&mut self, size: PhysicalSize<u32>) {
    self.config.width = size.width.max(1);
    self.config.height = size.height.max(1);
    self.surface.configure(&self.device, &self.config);
    self.targets[0] = self.target();
    self.targets[1] = self.target();
  }
}
