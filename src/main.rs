/*
    学习文章
    https://jinleili.github.io/learn-wgpu-zh/beginner/tutorial1-window#%E6%B7%BB%E5%8A%A0%E5%AF%B9-web-%E7%9A%84%E6%94%AF%E6%8C%81
*/
use parking_lot::Mutex;
use std::sync::Arc;
use wgpu::{include_wgsl, util::DeviceExt};
use winit::{
    application::ApplicationHandler,
    dpi::PhysicalSize,
    event::{ElementState, KeyEvent, WindowEvent},
    event_loop::{ActiveEventLoop, EventLoop},
    keyboard::{KeyCode, PhysicalKey},
    window::{Window, WindowId},
};
mod texture;

// 第四章 缓冲区与索引
#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
struct Vertex {
    position: [f32; 3],
    tex_coords: [f32; 2],
}
// 第四章 缓冲区与索引
const VERTICES: &[Vertex] = &[
    // 修改后的
    Vertex {
        position: [-0.0868241, 0.49240386, 0.0],
        tex_coords: [0.4131759, 0.00759614],
    }, // A
    Vertex {
        position: [-0.49513406, 0.06958647, 0.0],
        tex_coords: [0.0048659444, 0.43041354],
    }, // B
    Vertex {
        position: [-0.21918549, -0.44939706, 0.0],
        tex_coords: [0.28081453, 0.949397],
    }, // C
    Vertex {
        position: [0.35966998, -0.3473291, 0.0],
        tex_coords: [0.85967, 0.84732914],
    }, // D
    Vertex {
        position: [0.44147372, 0.2347359, 0.0],
        tex_coords: [0.9414737, 0.2652641],
    }, // E
];
// 第四章 缓冲区与索引
const INDICES: &[u16] = &[0, 1, 4, 1, 2, 4, 2, 3, 4];

// 第四章 缓冲区与索引
impl Vertex {
    fn desc<'a>() -> wgpu::VertexBufferLayout<'a> {
        wgpu::VertexBufferLayout {
            array_stride: core::mem::size_of::<Vertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 0,
                    format: wgpu::VertexFormat::Float32x3,
                },
                wgpu::VertexAttribute {
                    offset: core::mem::size_of::<[f32; 3]>() as wgpu::BufferAddress,
                    shader_location: 1,
                    // // 第五章-纹理和绑定组
                    format: wgpu::VertexFormat::Float32x2,
                },
            ],
        }
    }
}

struct WgpuApp {
    window: Arc<Window>,
    surface: wgpu::Surface<'static>,
    _adapter: wgpu::Adapter,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    size: winit::dpi::PhysicalSize<u32>,
    size_changed: bool,
    clear_color: wgpu::Color,
    // 第3章渲染管线
    render_pipeline: wgpu::RenderPipeline,
    // 第4章 缓冲区与索引
    vertex_buffer: wgpu::Buffer,
    num_vertices: u32,
    index_buffer: wgpu::Buffer,
    num_indices: u32,
    // 第五章-纹理和绑定组
    diffuse_bind_group: wgpu::BindGroup,
    diffuse_texture: texture::Texture,
}

impl WgpuApp {
    async fn new(window: Arc<Window>) -> Self {
        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
            backends: wgpu::Backends::all(),
            ..Default::default()
        });
        let surface = instance.create_surface(window.clone()).unwrap();

        // 第五章-纹理和绑定组
        let diffuse_bytes = include_bytes!("happy-tree.png");
        // let diffuse_image = image::load_from_memory(diffuse_bytes).unwrap();
        // let diffuse_rgba = diffuse_image.to_rgba8();
        // 第五章-纹理和绑定组
        // use image::GenericImageView;
        // let dimensions = diffuse_image.dimensions();
        // let texture_size = wgpu::Extent3d {
        //     width: dimensions.0,
        //     height: dimensions.1,
        //     depth_or_array_layers: 1,
        // };

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::default(),
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await
            .unwrap();
        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                required_features: wgpu::Features::empty(),
                required_limits: wgpu::Limits::defaults(),
                label: None,
                memory_hints: wgpu::MemoryHints::Performance,
                trace: wgpu::Trace::Off,
                // 项目依赖：wgpu 27.0.0 新增了实验性特征, 需要添加这一行
                // experimental_features: wgpu::ExperimentalFeatures::disabled(),
            })
            .await
            .unwrap();
        let caps = surface.get_capabilities(&adapter);

        // 这一段。。在第二章中没有提及，翻了一下源码中，有这三行
        let mut size = window.inner_size();
        size.width = size.width.max(1);
        size.height = size.height.max(1);

        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: caps.formats[0],
            width: size.width,
            height: size.height,
            present_mode: wgpu::PresentMode::Fifo,
            alpha_mode: caps.alpha_modes[0],
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };
        surface.configure(&device, &config);
        let clear_color = wgpu::Color::BLACK;

        // 第4章 缓冲区与索引
        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Vertex Buffer"),
            contents: bytemuck::cast_slice(VERTICES),
            usage: wgpu::BufferUsages::VERTEX,
        });

        let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Index Buffer"),
            contents: bytemuck::cast_slice(INDICES),
            usage: wgpu::BufferUsages::INDEX,
        });
        let num_indices = INDICES.len() as u32;

        // 第五章-纹理和绑定组
        let diffuse_texture =
            texture::Texture::from_byts(&device, &queue, diffuse_bytes, "happy_tree.png").unwrap();

        // 第五章-纹理和绑定组
        let texture_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            multisampled: false,
                            view_dimension: wgpu::TextureViewDimension::D2,
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                        count: None,
                    },
                ],
                label: Some("Texture_bind_group_layout"),
            });

        let diffuse_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &texture_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&diffuse_texture.view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&diffuse_texture.sampler),
                },
            ],
            label: Some("diffuse_bind_group"),
        });

        // 第3章渲染管线内容
        let shader = device.create_shader_module(include_wgsl!("shader.wgsl"));
        let render_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Render Pipeline Layout"),
                bind_group_layouts: &[&texture_bind_group_layout],
                push_constant_ranges: &[],
            });

        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Render Pipeline"),
            layout: Some(&render_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                compilation_options: Default::default(),
                entry_point: Some("vs_main"),
                // 第4章 缓冲区与索引
                buffers: &[Vertex::desc()],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                compilation_options: Default::default(),
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: config.format,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList, // 1.
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw, // 2.
                cull_mode: Some(wgpu::Face::Back),
                // 将此设置为 Fill 以外的任何值都要需要开启 Feature::NON_FILL_POLYGON_MODE
                polygon_mode: wgpu::PolygonMode::Fill,
                // 需要开启 Features::DEPTH_CLIP_CONTROL
                unclipped_depth: false,
                // 需要开启 Features::CONSERVATIVE_RASTERIZATION
                conservative: false,
            },
            depth_stencil: None, // 1.
            multisample: wgpu::MultisampleState {
                count: 1,                         // 2.
                mask: !0,                         // 3.
                alpha_to_coverage_enabled: false, // 4.
            },
            multiview: None, // 5.
            cache: None,
        });

        // 第四章 缓冲区与索引
        let num_vertices = VERTICES.len() as u32;

        Self {
            window,
            surface,
            device,
            queue,
            config,
            size,
            // 第二章-以下两个参数，同上的size，没有在第二章中提及
            size_changed: false,
            _adapter: adapter,
            clear_color,
            // 第三章-渲染管线
            render_pipeline,
            // 第四章-缓冲区与索引
            vertex_buffer,
            num_vertices,
            index_buffer,
            num_indices,
            // 第五章-纹理和绑定组
            diffuse_bind_group,
            diffuse_texture,
        }
    }

    fn keyboard_input(&mut self, event: &KeyEvent) -> bool {
        if event.physical_key == PhysicalKey::Code(KeyCode::Space) {
            self.clear_color = if event.state == ElementState::Released {
                wgpu::Color::BLACK
            } else {
                wgpu::Color::WHITE
            };
            return true;
        }
        false
    }

    fn set_window_resized(&mut self, new_size: PhysicalSize<u32>) {
        if new_size == self.size {
            return;
        }
        self.size = new_size;
        self.size_changed = true;
    }

    fn resize_surface_if_needed(&mut self) {
        if self.size_changed {
            self.config.width = self.size.width;
            self.config.height = self.size.height;
            self.surface.configure(&self.device, &self.config);
            self.size_changed = false;
        }
    }

    fn render(&mut self) -> Result<(), wgpu::SurfaceError> {
        if self.size.width == 0 || self.size.height == 0 {
            return Ok(());
        }
        self.resize_surface_if_needed();

        let output = self.surface.get_current_texture()?;
        let view = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Render Encoder"),
            });

        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    depth_slice: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.1,
                            g: 0.2,
                            b: 0.3,
                            a: 1.0,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                ..Default::default()
            });

            render_pass.set_pipeline(&self.render_pipeline);
            // 第五章-纹理和绑定组
            render_pass.set_bind_group(0, &self.diffuse_bind_group, &[]);
            // 第四章 缓冲区与索引
            render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
            render_pass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint16);
            // render_pass.draw(0..self.num_vertices, 0..1);
            render_pass.draw_indexed(0..self.num_indices, 0, 0..1);
        }

        self.queue.submit(Some(encoder.finish()));
        output.present();

        Ok(())
    }
}

#[derive(Default)]
struct WgpuAppHandler {
    app: Arc<Mutex<Option<WgpuApp>>>,
    #[allow(dead_code)]
    missed_resize: Arc<Mutex<Option<PhysicalSize<u32>>>>,
}

impl ApplicationHandler for WgpuAppHandler {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.app.lock().is_some() {
            return;
        }
        let window_attributes = Window::default_attributes().with_title("demo2");
        let window = Arc::new(event_loop.create_window(window_attributes).unwrap());
        let wgpu_app = pollster::block_on(WgpuApp::new(window));
        self.app.lock().replace(wgpu_app);
    }

    /// 暂停事件
    fn suspended(&mut self, _event_loop: &ActiveEventLoop) {}

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _window_id: WindowId,
        event: WindowEvent,
    ) {
        let mut app = self.app.lock();
        if app.as_ref().is_none() {
            // 如果 app 还没有初始化完成，则记录错失的窗口事件
            if let WindowEvent::Resized(physical_size) = event
                && physical_size.width > 0
                && physical_size.height > 0
            {
                let mut missed_resize = self.missed_resize.lock();
                *missed_resize = Some(physical_size);
            }
            return;
        }

        let app = app.as_mut().unwrap();
        // 窗口事件
        match event {
            WindowEvent::KeyboardInput { event, .. } => {
                app.keyboard_input(&event);
            }
            WindowEvent::CloseRequested => {
                event_loop.exit();
            }
            WindowEvent::Resized(physical_size) => {
                if physical_size.width == 0 || physical_size.height == 0 {
                    // 处理最小化窗口的事件
                    log::info!("Window minimized!");
                } else {
                    log::info!("Window resized: {:?}", physical_size);

                    app.set_window_resized(physical_size);
                }
            }
            WindowEvent::RedrawRequested => {
                // surface 重绘事件
                app.window.pre_present_notify();

                match app.render() {
                    Ok(_) => {}
                    // 当展示平面的上下文丢失，就需重新配置
                    Err(wgpu::SurfaceError::Lost) => eprintln!("Surface is lost"),
                    Err(wgpu::SurfaceError::Outdated) => {
                        // 本章实现的屏幕如果最小化后会在终端刷屏打印Outdated，所以在此处暂时这样处理
                    }
                    // 所有其他错误（过期、超时等）应在下一帧解决
                    Err(e) => eprintln!("{e:?}"),
                }
                // 除非我们手动请求，RedrawRequested 将只会触发一次。
                app.window.request_redraw();
            }
            _ => (),
        }
    }
}

fn main() -> Result<(), impl std::error::Error> {
    let events_loop = EventLoop::new().unwrap();
    let mut app = WgpuAppHandler::default();
    events_loop.run_app(&mut app)
}
