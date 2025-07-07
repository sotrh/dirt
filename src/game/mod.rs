use std::sync::Arc;

use winit::{dpi::PhysicalPosition, event::MouseButton, keyboard::KeyCode, window::{Fullscreen, Window}};

use crate::{
    app::AppController,
    game::{
        render::Renderer,
        world::{World, camera::CameraController},
    },
};

mod render;
mod world;

pub struct Game {
    renderer: Renderer,
    world: World,
    pub(crate) window: Arc<Window>,
    terrain_id: usize,
    camera_controller: CameraController,
    game_play_timer: web_time::Instant,
    lmb_pressed: bool,
}

impl Game {
    pub async fn new(app: &AppController, window: Arc<Window>) -> anyhow::Result<Self> {
        log::debug!("Creating Renderer");
        let mut renderer = Renderer::new(app, window.clone()).await?;

        let width = window.inner_size().width.max(1);
        let height = window.inner_size().height.max(1);

        let world = World::new(width, height, 16, 256, 50.0);

        let terrain_id = renderer.buffer_terrain(&world.terrain);

        let camera_controller = CameraController::new(5.0, 1.0);

        Ok(Self {
            renderer,
            window,
            world,
            terrain_id,
            camera_controller,
            game_play_timer: web_time::Instant::now(),
            lmb_pressed: false,
        })
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        self.renderer.resize(width, height);
        self.world.resize(width, height);
    }

    pub fn render(&mut self, app: &AppController) {
        self.window.request_redraw();

        let dt = self.game_play_timer.elapsed();
        self.game_play_timer = web_time::Instant::now();

        self.camera_controller
            .update_camera(&mut self.world.player_camera, dt);

        self.renderer
            .update_terrain(self.terrain_id, &self.world.terrain);

        self.renderer
            .render(app, &self.world.ui_camera, &self.world.player_camera);
    }

    pub(crate) fn handle_close_requested(&mut self, app: &AppController) {
        app.exit();
    }

    pub(crate) fn handle_mouse_motion(&mut self, dx: f32, dy: f32) {
        if self.lmb_pressed {
            self.camera_controller.process_mouse(dx, dy);
            let size = self.window.inner_size();
            self.window.set_cursor_position(PhysicalPosition::new(size.width / 2, size.height / 2)).unwrap();
        }
    }

    pub(crate) fn handle_key(&mut self, app: &AppController, key: KeyCode, is_pressed: bool) {
        if self.camera_controller.process_keyboard(key, is_pressed) {
            return;
        }

        match (key, is_pressed) {
            (KeyCode::Escape, _) => app.exit(),
            (KeyCode::KeyF, true) => self.toggle_fullscreen(),
            _ => {}
        }
    }

    pub(crate) fn handle_axis(&self, axis: gilrs::Axis, _amount: f32) {
        match axis {
            _ => {}
        }
    }

    pub(crate) fn handle_mouse_button(
        &mut self,
        button: MouseButton,
        is_pressed: bool,
    ) {
        match button {
            MouseButton::Left => {
                self.lmb_pressed = is_pressed;
                self.window.set_cursor_visible(!is_pressed);
            },
            _ => {}
        }
    }
    
    fn toggle_fullscreen(&self) {
        match self.window.fullscreen() {
            Some(_) => self.window.set_fullscreen(None),
            None => self.window.set_fullscreen(Some(Fullscreen::Borderless(None))),
        }
    }
}
