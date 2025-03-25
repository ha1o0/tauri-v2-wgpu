// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::{borrow::Cow, sync::Mutex};

use tauri::{async_runtime::block_on, Manager, RunEvent, WindowEvent};

// Learn more about Tauri commands at https://tauri.app/v1/guides/features/command
#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

// 添加一个命令来控制渲染状态
#[tauri::command]
fn toggle_rendering(state: bool, app_handle: tauri::AppHandle) -> Result<(), String> {
    let render_state = app_handle.state::<Mutex<bool>>();
    let mut render_state = render_state
        .lock()
        .map_err(|_| "无法获取渲染状态锁".to_string())?;
    *render_state = state;
    if let Some(window) = app_handle.get_webview_window("main") {
        window.reload().expect("Failed to reload window");
    }

    Ok(())
}

fn main() {
    tauri::Builder::default()
        .setup(|app| {
            // 初始化渲染状态为 false（不渲染）
            app.manage(Mutex::new(false));

            let window = app.get_webview_window("main").unwrap();
            let size = window.inner_size()?;

            let instance = wgpu::Instance::default();

            let surface = instance.create_surface(window).unwrap();
            let adapter = block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::default(),
                force_fallback_adapter: false,
                // Request an adapter which can render to our surface
                compatible_surface: Some(&surface),
            }))
            .expect("Failed to find an appropriate adapter");

            // Create the logical device and command queue
            let (device, queue) = block_on(
                adapter.request_device(
                    &wgpu::DeviceDescriptor {
                        label: None,
                        required_features: wgpu::Features::empty(),
                        // Make sure we use the texture resolution limits from the adapter, so we can support images the size of the swapchain.
                        required_limits: wgpu::Limits::downlevel_webgl2_defaults()
                            .using_resolution(adapter.limits()),
                    },
                    None,
                ),
            )
            .expect("Failed to create device");

            // Load the shaders from disk
            let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
                label: None,
                source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(
                    r#"
@vertex
fn vs_main(@builtin(vertex_index) in_vertex_index: u32) -> @builtin(position) vec4<f32> {
    let x = f32(i32(in_vertex_index) - 1);
    let y = f32(i32(in_vertex_index & 1u) * 2 - 1);
    return vec4<f32>(x, y, 0.0, 1.0);
}

@fragment
fn fs_main() -> @location(0) vec4<f32> {
    return vec4<f32>(1.0, 0.0, 0.0, 1.0);
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
                    entry_point: "vs_main",
                    buffers: &[],
                },
                fragment: Some(wgpu::FragmentState {
                    module: &shader,
                    entry_point: "fs_main",
                    targets: &[Some(swapchain_format.into())],
                }),
                primitive: wgpu::PrimitiveState::default(),
                depth_stencil: None,
                multisample: wgpu::MultisampleState::default(),
                multiview: None,
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

            app.manage(surface);
            app.manage(render_pipeline);
            app.manage(device);
            app.manage(queue);
            app.manage(Mutex::new(config));

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![greet, toggle_rendering])
        .build(tauri::generate_context!())
        .expect("error while building tauri application")
        .run(|app_handle, event| {
            match event {
                RunEvent::WindowEvent {
                    label: _,
                    event: WindowEvent::Resized(size),
                    ..
                } => {
                    let config = app_handle.state::<Mutex<wgpu::SurfaceConfiguration>>();
                    let surface = app_handle.state::<wgpu::Surface>();
                    let device = app_handle.state::<wgpu::Device>();

                    let mut config = config.lock().unwrap();
                    config.width = if size.width > 0 { size.width } else { 1 };
                    config.height = if size.height > 0 { size.height } else { 1 };
                    surface.configure(&device, &config)

                    // TODO: Request redraw on macos (not exposed in tauri yet).
                }
                RunEvent::MainEventsCleared => {
                    // 获取渲染状态
                    let render_state = app_handle.state::<Mutex<bool>>();
                    let render_state = render_state.lock().unwrap();

                    // 无论是否渲染，都需要获取这些资源
                    let surface = app_handle.state::<wgpu::Surface>();
                    let device = app_handle.state::<wgpu::Device>();
                    let queue = app_handle.state::<wgpu::Queue>();

                    let frame = surface
                        .get_current_texture()
                        .expect("Failed to acquire next swap chain texture");
                    let view = frame
                        .texture
                        .create_view(&wgpu::TextureViewDescriptor::default());
                    let mut encoder = device
                        .create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });

                    if *render_state {
                        // 渲染红色三角形
                        let render_pipeline = app_handle.state::<wgpu::RenderPipeline>();
                        {
                            let mut rpass =
                                encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
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
                            rpass.set_pipeline(&render_pipeline);
                            rpass.draw(0..3, 0..1);
                        }
                    } else {
                        // 只清除屏幕，不渲染三角形
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
                            // 不需要设置管线和绘制，只需要清屏
                        }
                    }

                    queue.submit(Some(encoder.finish()));
                    frame.present();
                }
                _ => (),
            }
        });
}
