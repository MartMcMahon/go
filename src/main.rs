use bytemuck::bytes_of;
use camera::Camera;
use camera::CameraUniform;
use cgmath::InnerSpace;
use std::fs::OpenOptions;
use std::sync::Arc;
use timer::Timer;
use vertex::{BasicVertex, EffectVertex, Vertex};
use wgpu::util::DeviceExt;
use wgpu::Surface;
use winit::application::ApplicationHandler;
use winit::event::{KeyEvent, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::keyboard::{Key, NamedKey};
use winit::window::{Window, WindowId};
// use game:Game;

mod camera;
mod controller;
mod cube;
mod texture;
mod timer;
mod vertex;
// mod game;

const BACKGROUND_QUAD: &[BasicVertex] = &[
    BasicVertex {
        position: [-1.0, 1.0, 0.0],
        tex_coords: [0.0, 0.0],
    },
    BasicVertex {
        position: [1.0, 1.0, 0.0],
        tex_coords: [1.0, 0.0],
    },
    BasicVertex {
        position: [1.0, -1.0, 0.0],
        tex_coords: [1.0, 1.0],
    },
    BasicVertex {
        position: [-1.0, -1.0, 0.0],
        tex_coords: [0.0, 1.0],
    },
];
const QUAD_INDICES: &[u16] = &[0, 1, 2, 0, 2, 3];

const STONE_QUAD: &[EffectVertex] = &[
    EffectVertex {
        position: [-1.0, 1.0, 0.0],
        color: [1.0, 0.0, 0.0],
    },
    EffectVertex {
        position: [1.0, 1.0, 0.0],
        color: [0.0, 1.0, 0.0],
    },
    EffectVertex {
        position: [1.0, -1.0, 0.0],
        color: [0.0, 0.0, 1.0],
    },
    EffectVertex {
        position: [-1.0, -1.0, 0.0],
        color: [0.3, 0.3, 0.3],
    },
];

const BOARD_PIXELS: u16 = 2000;
const MARGIN_OFFSET_PIXELS: u16 = 61;
const BOARD_LINE_THICKNESS_PX: u16 = 5;
const BOARD_SQUARE_SIZE_PX: u16 = 99;
const WIDTH: u32 = 600;
const HEIGHT: u32 = 600;

// ///////
// programatically generate pixel vals for stone quads
// for vertices also indicies
fn game_space_to_px(x: u16, y: u16) -> (u32, u32) {
    (
        (x * BOARD_SQUARE_SIZE_PX + BOARD_LINE_THICKNESS_PX * (x + 1) + MARGIN_OFFSET_PIXELS)
            as u32,
        (y * BOARD_SQUARE_SIZE_PX + BOARD_LINE_THICKNESS_PX * (y + 1) + MARGIN_OFFSET_PIXELS)
            as u32,
    )
}

enum PlayerColor {
    Black,
    White,
}
struct StoneInstance {
    game_pos: [usize; 2],
    position: cgmath::Vector3<f32>,
    player_color: PlayerColor,
}
// impl StoneInstance {
//     fn to_raw(&self) ->
// }
// }

struct GameCursor {
    x: f64,
    y: f64,
}

#[derive(Default)]
struct App {
    window: Option<Arc<Window>>,
    instance: Option<wgpu::Instance>,
    surface: Option<Surface<'static>>,
    device: Option<wgpu::Device>,
    queue: Option<wgpu::Queue>,

    timer: Option<Timer>,
    cursor: Option<GameCursor>,

    // camera
    camera: Option<Camera>,
    camera_buffer: Option<wgpu::Buffer>,
    camera_bind_group: Option<wgpu::BindGroup>,

    // main texture
    main_texture_render_pipeline: Option<wgpu::RenderPipeline>,
    main_texture_bind_group: Option<wgpu::BindGroup>,

    board_vertex_buffer: Option<wgpu::Buffer>,
    board_index_buffer: Option<wgpu::Buffer>,

    render_pipelines: Vec<wgpu::RenderPipeline>,

    stone_vertex_buffer: Option<wgpu::Buffer>,
    stone_index_buffer: Option<wgpu::Buffer>,

    stone_render_pipeline: Option<wgpu::RenderPipeline>,
    stone_bind_group: Option<wgpu::BindGroup>,

    stone_instance_buffer: Option<wgpu::Buffer>,

    // game: Game,

    // player
    // cube_position: Option<cgmath::Vector3<f32>>,

    // controller
    controller: controller::Controller,
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        ///// window
        self.window = Some(Arc::new(
            event_loop
                .create_window(
                    Window::default_attributes()
                        // .with_decorations(false)
                        .with_inner_size(winit::dpi::LogicalSize::new(WIDTH, HEIGHT))
                        // .with_position(winit::dpi::LogicalPosition::new(x, y))
                        .with_transparent(true), // .with_window_level(WindowLevel::AlwaysOnTop),
                )
                .unwrap(),
        ));

        self.instance = Some(wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::PRIMARY,
            flags: wgpu::InstanceFlags::empty(),
            ..Default::default()
        }));
        self.surface = Some(
            self.instance
                .as_ref()
                .unwrap()
                .create_surface(self.window.clone().unwrap())
                .unwrap(),
        );

        let adapter = pollster::block_on(self.instance.as_ref().unwrap().request_adapter(
            &wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::default(),
                compatible_surface: self.surface.as_ref(),
                force_fallback_adapter: false,
            },
        ))
        .unwrap();
        let device_queue = pollster::block_on(adapter.request_device(
            &wgpu::DeviceDescriptor {
                label: Some("device-descriptor"),
                required_features: wgpu::Features::empty(),
                required_limits: wgpu::Limits::default(),
                ..Default::default()
            },
            None,
        ))
        .unwrap();

        self.device = Some(device_queue.0);
        self.queue = Some(device_queue.1);

        let texture_format = wgpu::TextureFormat::Bgra8UnormSrgb;

        self.set_camera(Camera {
            eye: (8.4, 25.0, -8.4).into(),
            target: (0.0, 0.0, 0.0).into(),
            up: (0.0, 1.0, 0.0).into(),
            aspect: WIDTH as f32 / HEIGHT as f32,
            fovy: 90.0,
            znear: 0.1,
            zfar: 100.0,
        });

        let size = self.window.as_ref().unwrap().inner_size();
        self.surface.as_ref().unwrap().configure(
            &self.device.as_ref().unwrap(),
            &wgpu::SurfaceConfiguration {
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
                // not really sure what the TextureFormat is
                format: texture_format,
                width: size.width,
                height: size.height,
                present_mode: wgpu::PresentMode::Fifo,
                desired_maximum_frame_latency: 1,
                alpha_mode: wgpu::CompositeAlphaMode::PostMultiplied,
                // alpha_mode: wgpu::CompositeAlphaMode::Opaque,
                view_formats: vec![wgpu::TextureFormat::Bgra8UnormSrgb],
            },
        );

        ////// controller
        self.controller.velocity = 0.5; // = controller::Controller::new(0.5);

        // /////////
        // stones
        self.stone_vertex_buffer = Some(self.device.as_ref().unwrap().create_buffer_init(
            &wgpu::util::BufferInitDescriptor {
                label: Some("stone vertex buffer"),
                contents: bytemuck::cast_slice(STONE_QUAD),
                usage: wgpu::BufferUsages::UNIFORM
                    | wgpu::BufferUsages::VERTEX
                    | wgpu::BufferUsages::COPY_DST,
            },
        ));
        self.stone_index_buffer = Some(self.device.as_ref().unwrap().create_buffer_init(
            &wgpu::util::BufferInitDescriptor {
                label: Some("stone index buffer"),
                contents: bytemuck::cast_slice(QUAD_INDICES),
                usage: wgpu::BufferUsages::INDEX,
            },
        ));
        let stone_bind_group_layout = &self.device.as_ref().unwrap().create_bind_group_layout(
            &wgpu::BindGroupLayoutDescriptor {
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }],
                label: Some("stone bind group layout"),
            },
        );
        self.stone_bind_group = Some(
            self.device
                .as_ref()
                .unwrap()
                .create_bind_group(&wgpu::BindGroupDescriptor {
                    label: Some("stone_bind_group"),
                    layout: &stone_bind_group_layout,
                    entries: &[wgpu::BindGroupEntry {
                        binding: 0,
                        resource: self
                            .stone_vertex_buffer
                            .as_ref()
                            .unwrap()
                            .as_entire_binding(),
                    }],
                }),
        );

        //// timer buffer
        self.timer = Some(Timer::new(self.device.as_ref().unwrap()));

        // cursor
        self.cursor = Some(GameCursor { x: 0.0, y: 0.0 });

        ///// shader time
        let basic_shader =
            self.device
                .as_ref()
                .unwrap()
                .create_shader_module(wgpu::ShaderModuleDescriptor {
                    label: Some("Shader"),
                    source: wgpu::ShaderSource::Wgsl(include_str!("basic.wgsl").into()),
                });

        let stone_shader =
            self.device
                .as_ref()
                .unwrap()
                .create_shader_module(wgpu::ShaderModuleDescriptor {
                    label: Some("stone shader"),
                    source: wgpu::ShaderSource::Wgsl(include_str!("stone.Wgsl").into()),
                });
        let background_texture_bind_group_layout =
            &self.device.as_ref().unwrap().create_bind_group_layout(
                &wgpu::BindGroupLayoutDescriptor {
                    entries: &[
                        wgpu::BindGroupLayoutEntry {
                            binding: 0,
                            visibility: wgpu::ShaderStages::FRAGMENT,
                            ty: wgpu::BindingType::Texture {
                                sample_type: wgpu::TextureSampleType::Float { filterable: true },
                                view_dimension: wgpu::TextureViewDimension::D2,
                                multisampled: false,
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
                    label: Some("background texture bind group layout"),
                },
            );
        let main_texture_pipeline_layout =
            self.device
                .as_ref()
                .unwrap()
                .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label: Some("cube pipeline layout"),
                    bind_group_layouts: &[background_texture_bind_group_layout],
                    push_constant_ranges: &[],
                });
        self.main_texture_render_pipeline =
            Some(self.device.as_ref().unwrap().create_render_pipeline(
                &wgpu::RenderPipelineDescriptor {
                    label: Some("background render pipeline"),
                    layout: Some(&main_texture_pipeline_layout),
                    vertex: wgpu::VertexState {
                        module: &basic_shader,
                        entry_point: Some("vs_main"),
                        compilation_options: wgpu::PipelineCompilationOptions::default(),
                        buffers: &[BasicVertex::desc()],
                    },
                    fragment: Some(wgpu::FragmentState {
                        module: &basic_shader,
                        entry_point: Some("fs_main"),
                        targets: &[Some(wgpu::ColorTargetState {
                            format: texture_format,
                            blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                            write_mask: wgpu::ColorWrites::ALL,
                        })],
                        compilation_options: wgpu::PipelineCompilationOptions::default(),
                    }),
                    primitive: wgpu::PrimitiveState {
                        topology: wgpu::PrimitiveTopology::TriangleList,
                        strip_index_format: None,
                        front_face: wgpu::FrontFace::Cw,
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
                    multiview: None,
                    cache: None,
                },
            ));
        self.board_vertex_buffer = Some(self.device.as_ref().unwrap().create_buffer_init(
            &wgpu::util::BufferInitDescriptor {
                label: Some("background vertex buffer"),
                contents: bytemuck::cast_slice(BACKGROUND_QUAD.to_vec().as_slice()),
                usage: wgpu::BufferUsages::VERTEX,
            },
        ));
        self.board_index_buffer = Some(self.device.as_ref().unwrap().create_buffer_init(
            &wgpu::util::BufferInitDescriptor {
                label: Some("background index buffer"),
                contents: bytemuck::cast_slice(QUAD_INDICES),
                usage: wgpu::BufferUsages::INDEX,
            },
        ));

        let stone_render_pipeline_layout = &self.device.as_ref().unwrap().create_pipeline_layout(
            &wgpu::PipelineLayoutDescriptor {
                label: Some("stone pipeline layout"),
                bind_group_layouts: &[stone_bind_group_layout],
                push_constant_ranges: &[],
            },
        );
        self.stone_render_pipeline = Some(self.device.as_ref().unwrap().create_render_pipeline(
            &wgpu::RenderPipelineDescriptor {
                label: Some("stone render pipeline"),
                layout: Some(&stone_render_pipeline_layout),
                vertex: wgpu::VertexState {
                    module: &stone_shader,
                    entry_point: Some("vs_main"),
                    compilation_options: wgpu::PipelineCompilationOptions::default(),
                    buffers: &[EffectVertex::desc()],
                },
                fragment: Some(wgpu::FragmentState {
                    module: &stone_shader,
                    entry_point: Some("fs_main"),
                    compilation_options: wgpu::PipelineCompilationOptions::default(),
                    targets: &[Some(wgpu::ColorTargetState {
                        format: texture_format,
                        blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                }),
                primitive: wgpu::PrimitiveState {
                    topology: wgpu::PrimitiveTopology::TriangleList,
                    strip_index_format: None,
                    front_face: wgpu::FrontFace::Cw,
                    cull_mode: Some(wgpu::Face::Back),
                    unclipped_depth: false,
                    polygon_mode: wgpu::PolygonMode::Fill,
                    conservative: false,
                },
                depth_stencil: None,
                multisample: wgpu::MultisampleState {
                    count: 1,
                    mask: !0,
                    alpha_to_coverage_enabled: false,
                },
                multiview: None,
                cache: None,
            },
        ));

        // self.white_pawn_vertex_buffer = Some(self.device.as_ref().unwrap().create_buffer_init(
        //     &wgpu::util::BufferInitDescriptor {
        //         label: Some("white pawn"),
        //         contents: bytemuck::cast_slice(WHITE_PAWN_QUAD.to_vec().as_slice()),
        //         usage: wgpu::BufferUsages::VERTEX,
        //     },
        // ));
        // self.white_pawn_index_buffer = Some(self.device.as_ref().unwrap().create_buffer_init(
        //     &wgpu::util::BufferInitDescriptor {
        //         label: Some("white pawn index buffer"),
        //         contents: bytemuck::cast_slice(QUAD_INDICES),
        //         usage: wgpu::BufferUsages::INDEX,
        //     },
        // ));

        let main_texture_diffuse_bytes = include_bytes!("../res/board.png");
        let main_texture = texture::Texture::from_bytes(
            &self.device.as_ref().unwrap(),
            &self.queue.as_ref().unwrap(),
            main_texture_diffuse_bytes,
            "background image",
            false,
        )
        .unwrap();
        self.main_texture_bind_group = Some(self.device.as_ref().unwrap().create_bind_group(
            &wgpu::BindGroupDescriptor {
                layout: &background_texture_bind_group_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::TextureView(&main_texture.view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::Sampler(&main_texture.sampler),
                    },
                ],
                label: Some("backgroundd texture bind group"),
            },
        ));

        // initial redraw request
        self.window.as_ref().unwrap().request_redraw();

        // let stone_instance_data = self.stone_instances.iter().map(Instance::to_raw)
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, id: WindowId, event: WindowEvent) {
        if self.controller.process_events(&event) {
            return;
        }
        match event {
            WindowEvent::CloseRequested
            | WindowEvent::KeyboardInput {
                event:
                    KeyEvent {
                        state: winit::event::ElementState::Pressed,
                        logical_key: Key::Named(NamedKey::Escape),
                        ..
                    },
                ..
            } => {
                println!("The close button was pressed; stopping");
                event_loop.exit();
            }
            WindowEvent::KeyboardInput {
                event:
                    KeyEvent {
                        state: winit::event::ElementState::Pressed,
                        logical_key: Key::Named(NamedKey::Space),
                        ..
                    },
                ..
            } => {} //self.add_cube(),
            WindowEvent::CursorMoved {
                device_id,
                position,
            } => {
                println!("{} {}", position.x, position.y);
                let mut c = self.cursor.as_mut().unwrap();
                c.x = position.x;
                c.y = position.y;
            }

            WindowEvent::RedrawRequested => {
                self.update();
                let output = self
                    .surface
                    .as_ref()
                    .unwrap()
                    .get_current_texture()
                    .unwrap();

                let view = output
                    .texture
                    .create_view(&wgpu::TextureViewDescriptor::default());
                let mut encoder = self.device.as_ref().unwrap().create_command_encoder(
                    &wgpu::CommandEncoderDescriptor {
                        label: Some("render encoder"),
                    },
                );

                {
                    let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                        label: Some("render pass"),
                        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                            view: &view,
                            resolve_target: None,
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
                        depth_stencil_attachment: None,
                        timestamp_writes: None,
                        occlusion_query_set: None,
                    });

                    //////
                    // draw board
                    render_pass.set_pipeline(&self.main_texture_render_pipeline.as_ref().unwrap());
                    render_pass.set_bind_group(0, &self.main_texture_bind_group, &[]);
                    render_pass.set_bind_group(1, self.camera_bind_group.as_ref().unwrap(), &[]);
                    render_pass
                        .set_vertex_buffer(0, self.board_vertex_buffer.as_ref().unwrap().slice(..));
                    render_pass.set_index_buffer(
                        self.board_index_buffer.as_ref().unwrap().slice(..),
                        wgpu::IndexFormat::Uint16,
                    );
                    render_pass.draw_indexed(0..QUAD_INDICES.len() as u32, 0, 0..1);

                    // // draw stones
                    render_pass.set_pipeline(&self.stone_render_pipeline.as_ref().unwrap());
                    render_pass.set_bind_group(0, &self.stone_bind_group, &[]);
                    render_pass.set_bind_group(
                        0,
                        &self.timer.as_ref().unwrap().timer_bind_group,
                        &[],
                    );
                    render_pass
                        .set_vertex_buffer(0, self.stone_vertex_buffer.as_ref().unwrap().slice(..));
                    render_pass.set_index_buffer(
                        self.stone_index_buffer.as_ref().unwrap().slice(..),
                        wgpu::IndexFormat::Uint16,
                    );
                    render_pass.draw_indexed(0..QUAD_INDICES.len() as u32, 0, 0..1);

                    //// draw pawn
                    // render_pass.set_bind_group(0, &self.main_texture_bind_group, offsets);
                    // render_pass.set
                    // render_pass.set_vertex_buffer( 0, self.white_pawn_vertex_buffer.as_ref().unwrap().slice(..),);
                    // render_pass.draw_indexed(0..QUAD_INDICES.len() as u32, 0, 0..1);
                }

                // submit will accept anything that implements IntoIter
                self.queue
                    .as_ref()
                    .unwrap()
                    .submit(std::iter::once(encoder.finish()));
                output.present();
                self.window.as_ref().unwrap().request_redraw();
            }
            _ => (),
        }
    }
}
impl App {
    fn update(&mut self) {
        // Update the cube's position
        let mut x = 0.0;
        let mut y = 0.0;
        let mut z = 0.0;
        if self.controller.is_up_pressed {
            z += 1.0;
        }
        if self.controller.is_down_pressed {
            z -= 1.0;
        }
        if self.controller.is_left_pressed {
            x -= 1.0;
        }
        if self.controller.is_right_pressed {
            x += 1.0;
        }
        let mut move_vector = cgmath::Vector3::new(x, y, z);
        if move_vector.magnitude() != 0.0 {
            move_vector = move_vector.normalize();
        }
        move_vector *= self.controller.velocity;

        match self.timer.as_mut() {
            Some(timer) => {
                let target_fps = 1.0 / 60.0 as f64;
                timer.elapsed = timer.start.elapsed().as_secs_f64();
                timer.acc += timer.elapsed - timer.last;
                timer.last = timer.elapsed;
                // framerate stuff goes here?
                timer.timer_uniform.t = timer.elapsed as f32;
                self.queue.as_ref().unwrap().write_buffer(
                    &timer.timer_buffer,
                    0,
                    &timer.timer_uniform.t.to_le_bytes(),
                );
            }
            None => {}
        };

        ////////

        let c = self.cursor.as_ref().unwrap();
        self.queue.as_ref().unwrap().write_buffer(
            self.stone_vertex_buffer.as_mut().unwrap(),
            0,
            bytemuck::cast_slice(&[c.x, c.y].as_slice()),
        );
    }

    fn set_camera(&mut self, camera: Camera) {
        let mut camera_uniform = CameraUniform::new();
        camera_uniform.update_view_proj(&camera);

        self.camera_buffer = Some(self.device.as_ref().unwrap().create_buffer_init(
            &wgpu::util::BufferInitDescriptor {
                label: Some("Camera Buffer"),
                contents: bytemuck::cast_slice(&[camera_uniform]),
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            },
        ));

        let camera_bind_group_layout = self.device.as_ref().unwrap().create_bind_group_layout(
            &wgpu::BindGroupLayoutDescriptor {
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }],
                label: Some("camera_bind_group_layout"),
            },
        );

        self.camera_bind_group = Some(self.device.as_ref().unwrap().create_bind_group(
            &wgpu::BindGroupDescriptor {
                layout: &camera_bind_group_layout,
                entries: &[wgpu::BindGroupEntry {
                    binding: 0,
                    resource: self.camera_buffer.as_ref().unwrap().as_entire_binding(),
                }],
                label: Some("camera_bind_group"),
            },
        ));

        self.camera = Some(camera);
    }

    // fn draw_board(&self) {
    //     render_pass.set_pipeline(&self.sprite_render_pipeline)
    // }
}

fn main() {
    let event_loop = EventLoop::new().unwrap();
    event_loop.set_control_flow(ControlFlow::Wait);
    let mut app = App::default();
    let _ = event_loop.run_app(&mut app);
}
