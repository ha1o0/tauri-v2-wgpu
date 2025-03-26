use std::{borrow::Cow, sync::Mutex};
use tauri::async_runtime::block_on;
use wgpu;

pub struct Renderer<'a> {
    surface: wgpu::Surface<'a>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    render_pipeline: wgpu::RenderPipeline,
    config: Mutex<wgpu::SurfaceConfiguration>,
}

// 在 renderer.rs 中添加
pub fn init_wgpu(window: &tauri::WebviewWindow) -> Result<Renderer, Box<dyn std::error::Error>> {
    let size = window.inner_size()?;
    let instance = wgpu::Instance::default();
    let surface = instance.create_surface(window)?;

    let adapter = block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
        power_preference: wgpu::PowerPreference::default(),
        compatible_surface: Some(&surface),
        ..Default::default()
    }))
    .expect("Failed to find adapter");

    let (device, queue) =
        block_on(adapter.request_device(&wgpu::DeviceDescriptor::default(), None))?;

    // 着色器代码（蓝色三角形）
    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: None,
        source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(
            r#"
@vertex
fn vs_main(@builtin(vertex_index) in_vertex_index: u32) -> @builtin(position) vec4<f32> {
    let x = f32(i32(in_vertex_index) - 1);
    let y = f32(i32(in_vertex_index & 1u) * 2 - 1);
    return vec4(x, y, 0.0, 1.0);
}

@fragment
fn fs_main() -> @location(0) vec4<f32> {
    return vec4(0.0, 0.0, 1.0, 1.0); // 蓝色
}
"#,
        )),
    });

    let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: None,
        bind_group_layouts: &[],
        push_constant_ranges: &[],
    });

    let swapchain_capabilities = surface.get_capabilities(&adapter);
    let swapchain_format = swapchain_capabilities.formats[0];

    let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: None,
        layout: Some(&pipeline_layout),
        vertex: wgpu::VertexState {
            module: &shader,
            entry_point: Some("vs_main"), // 改为 Option
            buffers: &[],
            compilation_options: Default::default(), // 新增字段
        },
        fragment: Some(wgpu::FragmentState {
            module: &shader,
            entry_point: Some("fs_main"), // 改为 Option
            targets: &[Some(swapchain_format.into())],
            compilation_options: Default::default(), // 新增字段
        }),
        primitive: wgpu::PrimitiveState::default(),
        depth_stencil: None,
        multisample: wgpu::MultisampleState::default(),
        multiview: None,
        cache: None, // 新增字段
    });

    let config = wgpu::SurfaceConfiguration {
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        format: swapchain_format,
        width: size.width,
        height: size.height,
        present_mode: wgpu::PresentMode::Fifo,
        alpha_mode: swapchain_capabilities.alpha_modes[0],
        view_formats: vec![],
        desired_maximum_frame_latency: 2,
    };

    surface.configure(&device, &config);

    Ok(Renderer::new(
        surface,
        device,
        queue,
        render_pipeline,
        config,
    ))
}

impl<'a> Renderer<'a> {
    pub fn new(
        surface: wgpu::Surface<'a>,
        device: wgpu::Device,
        queue: wgpu::Queue,
        render_pipeline: wgpu::RenderPipeline,
        config: wgpu::SurfaceConfiguration,
    ) -> Self {
        Self {
            surface,
            device,
            queue,
            render_pipeline,
            config: Mutex::new(config),
        }
    }

    pub fn resize(&self, width: u32, height: u32) {
        let mut config = self.config.lock().unwrap();
        config.width = width;
        config.height = height;
        self.surface.configure(&self.device, &config);
    }

    pub fn render(&self, should_render: bool) {
        let frame = self
            .surface
            .get_current_texture()
            .expect("Failed to acquire next swap chain texture");
        let view = frame
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });

        if should_render {
            // 渲染红色三角形
            {
                let mut rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: None,
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view: &view,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Clear(wgpu::Color::WHITE),
                            store: wgpu::StoreOp::Store,
                        },
                    })],
                    depth_stencil_attachment: None,
                    timestamp_writes: None,
                    occlusion_query_set: None,
                });
                rpass.set_pipeline(&self.render_pipeline);
                rpass.draw(0..3, 0..1);
            }
        } else {
            // 只清除屏幕
            {
                let _rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: None,
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view: &view,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Clear(wgpu::Color::WHITE),
                            store: wgpu::StoreOp::Store,
                        },
                    })],
                    depth_stencil_attachment: None,
                    timestamp_writes: None,
                    occlusion_query_set: None,
                });
            }
        }

        self.queue.submit(Some(encoder.finish()));
        frame.present();
    }
}
