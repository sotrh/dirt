use std::{path::PathBuf, sync::Arc};

use winit::{keyboard::KeyCode, window::Window};

use crate::{
    app::{App, AppController},
    game::{render::Renderer, world::{camera::{Camera2d, PerspectiveCamera}, World}},
};

mod render;
mod world;

pub struct Game {
    renderer: Renderer,
    window: Arc<Window>,
    world: World,
}

impl Game {
    pub async fn new(app: &AppController, window: Arc<Window>) -> anyhow::Result<Self> {
        let renderer = Renderer::new(app, window.clone()).await?;
        let world = World::new();

        window.request_redraw();

        Ok(Self {
            renderer,
            window,
            world,
        })
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        self.renderer.resize(width, height);
        self.world.resize(width, height);
    }

    pub fn render(&mut self, app: &AppController) {
        self.window.request_redraw();
        self.renderer.render(app);
    }

    pub(crate) fn handle_close_requested(&mut self, app: &AppController) {
        app.exit();
    }

    pub(crate) fn handle_mouse_motion(&mut self, dx: f32, dy: f32) {}

    pub(crate) fn handle_key(&mut self, app: &AppController, key: KeyCode, is_pressed: bool) {
        match (key, is_pressed) {
            (KeyCode::Escape, _) => app.exit(),
            _ => {}
        }
    }
    
    pub(crate) fn handle_axis(&self, axis: gilrs::Axis, amount: f32) {
        match axis {

            _ => {}
        }
    }
    
    
}
