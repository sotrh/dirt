use bytemuck::{Pod, Zeroable};

use crate::{
    app::AppController,
    game::render::{
        bindings::{
            CameraBinder, CameraBinding, SampledTextureArrayBinder, SampledTextureArrayBinding,
            UniformBinder, UniformBinding,
        },
        buffer::BackedBuffer,
        utils::RenderPipelineBuilder,
    },
};

#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct TileInstance {
    pub position: glam::Vec2,
}

impl TileInstance {
    pub const LAYOUT: wgpu::VertexBufferLayout<'static> = wgpu::VertexBufferLayout {
        array_stride: size_of::<Self>() as _,
        step_mode: wgpu::VertexStepMode::Instance,
        attributes: &wgpu::vertex_attr_array![
            0 => Float32x2,
        ],
    };
}

#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct TerrainData {
    tile_size: f32,
    mountain_height: f32,
    dune_height: f32,
    spire_height: f32,
}

pub struct TerrainBuffer {
    indices: BackedBuffer<u32>,
    pub tiles: BackedBuffer<TileInstance>,
    terrain_data: BackedBuffer<TerrainData>,
    binding: UniformBinding<TerrainData>,
    // todo: textures
}

impl TerrainBuffer {
    pub fn new(
        device: &wgpu::Device,
        binder: &UniformBinder<TerrainData>,
        tile_size: u32,
        mountain_height: f32,
        dune_height: f32,
        spire_height: f32,
    ) -> Self {
        let mut index_data = Vec::new();
        for z in 0..tile_size - 1 {
            for x in 0..tile_size - 1 {
                let i = x + z * tile_size;
                index_data.push(i);
                index_data.push(i + 1 + tile_size);
                index_data.push(i + 1);
                index_data.push(i);
                index_data.push(i + tile_size);
                index_data.push(i + 1 + tile_size);
            }
        }
        let indices = BackedBuffer::with_data(&device, index_data, wgpu::BufferUsages::INDEX);
        let tiles = BackedBuffer::with_capacity(&device, 8, wgpu::BufferUsages::VERTEX);
        let terrain_data = BackedBuffer::with_data(
            &device,
            vec![TerrainData {
                tile_size: tile_size as f32,
                mountain_height,
                dune_height,
                spire_height,
            }],
            wgpu::BufferUsages::UNIFORM,
        );

        let binding = binder.bind(device, &terrain_data);

        Self {
            indices,
            tiles,
            terrain_data,
            binding,
        }
    }
}

pub struct TerrainPipeline {
    triplanar_pipeline: wgpu::RenderPipeline,
    debug_pipeline: wgpu::RenderPipeline,
}

impl TerrainPipeline {
    pub async fn new(
        app: &AppController,
        device: &wgpu::Device,
        uniform_binder: &UniformBinder<TerrainData>,
        camera_binder: &CameraBinder,
        texture_binder: &SampledTextureArrayBinder,
        surface_format: wgpu::TextureFormat,
        depth_format: wgpu::TextureFormat,
    ) -> anyhow::Result<Self> {
        let triplanar_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            bind_group_layouts: &[
                uniform_binder.layout(),
                camera_binder.layout(),
                texture_binder.layout(),
            ],
            ..Default::default()
        });

        let debug_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            bind_group_layouts: &[uniform_binder.layout(), camera_binder.layout()],
            ..Default::default()
        });

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("shaders/terrain.wgsl"),
            source: wgpu::ShaderSource::Wgsl(app.load_string("shaders/terrain.wgsl").await?.into()),
        });

        log::debug!("triplanar_pipeline");
        let triplanar_pipeline = RenderPipelineBuilder::new()
            .layout(&triplanar_layout)
            .cull_mode(Some(wgpu::Face::Back))
            .vertex(wgpu::VertexState {
                module: &shader,
                entry_point: Some("displace_terrain"),
                compilation_options: Default::default(),
                buffers: &[TileInstance::LAYOUT],
            })
            .depth(depth_format, wgpu::CompareFunction::Less)
            .fragment(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("triplanar_shaded"),
                compilation_options: Default::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format: surface_format,
                    blend: None,
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            })
            .build(device)?;

        let debug_pipeline = RenderPipelineBuilder::new()
            .layout(&debug_layout)
            .cull_mode(Some(wgpu::Face::Back))
            .vertex(wgpu::VertexState {
                module: &shader,
                entry_point: Some("displace_terrain"),
                compilation_options: Default::default(),
                buffers: &[TileInstance::LAYOUT],
            })
            .depth(depth_format, wgpu::CompareFunction::Less)
            .fragment(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("debug"),
                compilation_options: Default::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format: surface_format,
                    blend: None,
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            })
            .build(device)?;

        Ok(Self {
            triplanar_pipeline,
            debug_pipeline,
        })
    }

    pub fn draw<'a, 'b: 'a>(
        &'a self,
        pass: &'a mut wgpu::RenderPass<'b>,
        camera: &CameraBinding,
        textures: &SampledTextureArrayBinding,
        buffer: &'a TerrainBuffer,
    ) {
        if buffer.tiles.len() == 0 {
            return;
        }

        pass.set_pipeline(&self.triplanar_pipeline);
        pass.set_bind_group(0, buffer.binding.bind_group(), &[]);
        pass.set_bind_group(1, camera.bind_group(), &[]);
        pass.set_bind_group(2, textures.bind_group(), &[]);
        pass.set_index_buffer(buffer.indices.slice(), wgpu::IndexFormat::Uint32);
        pass.set_vertex_buffer(0, buffer.tiles.slice());
        pass.draw_indexed(0..buffer.indices.len(), 0, 0..buffer.tiles.len());
    }

    pub fn debug<'a, 'b: 'a>(
        &'a self,
        pass: &'a mut wgpu::RenderPass<'b>,
        camera: &CameraBinding,
        buffer: &'a TerrainBuffer,
    ) {
        if buffer.tiles.len() == 0 {
            return;
        }

        pass.set_pipeline(&self.debug_pipeline);
        pass.set_bind_group(0, buffer.binding.bind_group(), &[]);
        pass.set_bind_group(1, camera.bind_group(), &[]);
        pass.set_index_buffer(buffer.indices.slice(), wgpu::IndexFormat::Uint32);
        pass.set_vertex_buffer(0, buffer.tiles.slice());
        pass.draw_indexed(0..buffer.indices.len(), 0, 0..buffer.tiles.len());
    }
}
