pub mod bindings;
pub mod buffer;
pub mod data;
pub mod font;
pub mod pipeline;
pub mod utils;

use std::sync::Arc;

use anyhow::Context;
use winit::window::Window;

use crate::{
    app::AppController,
    game::{
        render::{
            bindings::{CameraBinder, SampledTextureBinder},
            buffer::BackedBuffer,
            data::CameraData,
            font::{Font, TextPipeline},
        },
        world::camera::Camera,
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
    debug_text: font::TextBuffer,
    ui_camera_buffer: BackedBuffer<CameraData>,
    ui_camera_binding: bindings::CameraBinding,
}

impl Renderer {
    pub async fn new(app: &AppController, window: Arc<Window>) -> anyhow::Result<Self> {
        let width = window.inner_size().width.max(1);
        let height = window.inner_size().height.max(1);

        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
            ..Default::default()
        });

        let surface = instance.create_surface(window)?;

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

        let debug_text = text_pipeline.buffer_text(&font, &device, "Hello, world!")?;

        let ui_camera_buffer = BackedBuffer::with_data(
            &device,
            vec![CameraData::IDENTITY],
            wgpu::BufferUsages::UNIFORM,
        );
        let ui_camera_binding = camera_binder.bind(&device, &ui_camera_buffer);

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
            debug_text,
            ui_camera_buffer,
            ui_camera_binding,
        })
    }

    pub(crate) fn resize(&mut self, width: u32, height: u32) {
        self.is_surface_configured = true;
        self.config.width = width.max(1);
        self.config.height = height.max(1);
        self.surface.configure(&self.device, &self.config);
    }

    pub(crate) fn render(&mut self, app: &AppController, ui_camera: &impl Camera) {
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

        let view = frame.texture.create_view(&Default::default());

        let mut encoder = self.device.create_command_encoder(&Default::default());

        {
            let mut ui_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: None,
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });

            self.text_pipeline
                .draw_text(&mut ui_pass, &self.debug_text, &self.ui_camera_binding);
        }

        self.queue.submit([encoder.finish()]);
        frame.present();
    }
}
