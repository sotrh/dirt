use crate::game::world::camera::{Camera2d, PerspectiveCamera};

pub mod camera;
mod terrain;

pub struct World {
    ui_camera: Camera2d,
    player_camera: PerspectiveCamera,
}

impl World {
    pub(crate) fn new() -> Self {
        let ui_camera = Camera2d::new(1.0, 1.0);
        let player_camera = PerspectiveCamera::new(
            glam::vec3(0.0, 0.0, 3.0),
            -std::f32::consts::FRAC_PI_2,
            0.0,
            1,
            1,
            std::f32::consts::FRAC_PI_4,
            0.1,
            100.0,
        );
        Self {
            ui_camera,
            player_camera,
        }
    }

    pub(crate) fn resize(&mut self, width: u32, height: u32) {
        self.ui_camera.resize(width, height);
        self.player_camera.resize(width, height);
    }
}
