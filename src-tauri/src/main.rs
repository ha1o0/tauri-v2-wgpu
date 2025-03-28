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

mod renderer;

use renderer::Renderer;

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
                        required_limits: wgpu::Limits::downlevel_webgl2_defaults()
                            .using_resolution(adapter.limits()),
                        memory_hints: Default::default(), // 新增字段
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
            let renderer = Renderer::new(surface, device, queue, render_pipeline, config);
            app.manage(renderer);

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![greet, toggle_rendering])
        .build(tauri::generate_context!())
        .expect("error while building tauri application")
        .run(|app_handle, event| match event {
            RunEvent::WindowEvent {
                label: _,
                event: WindowEvent::Resized(size),
                ..
            } => {
                let renderer = app_handle.state::<Renderer>();
                renderer.resize(
                    if size.width > 0 { size.width } else { 1 },
                    if size.height > 0 { size.height } else { 1 },
                );
            }
            RunEvent::MainEventsCleared => {
                let render_state = app_handle.state::<Mutex<bool>>();
                let render_state = render_state.lock().unwrap();
                let renderer = app_handle.state::<Renderer>();
                renderer.render(*render_state);
            }
            _ => (),
        });
}
