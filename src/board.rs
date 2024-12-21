pub struct Board {
    pipeline: wgpu::Pipeline,
    vertex_buffer: wgpu::Buffer,
}
impl Board {
    pub fn new(device: &wgpu::Device, layout: &wgpu::PipelineLayout) -> Self {
        &wgpu::RenderPassDescriptor {
            label: Some("board pipeline"),
            layout: Some(&layout),
        }
    }
}
