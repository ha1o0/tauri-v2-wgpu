use wgpu;
use tauri::Manager;
use std::sync::Mutex;

pub struct Renderer<'a> {
    surface: wgpu::Surface<'a>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    render_pipeline: wgpu::RenderPipeline,
    config: Mutex<wgpu::SurfaceConfiguration>,
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