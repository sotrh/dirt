pub mod bindings;
pub mod buffer;
pub mod data;
pub mod font;
pub mod pipeline;
pub mod terrain;
pub mod utils;

use std::sync::Arc;

use anyhow::Context;
use wgpu::util::DeviceExt;
use winit::window::Window;

use crate::{
    app::AppController,
    game::{
        render::{
            bindings::{
                CameraBinder, SampledTextureArrayBinder, SampledTextureBinder, UniformBinder,
            },
            buffer::BackedBuffer,
            data::CameraData,
            font::{Font, TextPipeline},
            terrain::{TerrainBuffer, TerrainPipeline, TileInstance},
        },
        world::{camera::Camera, terrain::Terrain},
    },
};

pub struct Renderer {
    surface: wgpu::Surface<'static>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    is_surface_configured: bool,
    config: wgpu::wgt::SurfaceConfiguration<Vec<wgpu::TextureFormat>>,
    camera_binder: CameraBinder,
    sampled_texture_binder: SampledTextureBinder,
    font: Font,
    text_pipeline: TextPipeline,
    text_buffers: Vec<font::TextBuffer>,
    ui_camera_buffer: BackedBuffer<CameraData>,
    ui_camera_binding: bindings::CameraBinding,
    terrain_binder: UniformBinder<terrain::TerrainData>,
    terrain_pipeline: TerrainPipeline,
    terrain_buffers: Vec<TerrainBuffer>,
    depth_buffer: wgpu::Texture,
    depth_buffer_view: wgpu::TextureView,
    main_camera_buffer: BackedBuffer<CameraData>,
    main_camera_binding: bindings::CameraBinding,
    terrain_texture_binding: bindings::SampledTextureArrayBinding,
}

impl Renderer {
    pub async fn new(app: &AppController, window: Arc<Window>) -> anyhow::Result<Self> {
        let width = window.inner_size().width.max(1);
        let height = window.inner_size().height.max(1);

        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
            ..Default::default()
        });

        #[cfg(not(target_os = "windows"))]
        let surface = instance.create_surface(window)?;

        // Safety: [Window] is technically [Send], but on Windows some operations are only
        // permitted on the thread that created the [Window], so this is a work around.
        #[cfg(target_os = "windows")]
        let surface = unsafe {
            use wgpu::rwh::HasDisplayHandle;
            use winit::platform::windows::WindowExtWindows;
            instance.create_surface_unsafe(wgpu::SurfaceTargetUnsafe::RawHandle {
                raw_display_handle: window.display_handle()?.as_raw(),
                raw_window_handle: window.window_handle_any_thread()?.as_raw(),
            })
        }?;
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                compatible_surface: Some(&surface),
                ..Default::default()
            })
            .await?;

        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                ..Default::default()
            })
            .await?;

        let mut config = surface
            .get_default_config(&adapter, width, height)
            .with_context(|| "Surface is invalid")?;
        config.view_formats.push(config.format.add_srgb_suffix());

        #[cfg(not(target_arch = "wasm32"))]
        surface.configure(&device, &config);

        let camera_binder = CameraBinder::new(&device);
        let sampled_texture_binder = SampledTextureBinder::new(&device);

        let font = Font::load(app, "fonts/OpenSans MSDF.zip", '�', &device, &queue).await?;
        let text_pipeline = TextPipeline::new(
            app,
            &device,
            &font,
            config.format,
            &camera_binder,
            &sampled_texture_binder,
        )
        .await?;

        let ui_camera_buffer = BackedBuffer::with_data(
            &device,
            vec![CameraData::IDENTITY],
            wgpu::BufferUsages::UNIFORM,
        );
        let ui_camera_binding = camera_binder.bind(&device, &ui_camera_buffer);

        let main_camera_buffer = BackedBuffer::with_data(
            &device,
            vec![CameraData::IDENTITY],
            wgpu::BufferUsages::UNIFORM,
        );
        let main_camera_binding = camera_binder.bind(&device, &main_camera_buffer);

        let texture_array_binder = SampledTextureArrayBinder::new(&device);

        let depth_format = wgpu::TextureFormat::Depth32Float;
        let depth_buffer = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("depth_buffer"),
            size: wgpu::Extent3d {
                width: config.width,
                height: config.height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: depth_format,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        });
        let depth_buffer_view = depth_buffer.create_view(&Default::default());

        let terrain_binder = UniformBinder::new(&device, wgpu::ShaderStages::VERTEX_FRAGMENT);
        let terrain_pipeline = TerrainPipeline::new(
            app,
            &device,
            &terrain_binder,
            &camera_binder,
            &texture_array_binder,
            config.format,
            depth_format,
        )
        .await?;

        let terrain_texture_array = device.create_texture_with_data(
            &queue,
            &wgpu::TextureDescriptor {
                label: Some("terrain_texture_array"),
                size: wgpu::Extent3d {
                    width: 1,
                    height: 1,
                    depth_or_array_layers: 4,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::Rgba8Unorm,
                usage: wgpu::TextureUsages::TEXTURE_BINDING,
                view_formats: &[],
            },
            wgpu::wgt::TextureDataOrder::LayerMajor,
            &[
                0x28, 0xaa, 0x00, 0xff, 127, 127, 255, 255, 0x62, 0x3b, 15, 0xff, 127, 127, 255,
                255,
            ],
        );
        let terrain_texture_array_view = terrain_texture_array.create_view(&Default::default());
        let terrain_texture_sampler = device.create_sampler(&Default::default());
        let terrain_texture_binding = texture_array_binder.bind(
            &device,
            &terrain_texture_array_view,
            &terrain_texture_sampler,
        );

        Ok(Self {
            surface,
            device,
            queue,
            config,
            is_surface_configured: cfg!(not(target_arch = "wasm32")),
            camera_binder,
            sampled_texture_binder,
            font,
            text_pipeline,
            text_buffers: Vec::new(),
            ui_camera_buffer,
            ui_camera_binding,
            main_camera_buffer,
            main_camera_binding,
            depth_buffer,
            depth_buffer_view,
            terrain_binder,
            terrain_pipeline,
            terrain_buffers: Vec::new(),
            terrain_texture_binding,
        })
    }

    pub(crate) fn resize(&mut self, width: u32, height: u32) {
        self.is_surface_configured = true;
        self.config.width = width.max(1);
        self.config.height = height.max(1);
        self.surface.configure(&self.device, &self.config);
        let depth_format = self.depth_buffer.format();
        self.depth_buffer = self.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("depth_buffer"),
            size: wgpu::Extent3d {
                width: self.config.width,
                height: self.config.height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: depth_format,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        });
        self.depth_buffer_view = self.depth_buffer.create_view(&Default::default());
    }

    pub(crate) fn render(
        &mut self,
        app: &AppController,
        ui_camera: &impl Camera,
        player_camera: &impl Camera,
        debug_mode_active: bool,
    ) {
        if !self.is_surface_configured {
            self.surface.configure(&self.device, &self.config);
            self.is_surface_configured = true;
        }

        let frame = match self.surface.get_current_texture() {
            Ok(frame) => frame,
            Err(e) => match e {
                wgpu::SurfaceError::Outdated => {
                    return;
                }
                e => {
                    log::error!("{e}");
                    app.exit();
                    return;
                }
            },
        };

        self.ui_camera_buffer
            .update(&self.queue, |data| data[0].update(ui_camera));
        self.main_camera_buffer
            .update(&self.queue, |data| data[0].update(player_camera));

        let view = frame.texture.create_view(&Default::default());

        let mut encoder = self.device.create_command_encoder(&Default::default());

        {
            let mut main_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("main_pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &self.depth_buffer_view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                timestamp_writes: None,
                occlusion_query_set: None,
            });

            for buffer in &self.terrain_buffers {
                if debug_mode_active {
                    self.terrain_pipeline
                        .debug(&mut main_pass, &self.main_camera_binding, buffer);
                } else {
                    self.terrain_pipeline.draw(
                        &mut main_pass,
                        &self.main_camera_binding,
                        &self.terrain_texture_binding,
                        buffer,
                    );
                }
            }
        }

        {
            let mut ui_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("ui_pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });

            for text in &self.text_buffers {
                self.text_pipeline
                    .draw_text(&mut ui_pass, text, &self.ui_camera_binding);
            }
        }

        self.queue.submit([encoder.finish()]);
        frame.present();
    }

    pub fn buffer_terrain(&mut self, terrain: &Terrain) -> usize {
        let id = self.terrain_buffers.len();
        let buffer = TerrainBuffer::new(
            &self.device,
            &self.terrain_binder,
            terrain.tile_size,
            terrain.max_height,
        );
        self.terrain_buffers.push(buffer);

        id
    }

    pub fn update_terrain(&mut self, terrain_id: usize, terrain: &Terrain) {
        let buffer = &mut self.terrain_buffers[terrain_id];
        buffer.tiles.clear();
        let mut batch = buffer.tiles.batch(&self.device, &self.queue);
        let range = 0..2;
        for tile in &terrain.tiles {
            if range.contains(&tile.id.0) && range.contains(&tile.id.1) {
                let position = glam::vec2(
                    (tile.id.0 * (terrain.tile_size - 1)) as _,
                    (tile.id.1 * (terrain.tile_size - 1)) as _,
                );
                batch.push(TileInstance { position });
            }
        }
    }

    pub fn buffer_text(&mut self, text: &str) -> usize {
        let id = self.text_buffers.len();
        self.text_buffers.push(
            self.text_pipeline
                .buffer_text(&self.font, &self.device, text)
                .unwrap(),
        );
        id
    }

    pub fn update_text(&mut self, text_id: usize, text: &str) {
        self.text_pipeline.update_text(&self.font, text, &mut self.text_buffers[text_id], &self.device, &self.queue);
    }

    // pub fn update_terrain(&)
}
