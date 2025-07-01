use std::{
    path::{Path, PathBuf},
    sync::Arc,
};

use anyhow::Context;
use async_channel::bounded;
use winit::{
    application::ApplicationHandler,
    event::{DeviceEvent, KeyEvent, WindowEvent},
    event_loop::{ActiveEventLoop, EventLoopProxy},
    keyboard::PhysicalKey,
    window::WindowAttributes,
};

use crate::game::Game;

pub enum AppEvent {
    GameStarted(Game),
    Exit,
    LoadString(PathBuf, async_channel::Sender<anyhow::Result<String>>),
    LoadBinary(PathBuf, async_channel::Sender<anyhow::Result<Vec<u8>>>),
}

impl std::fmt::Debug for AppEvent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AppEvent::GameStarted(_) => f.debug_tuple("GameStarted").field(&"..").finish(),
            AppEvent::Exit => f.write_str("Exit"),
            AppEvent::LoadString(path_buf, ..) => f
                .debug_tuple("LoadString")
                .field(path_buf)
                .field(&"..")
                .finish(),
            AppEvent::LoadBinary(path_buf, ..) => f
                .debug_tuple("LoadBinary")
                .field(path_buf)
                .field(&"..")
                .finish(),
        }
    }
}

pub struct App {
    game: Option<Game>,
    controller: AppController,
    gamepads: gilrs::Gilrs,
}

impl App {
    pub fn new(proxy: EventLoopProxy<AppEvent>, res_dir: impl Into<PathBuf>) -> Self {
        let gamepads = gilrs::GilrsBuilder::new().build().unwrap();
        Self {
            game: None,
            gamepads,
            controller: AppController {
                proxy,
                res_dir: res_dir.into(),
            },
        }
    }

    fn spawn_task<Fut>(&self, task: Fut)
    where
        // F: Send + 'static + FnOnce() -> Fut,
        Fut: Future<Output = anyhow::Result<()>> + Send + 'static,
    {
        let app = self.controller.clone();
        std::thread::spawn(move || {
            match pollster::block_on(task) {
                Ok(_) => {}
                Err(e) => {
                    log::error!("{e}");
                    app.exit();
                }
            };
        });
    }
}

impl ApplicationHandler<AppEvent> for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        let window_attributes = WindowAttributes::default();
        let app = self.controller.clone();
        let window = Arc::new(event_loop.create_window(window_attributes).unwrap());

        self.spawn_task(async move {
            let game = Game::new(&app, window).await?;
            app.proxy.send_event(AppEvent::GameStarted(game))?;
            Ok(())
        });
    }

    fn device_event(
        &mut self,
        _event_loop: &ActiveEventLoop,
        _device_id: winit::event::DeviceId,
        event: DeviceEvent,
    ) {
        let game = match &mut self.game {
            Some(game) => game,
            None => return,
        };

        match event {
            DeviceEvent::MouseMotion { delta: (dx, dy) } => {
                game.handle_mouse_motion(dx as _, dy as _);
            }
            // DeviceEvent::MouseWheel { delta } => {
            //     // game.handle_m
            // }
            _ => {}
        }
    }

    fn user_event(&mut self, event_loop: &ActiveEventLoop, event: AppEvent) {
        match event {
            AppEvent::GameStarted(game) => self.game = Some(game),
            AppEvent::Exit => event_loop.exit(),
            AppEvent::LoadString(path, sender) => {
                self.spawn_task(async move {
                    sender
                        .send(
                            async_fs::read_to_string(&path).await.with_context(|| {
                                format!("Could not load string: {}", path.display())
                            }),
                        )
                        .await;
                    Ok(())
                });
            }
            AppEvent::LoadBinary(path, sender) => {
                self.spawn_task(async move {
                    sender
                        .send(
                            async_fs::read(&path).await.with_context(|| {
                                format!("Could not load string: {}", path.display())
                            }),
                        )
                        .await;
                    Ok(())
                });
            }
        }
    }

    fn window_event(
        &mut self,
        _event_loop: &ActiveEventLoop,
        _window_id: winit::window::WindowId,
        event: WindowEvent,
    ) {
        let game = match &mut self.game {
            Some(game) => game,
            None => return,
        };

        let app = &self.controller;

        match event {
            WindowEvent::CloseRequested => {
                game.handle_close_requested(app);
            }
            WindowEvent::KeyboardInput {
                event:
                    KeyEvent {
                        physical_key: PhysicalKey::Code(key),
                        state,
                        ..
                    },
                ..
            } => {
                game.handle_key(&self.controller, key, state.is_pressed());
            }
            WindowEvent::RedrawRequested => game.render(app),
            WindowEvent::Resized(size) => game.resize(size.width, size.height),
            _ => {}
        }
    }

    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
        let game = match &mut self.game {
            Some(game) => game,
            None => return,
        };

        while let Some(event) = self.gamepads.next_event() {
            match event.event {
                gilrs::EventType::AxisChanged(axis, mut amount, ..) => {
                    if amount.abs() < 0.1 {
                        amount = 0.0;
                    }
                    game.handle_axis(axis, amount);
                }
                gilrs::EventType::Connected => {}
                gilrs::EventType::Disconnected => {}
                _ => {}
            }
        }
    }
}

#[derive(Clone)]
pub struct AppController {
    res_dir: PathBuf,
    proxy: EventLoopProxy<AppEvent>,
}

impl AppController {
    pub fn exit(&self) {
        self.proxy.send_event(AppEvent::Exit).unwrap();
    }

    pub(crate) async fn load_string(&self, path: impl AsRef<Path>) -> anyhow::Result<String> {
        let path = self.res_dir.join(path);
        let (sender, receiver) = bounded(1);
        self.proxy
            .send_event(AppEvent::LoadString(path, sender))
            .unwrap();
        receiver.recv().await?
    }

    pub(crate) async fn load_binary(&self, path: impl AsRef<Path>) -> anyhow::Result<Vec<u8>> {
        let path = self.res_dir.join(path);
        let (sender, receiver) = bounded(1);
        self.proxy
            .send_event(AppEvent::LoadBinary(path, sender))
            .unwrap();
        receiver.recv().await?
    }
}
