/*
    学习文章
    https://jinleili.github.io/learn-wgpu-zh/beginner/tutorial1-window#%E6%B7%BB%E5%8A%A0%E5%AF%B9-web-%E7%9A%84%E6%94%AF%E6%8C%81
*/

use parking_lot::Mutex;
use std::{f32::consts, sync::Arc};
use wgpu::{include_wgsl, util::DeviceExt};
use winit::{
    application::ApplicationHandler,
    dpi::PhysicalSize,
    event::{ElementState, KeyEvent, WindowEvent},
    event_loop::{ActiveEventLoop, EventLoop},
    keyboard::{Key, KeyCode, NamedKey, PhysicalKey},
    window::{Window, WindowId},
};
mod model;
//第九章-模型加载
mod texture;
use model::Vertex;
mod resources;

// FPS帧率统计
use std::time::{Duration, Instant};

use crate::model::DrawModel;

struct Camera {
    eye: glam::Vec3,
    target: glam::Vec3,
    up: glam::Vec3,
    aspect: f32,
    fovy: f32,
    znear: f32,
    zfar: f32,
}

impl Camera {
    fn build_view_projection_matrix(&self) -> glam::Mat4 {
        // 1.
        let view = glam::Mat4::look_at_rh(self.eye, self.target, self.up);
        // 2.
        let proj =
            glam::Mat4::perspective_rh(self.fovy.to_radians(), self.aspect, self.znear, self.zfar);

        // 3.
        proj * view
    }
}

// 此属性标注数据的内存布局兼容 C-ABI，令其可用于着色器
#[repr(C)]
// derive 属性自动导入的这些 trait，令其可被存入缓冲区
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
struct CameraUniform {
    // glam 的数据类型不能直接用于 bytemuck
    // 需要先将 Matrix4 矩阵转为一个 4x4 的浮点数数组
    view_proj: [[f32; 4]; 4],
}

impl CameraUniform {
    fn new() -> Self {
        Self {
            view_proj: glam::Mat4::IDENTITY.to_cols_array_2d(),
        }
    }

    fn update_view_proj(&mut self, camera: &Camera) {
        self.view_proj = camera.build_view_projection_matrix().to_cols_array_2d();
    }
}

struct CameraController {
    speed: f32,
    is_forward_pressed: bool,
    is_backward_pressed: bool,
    is_left_pressed: bool,
    is_right_pressed: bool,
    is_up_pressed: bool,
    is_down_pressed: bool,
}

impl CameraController {
    fn new(speed: f32) -> Self {
        Self {
            speed,
            is_forward_pressed: false,
            is_backward_pressed: false,
            is_left_pressed: false,
            is_right_pressed: false,
            is_up_pressed: false,
            is_down_pressed: false,
        }
    }

    fn process_events(&mut self, event: &KeyEvent) -> bool {
        let is_pressed = event.state == ElementState::Pressed;

        // 处理逻辑键
        match &event.logical_key {
            Key::Named(NamedKey::Space) => {
                self.is_up_pressed = is_pressed;
                return true;
            }
            _ => {}
        }

        // 处理物理键
        match &event.physical_key {
            PhysicalKey::Code(KeyCode::ShiftLeft) => {
                self.is_down_pressed = is_pressed;
                true
            }
            PhysicalKey::Code(KeyCode::KeyW) | PhysicalKey::Code(KeyCode::ArrowUp) => {
                self.is_forward_pressed = is_pressed;
                true
            }
            PhysicalKey::Code(KeyCode::KeyA) | PhysicalKey::Code(KeyCode::ArrowLeft) => {
                self.is_left_pressed = is_pressed;
                true
            }
            PhysicalKey::Code(KeyCode::KeyS) | PhysicalKey::Code(KeyCode::ArrowDown) => {
                self.is_backward_pressed = is_pressed;
                true
            }
            PhysicalKey::Code(KeyCode::KeyD) | PhysicalKey::Code(KeyCode::ArrowRight) => {
                self.is_right_pressed = is_pressed;
                true
            }
            _ => false,
        }
    }

    fn update_camera(&self, camera: &mut Camera) {
        let forward = camera.target - camera.eye;
        let forward_norm = forward.normalize();
        let forward_mag = forward.length();

        // 防止摄像机离场景中心太近时出现问题
        if self.is_forward_pressed && forward_mag > self.speed {
            camera.eye += forward_norm * self.speed;
        }
        if self.is_backward_pressed {
            camera.eye -= forward_norm * self.speed;
        }

        let right = forward_norm.cross(camera.up);

        // 在按下前进或后退键时重做半径计算
        let forward = camera.target - camera.eye;
        let forward_mag = forward.length();

        if self.is_right_pressed {
            // 重新调整目标和眼睛之间的距离，以便其不发生变化。
            // 因此，眼睛仍然位于目标和眼睛形成的圆圈上。
            camera.eye = camera.target - (forward + right * self.speed).normalize() * forward_mag;
        }
        if self.is_left_pressed {
            camera.eye = camera.target - (forward - right * self.speed).normalize() * forward_mag;
        }
    }
}

struct Instance {
    position: glam::Vec3,
    rotation: glam::Quat,
}
#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
struct InstanceRaw {
    model: [[f32; 4]; 4],
}
// 第七章-新增!
impl Instance {
    fn to_raw(&self) -> InstanceRaw {
        InstanceRaw {
            model: (glam::Mat4::from_translation(self.position)
                * glam::Mat4::from_quat(self.rotation))
            .to_cols_array_2d(),
        }
    }
}
impl InstanceRaw {
    fn desc<'a>() -> wgpu::VertexBufferLayout<'a> {
        use core::mem;
        wgpu::VertexBufferLayout {
            array_stride: mem::size_of::<InstanceRaw>() as wgpu::BufferAddress,
            // step_mode 的值需要从 Vertex 改为 Instance
            // 这意味着只有着色器开始处理一次新实例化绘制时，才会使用下一个实例数据
            step_mode: wgpu::VertexStepMode::Instance,
            attributes: &[
                wgpu::VertexAttribute {
                    offset: 0,
                    // 虽然顶点着色器现在只使用了插槽 0 和 1，但在后面的教程中将会使用 2、3 和 4
                    // 此处从插槽 5 开始，确保与后面的教程不会有冲突
                    shader_location: 5,
                    format: wgpu::VertexFormat::Float32x4,
                },
                // mat4 从技术的角度来看是由 4 个 vec4 构成，占用 4 个插槽。
                // 我们需要为每个 vec4 定义一个插槽，然后在着色器中重新组装出 mat4。
                wgpu::VertexAttribute {
                    offset: mem::size_of::<[f32; 4]>() as wgpu::BufferAddress,
                    shader_location: 6,
                    format: wgpu::VertexFormat::Float32x4,
                },
                wgpu::VertexAttribute {
                    offset: mem::size_of::<[f32; 8]>() as wgpu::BufferAddress,
                    shader_location: 7,
                    format: wgpu::VertexFormat::Float32x4,
                },
                wgpu::VertexAttribute {
                    offset: mem::size_of::<[f32; 12]>() as wgpu::BufferAddress,
                    shader_location: 8,
                    format: wgpu::VertexFormat::Float32x4,
                },
            ],
        }
    }
}

#[repr(C)]
#[derive(Copy, Clone, Debug, Hash, Eq, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum CompareFunction {
    Undefined = 0,
    Never = 1,
    Less = 2,
    Equal = 3,
    LessEqual = 4,
    Greater = 5,
    NotEqual = 6,
    GreaterEqual = 7,
    Always = 8,
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
    // clear_color: wgpu::Color,
    // 第3章渲染管线
    render_pipeline: wgpu::RenderPipeline,
    // 第4章 缓冲区与索引
    // vertex_buffer: wgpu::Buffer,
    // num_vertices: u32,
    // index_buffer: wgpu::Buffer,
    // num_indices: u32,
    // 第五章-纹理和绑定组
    diffuse_bind_group: wgpu::BindGroup,
    diffuse_texture: texture::Texture,
    camera: Camera,
    camera_uniform: CameraUniform,
    camera_buffer: wgpu::Buffer,
    camera_bind_group: wgpu::BindGroup,
    camera_controller: CameraController,
    // 帧率统计
    frame_count: u32,
    last_fps_update: Instant,
    last_frame_time: Instant,
    fps: f32,
    // 第七章-新增!
    instances: Vec<Instance>,
    instance_buffer: wgpu::Buffer,
    // 第八章-深度缓冲区
    depth_texture: texture::Texture,
    // 第九章-new
    obj_model: model::Model,
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

        // 第五章-纹理和绑定组
        let diffuse_texture =
            texture::Texture::from_bytes(&device, &queue, diffuse_bytes, "happy_tree.png").unwrap();

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

        // 第四章 缓冲区与索引
        // let num_vertices = VERTICES.len() as u32;

        let camera = Camera {
            // 将摄像机向上移动 1 个单位，向后移动 2 个单位
            // +z 朝向屏幕外
            eye: (0.0, 1.0, 2.0).into(),
            // 摄像机看向原点
            target: (0.0, 0.0, 0.0).into(),
            // 定义哪个方向朝上
            up: glam::Vec3::Y,
            aspect: config.width as f32 / config.height as f32,
            fovy: 45.0,
            znear: 0.1,
            zfar: 100.0,
        };
        let mut camera_uniform = CameraUniform::new();
        camera_uniform.update_view_proj(&camera);

        let camera_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Camera Buffer"),
            contents: bytemuck::cast_slice(&[camera_uniform]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });
        let camera_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX, // 1
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false, // 2
                        min_binding_size: None,
                    },
                    count: None,
                }],
                label: Some("camera_bind_group_layout"),
            });

        let camera_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &camera_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: camera_buffer.as_entire_binding(),
            }],
            label: Some("camera_bind_group"),
        });

        let render_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Render Pipeline Layout"),
                bind_group_layouts: &[&texture_bind_group_layout, &camera_bind_group_layout],
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
                // buffers: &[Vertex::desc(), InstanceRaw::desc()],
                buffers: &[model::ModelVertex::desc(), InstanceRaw::desc()],
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
            // 第八章-深度缓冲区-new
            depth_stencil: Some(wgpu::DepthStencilState {
                format: texture::Texture::DEPTH_FORMAT,
                depth_write_enabled: true,
                depth_compare: wgpu::CompareFunction::Less, // 1.
                stencil: wgpu::StencilState::default(),     // 2.
                bias: wgpu::DepthBiasState::default(),
            }), // 1.
            multisample: wgpu::MultisampleState {
                count: 1,                         // 2.
                mask: !0,                         // 3.
                alpha_to_coverage_enabled: false, // 4.
            },
            multiview: None, // 5.
            cache: None,
        });
        // 第六章-更新相机，传入speed：移动速度
        let camera_controller = CameraController::new(0.1);

        // 帧率统计
        let now = Instant::now();

        // 第七章-新增! 实例化渲染
        // NUM_INSTANCES_PER_ROW 实例化渲染的实例数量
        const NUM_INSTANCES_PER_ROW: u32 = 10;
        const INSTANCE_DISPLACEMENT: glam::Vec3 = glam::Vec3::new(
            NUM_INSTANCES_PER_ROW as f32 * 0.5,
            0.0,
            NUM_INSTANCES_PER_ROW as f32 * 0.5,
        );
        // 第九章-new
        const SPACE_BETWEEN: f32 = 3.0;
        let instances = (0..NUM_INSTANCES_PER_ROW)
            .flat_map(|z| {
                (0..NUM_INSTANCES_PER_ROW).map(move |x| {
                    let x = SPACE_BETWEEN * (x as f32 - NUM_INSTANCES_PER_ROW as f32 / 2.0);
                    let z = SPACE_BETWEEN * (z as f32 - NUM_INSTANCES_PER_ROW as f32 / 2.0);

                    let position = glam::Vec3 { x, y: 0.0, z };

                    let rotation = if position.length().abs() <= f32::EPSILON {
                        glam::Quat::from_axis_angle(glam::Vec3::Z, 0.0)
                    } else {
                        glam::Quat::from_axis_angle(position.normalize(), consts::FRAC_PI_4)
                    };

                    Instance { position, rotation }
                })
            })
            .collect::<Vec<_>>();
        let instance_data = instances.iter().map(Instance::to_raw).collect::<Vec<_>>();
        let instance_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Instance Buffer"),
            contents: bytemuck::cast_slice(&instance_data),
            usage: wgpu::BufferUsages::VERTEX,
        });

        // 第八章-深度缓冲区
        let depth_texture =
            texture::Texture::create_depth_texture(&device, &config, "depth_texture");

        let obj_model =
            resources::load_model("cube.obj", &device, &queue, &texture_bind_group_layout)
                .await
                .unwrap();

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
            // clear_color,
            // 第三章-渲染管线
            render_pipeline,
            // 第四章-缓冲区与索引
            // vertex_buffer,
            // num_vertices,
            // index_buffer,
            // num_indices,
            // 第五章-纹理和绑定组
            diffuse_bind_group,
            diffuse_texture,
            camera,
            camera_uniform,
            camera_buffer,
            camera_bind_group,
            camera_controller,
            // FPS计数器初始化
            frame_count: 0,
            last_fps_update: now,
            last_frame_time: now,
            fps: 0.0,
            instances,
            instance_buffer,
            // 第八章-深度缓冲区
            depth_texture,
            // 第九章-new
            obj_model,
        }
    }

    fn keyboard_input(&mut self, event: &KeyEvent) -> bool {
        self.camera_controller.process_events(event)
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
            self.depth_texture =
                texture::Texture::create_depth_texture(&self.device, &self.config, "depth_texture");
        }
    }
    fn update(&mut self) {
        self.camera_controller.update_camera(&mut self.camera);
        self.camera_uniform.update_view_proj(&self.camera);
        self.queue.write_buffer(
            &self.camera_buffer,
            0,
            bytemuck::cast_slice(&[self.camera_uniform]),
        );
        // 每秒更新一次的FPS计算
        let now = Instant::now();
        self.frame_count += 1;

        // 每秒计算一次FPS
        let elapsed = now.duration_since(self.last_fps_update);
        if elapsed.as_secs() >= 1 {
            self.fps = self.frame_count as f32 / elapsed.as_secs_f32();
            self.frame_count = 0;
            self.last_fps_update = now;
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
                // 第八章-深度缓冲区-new
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &self.depth_texture.view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                ..Default::default()
            });

            render_pass.set_vertex_buffer(1, self.instance_buffer.slice(..));
            render_pass.set_pipeline(&self.render_pipeline);
            render_pass.draw_model_instanced(
                &self.obj_model,
                0..self.instances.len() as u32,
                &self.camera_bind_group,
            );
        }

        self.queue.submit(Some(encoder.finish()));
        output.present();

        // 每秒更新一次窗口标题显示FPS（减少更新频率）
        if self.frame_count == 0 {
            // 只在FPS计算后更新标题
            self.window
                .set_title(&format!("tutorial7 - FPS: {:.0}", self.fps));
        }

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

                // 第六章重点-更新相机，但文章中没有提及
                app.update();

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
