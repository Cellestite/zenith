use std::hash::{Hash, Hasher};

#[derive(Debug, Clone, Hash)]
pub struct VertexBufferLayout {
    // TODO: use SmallVec
    attributes: Vec<wgpu::VertexAttribute>,
    location_allocator: wgpu::ShaderLocation,
    current_offset: wgpu::BufferAddress,
}

impl VertexBufferLayout {
    pub fn new() -> Self {
        Self {
            attributes: Vec::new(),
            location_allocator: 0,
            current_offset: 0,
        }
    }

    pub fn with_attributes_count(count: usize) -> Self {
        Self {
            attributes: Vec::with_capacity(count),
            location_allocator: 0,
            current_offset: 0,
        }
    }

    pub fn push_attribute(mut self, format: wgpu::VertexFormat) -> Self {
        self.attributes.push(wgpu::VertexAttribute {
            format,
            offset: self.current_offset,
            shader_location: self.location_allocator,
        });

        self.current_offset += format.size();
        self.location_allocator += 1;

        self
    }

    pub fn build_as(&self) -> wgpu::VertexBufferLayout {
        wgpu::VertexBufferLayout {
            array_stride: self.current_offset,
            step_mode: Default::default(),
            attributes: &self.attributes,
        }
    }
}

impl PartialEq for VertexBufferLayout {
    fn eq(&self, other: &Self) -> bool {
        self.attributes == other.attributes
    }
}

#[derive(Debug)]
pub struct GraphicShader {
    pub name: String,
    pub source: String,
    pub vertex_entry: String,
    pub fragment_entry: Option<String>,
    pub vertex_layout: VertexBufferLayout,
}

impl GraphicShader {
    pub fn new(
        name: &str,
        source: String,
        vertex_entry: &str,
        fragment_entry: Option<&str>,
        vertex_layout: VertexBufferLayout,
    ) -> Self {
        Self {
            name: name.to_owned(),
            source,
            vertex_entry: vertex_entry.to_string(),
            fragment_entry: fragment_entry.map(ToOwned::to_owned),
            vertex_layout,
        }
    }
}

impl Hash for GraphicShader {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.source.hash(state);
        self.vertex_entry.hash(state);
        self.fragment_entry.hash(state);
        self.vertex_layout.hash(state);
    }
}