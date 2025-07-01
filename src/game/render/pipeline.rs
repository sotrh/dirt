use std::path::PathBuf;

pub struct PipelineDesc {
    pub label: Option<String>,
    pub binders: Vec<Binder>,
    pub vertex: VertexDesc,
}

pub struct Binder {
    group: u32,
    layout: BinderLayouts,
}

pub enum BinderLayouts {
    Camera,
    SampledTexture,
}

pub struct VertexDesc {
    shader: PathBuf,
    buffer_layouts: Vec<VertexLayouts>,
}

pub enum VertexLayouts {
    UiVertex,
    ModelVertex,
}
