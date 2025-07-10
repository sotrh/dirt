use std::sync::Arc;

use serde::{Deserialize, Serialize};
use web_time::{Duration, Instant};
use winit::{
    dpi::PhysicalPosition,
    event::{MouseButton, MouseScrollDelta},
    keyboard::KeyCode,
    window::{Fullscreen, Window},
};

use crate::{
    app::AppController,
    game::{
        render::Renderer,
        world::{World, camera::CameraController},
    },
};

mod render;
mod world;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Settings {
    debug_mode_active: bool,
    fullscreen: bool,
    #[serde(default = "default_move_speed")]
    move_speed: f32,
    #[serde(default = "default_tile_size")]
    tile_size: u32,
    #[serde(default = "default_terrain_height")]
    terrain_height: f32,
    #[serde(default = "default_terrain_size")]
    terrain_size: u32,
    #[serde(default = "default_chunk_radius")]
    chunk_radius: u32,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            debug_mode_active: false,
            fullscreen: false,
            move_speed: default_move_speed(),
            tile_size: default_tile_size(),
            terrain_height: default_terrain_height(),
            terrain_size: default_terrain_size(),
            chunk_radius: default_chunk_radius(),
        }
    }
}

fn default_terrain_height() -> f32 {
    50.0
}

fn default_move_speed() -> f32 {
    20.0
}

fn default_tile_size() -> u32 {
    256
}

fn default_terrain_size() -> u32 {
    16
}

fn default_chunk_radius() -> u32 {
    4
}

pub struct Game {
    renderer: Renderer,
    world: World,
    pub(crate) window: Arc<Window>,
    settings: Settings,
    terrain_id: usize,
    camera_controller: CameraController,
    game_play_timer: Instant,
    frame_timer: Instant,
    lmb_pressed: bool,
    num_frames: i32,
    tick_rate: Duration,
    debug_text: usize,
    render_time: Duration,
}

impl Game {
    pub async fn new(app: &AppController, window: Arc<Window>) -> anyhow::Result<Self> {
        let settings = match app.load_string("settings.json").await {
            Ok(json) => serde_json::from_str(&json)?,
            Err(_) => Settings::default(),
        };

        if settings.fullscreen {
            window.set_fullscreen(Some(Fullscreen::Borderless(None)));
        }

        log::debug!("Creating Renderer");
        let mut renderer = Renderer::new(app, window.clone()).await?;

        let debug_text = renderer.buffer_text(&format!(
            "Debug Mode: {}\nTickRate: ---",
            if settings.debug_mode_active {
                "ON"
            } else {
                "OFF"
            },
        ));

        let width = window.inner_size().width.max(1);
        let height = window.inner_size().height.max(1);

        let world = World::new(
            width,
            height,
            settings.terrain_size,
            settings.tile_size,
            settings.terrain_height,
        );

        let terrain_id = renderer.buffer_terrain(&world.terrain);

        renderer.update_terrain(terrain_id, &world.terrain, settings.chunk_radius);

        let camera_controller = CameraController::new(settings.move_speed, 1.0);

        Ok(Self {
            renderer,
            window,
            world,
            terrain_id,
            camera_controller,
            game_play_timer: Instant::now(),
            frame_timer: Instant::now(),
            num_frames: 0,
            lmb_pressed: false,
            tick_rate: Duration::ZERO,
            settings,
            debug_text,
            render_time: Duration::ZERO,
        })
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        log::info!("resize({width}, {height})");
        self.renderer.resize(width, height);
        self.world.resize(width, height);
    }

    pub fn render(&mut self, app: &AppController) {
        let render_timer = Instant::now();
        self.window.request_redraw();

        let dt = self.game_play_timer.elapsed();
        self.game_play_timer = Instant::now();

        self.num_frames += 1;
        if self.num_frames >= 100 {
            self.tick_rate = self.frame_timer.elapsed() / 100;
            self.num_frames = 0;
            self.frame_timer = Instant::now();
        }

        self.camera_controller
            .update_camera(&mut self.world.player_camera, dt);

        // self.renderer
        //     .update_terrain(self.terrain_id, &self.world.terrain);

        self.renderer.update_text(
            self.debug_text,
            &format!(
                "Debug Mode: {}\nTick Rate: {:?}\nRender Time:{:?}",
                if self.settings.debug_mode_active {
                    "ON"
                } else {
                    "OFF"
                },
                self.tick_rate,
                self.render_time,
            ),
        );

        self.renderer.render(
            app,
            &self.world.ui_camera,
            &self.world.player_camera,
            self.settings.debug_mode_active,
        );
        self.render_time = render_timer.elapsed();
    }

    pub(crate) fn handle_close_requested(&mut self, app: &AppController) {
        self.exit(app);
    }

    pub(crate) fn handle_mouse_motion(&mut self, dx: f32, dy: f32) {
        if self.lmb_pressed {
            self.camera_controller.process_mouse(dx, dy);
            let size = self.window.inner_size();
            self.window
                .set_cursor_position(PhysicalPosition::new(size.width / 2, size.height / 2))
                .unwrap();
        }
    }

    pub(crate) fn handle_key(&mut self, app: &AppController, key: KeyCode, is_pressed: bool) {
        if self.camera_controller.process_keyboard(key, is_pressed) {
            return;
        }

        match (key, is_pressed) {
            (KeyCode::Escape, _) => self.exit(app),
            (KeyCode::KeyF, true) => self.toggle_fullscreen(),
            (KeyCode::Digit0, true) => {
                self.settings.debug_mode_active = !self.settings.debug_mode_active
            }
            _ => {}
        }
    }

    pub(crate) fn handle_axis(&self, axis: gilrs::Axis, _amount: f32) {
        match axis {
            _ => {}
        }
    }

    pub(crate) fn handle_mouse_button(&mut self, button: MouseButton, is_pressed: bool) {
        match button {
            MouseButton::Left => {
                self.lmb_pressed = is_pressed;
                self.window.set_cursor_visible(!is_pressed);
            }
            _ => {}
        }
    }

    pub(crate) fn handle_mouse_scroll(&mut self, delta: MouseScrollDelta) {
        self.camera_controller.process_mouse_scroll(&delta);
    }

    fn toggle_fullscreen(&mut self) {
        match self.window.fullscreen() {
            Some(_) => self.window.set_fullscreen(None),
            None => self
                .window
                .set_fullscreen(Some(Fullscreen::Borderless(None))),
        }
        self.settings.fullscreen = self.window.fullscreen().is_some();
    }

    fn exit(&mut self, app: &AppController) {
        app.spawn_task({
            let settings = self.settings.clone();
            let app = app.clone();
            async move {
                let data = serde_json::to_string_pretty(&settings)?;
                app.save_string("settings.json", data).await?;
                app.exit();
                Ok(())
            }
        });
    }
}
