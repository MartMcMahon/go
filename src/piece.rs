use cgmath::Vector2;

pub struct Piece {
    pub x: usize,
    pub y: usize,
}

impl Piece {
    pub fn draw(render_pass: &mut wgpu::RenderPass) {}
    pub fn warp(&self, x: usize, y: usize) {
        self.x = x;
        self.y = y;
    }
}

// pub trait DrawPiece<'a> {
//     fn draw(&mut self) {
//         self.set_vertex_buffer(0, vertex_buffer.slice(..));
//         self.set_bind_group(0, &piece_material_bind_group, &[]);
//         self.draw_indexed(0..)
//     }
// }
