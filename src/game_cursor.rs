struct GameCursor {
    pub fn new(device: &wgpu::Device) -> GameCursor {
        let game_pos_uniform = GamePosUniform {x:0.0, y: 0.0}
        let game_pos_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor{
            label: Some("buffer for the board position of the mouse"),
            contents: &[game_pos_uniform.x, game_pos_uniform.y].to_le_bytes(),

        })
    }
}
