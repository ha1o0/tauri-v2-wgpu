// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::{borrow::Cow, collections::HashMap, sync::Mutex};

use tauri::{async_runtime::block_on, Manager, RunEvent, State, WebviewWindowBuilder, WindowEvent};

// Learn more about Tauri commands at https://tauri.app/v1/guides/features/command
#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

struct AppState {
    // 使用窗口标签作为key的渲染器映射
    window_renderers: Mutex<HashMap<String, Renderer<'static>>>,
}

#[tauri::command]
async fn init_window_wgpu(app: tauri::AppHandle, window_label: String) -> Result<(), String> {
    println!("Initializing window {}", window_label);
    app.clone()
        .run_on_main_thread(move || {
            // 初始化渲染状态为 false（不渲染）
            // app.manage(Mutex::new(false));
            // println!("Initializing window {}", window_label);
            println!("1111");
            let window = app.get_webview_window(&window_label).unwrap();
            // println!("window: {:?}", window);
            let size = window.inner_size().unwrap();
            println!("size: {:?}", size);

            let instance = wgpu::Instance::default();

            let surface = instance.create_surface(window).unwrap();
            let adapter = block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::default(),
                force_fallback_adapter: false,
                // Request an adapter which can render to our surface
                compatible_surface: Some(&surface),
            }))
            .expect("Failed to find an appropriate adapter");
            println!("2222");
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
return vec4<f32>(0.0, 0.0, 1.0, 1.0);  // 修改为蓝色，与 renderer.rs 保持一致
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
            println!("3333");
            let app_state = app.state::<AppState>();
            app_state.window_renderers.lock().unwrap().insert(
                window_label,
                Renderer::new(surface, device, queue, render_pipeline, config),
            );
            println!("4444");
            ()
        })
        .map_err(|e| e.to_string())
}

// 添加一个命令来控制渲染状态
#[tauri::command]
fn toggle_rendering(
    window_label: String,
    state: bool,
    app_handle: tauri::AppHandle,
) -> Result<(), String> {
    println!(
        "Toggling rendering for window {}, state: {}",
        window_label, state
    );
    let app_state = app_handle.state::<AppState>();
    // 找到对应的渲染器并更新状态
    if let Some(renderer) = app_state
        .window_renderers
        .lock()
        .unwrap()
        .get_mut(&window_label)
    {
        println!("5555");
        renderer.render(state);
        if let Some(window) = app_handle.get_webview_window(&window_label) {
            println!("6666");
            window.reload().expect("Failed to reload window");
        }
    }
    Ok(())
}

mod renderer;

use renderer::Renderer;

fn main() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            greet,
            toggle_rendering,
            init_window_wgpu
        ])
        .setup(|app| {
            // 管理AppState
            app.manage(AppState {
                window_renderers: Mutex::new(HashMap::new()),
            });
            Ok(())
        })
        .build(tauri::generate_context!())
        .expect("error while building tauri application")
        .run(|app_handle, event| match event {
            RunEvent::WindowEvent {
                label: _,
                event: WindowEvent::Resized(size),
                ..
            } => {
                let app_state = app_handle.state::<AppState>();
                for (_, renderer) in app_state.window_renderers.lock().unwrap().iter() {
                    renderer.resize(
                        if size.width > 0 { size.width } else { 1 },
                        if size.height > 0 { size.height } else { 1 },
                    );
                }
            }
            RunEvent::MainEventsCleared => {
                let app_state = app_handle.state::<AppState>();
                for (window_label, renderer) in app_state.window_renderers.lock().unwrap().iter() {
                    println!("777 Rendering window {}", window_label);
                    renderer.render(true);
                }
            }
            _ => (),
        });
}
