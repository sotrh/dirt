use crate::game::world::{camera::{Camera2d, PerspectiveCamera}, terrain::Terrain};

pub mod camera;
pub mod terrain;

pub struct World {
    pub ui_camera: Camera2d,
    pub player_camera: PerspectiveCamera,
    pub terrain: Terrain,
}

impl World {
    pub(crate) fn new(width: u32, height: u32, terrain_size: u32, tile_size: u32, max_height: f32) -> Self {
        let ui_camera = Camera2d::new(width as f32, height as f32);

        let center = tile_size as f32 * 0.5;
        let player_camera = PerspectiveCamera::new(
            glam::vec3(center, max_height, center),
            std::f32::consts::FRAC_PI_4,
            -0.1,
            width,
            height,
            std::f32::consts::FRAC_PI_4,
            0.1,
            1000.0,
        );
        
        let terrain = Terrain::generate(terrain_size, tile_size, max_height);

        Self {
            ui_camera,
            player_camera,
            terrain,
        }
    }

    pub(crate) fn resize(&mut self, width: u32, height: u32) {
        self.ui_camera.resize(width, height);
        self.player_camera.resize(width, height);
    }


}
