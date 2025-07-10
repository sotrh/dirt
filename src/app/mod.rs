use std::{
    path::{Path, PathBuf}, pin::Pin, sync::Arc, time::Duration
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
    SaveString(PathBuf, String, async_channel::Sender<anyhow::Result<()>>),
    SaveBinary(PathBuf, Vec<u8>, async_channel::Sender<anyhow::Result<()>>),
    LoadString(PathBuf, async_channel::Sender<anyhow::Result<String>>),
    LoadBinary(PathBuf, async_channel::Sender<anyhow::Result<Vec<u8>>>),
    Task(Pin<Box<dyn Future<Output=anyhow::Result<()>> + Send + Sync + 'static>>),
}

impl std::fmt::Debug for AppEvent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AppEvent::GameStarted(_) => f.debug_tuple("GameStarted").field(&"..").finish(),
            AppEvent::Exit => f.write_str("Exit"),
            AppEvent::Task(_) => f.write_str("Task(..)"),
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
            AppEvent::SaveString(path_buf, data, sender) => f
                .debug_tuple("SaveString")
                .field(path_buf)
                .field(data)
                .field(sender)
                .finish(),
            AppEvent::SaveBinary(path_buf, items, sender) => f
                .debug_tuple("SaveBinary")
                .field(path_buf)
                .field(items)
                .field(sender)
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
        Fut: Future<Output = anyhow::Result<()>> + Send + 'static,
    {
        std::thread::spawn(move || {
            pollster::block_on(task).unwrap();
        });
    }
}

impl ApplicationHandler<AppEvent> for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        let window_attributes = WindowAttributes::default();
        let app = self.controller.clone();
        let window = Arc::new(event_loop.create_window(window_attributes).unwrap());

        self.spawn_task(async move {
            log::debug!("Creating game");
            std::thread::sleep(Duration::from_millis(1000));
            let game = Game::new(&app, window).await?;
            log::debug!("Game ready");
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
            DeviceEvent::MouseWheel { delta } => {
                game.handle_mouse_scroll(delta);
            }
            _ => {}
        }
    }

    fn user_event(&mut self, event_loop: &ActiveEventLoop, event: AppEvent) {
        match event {
            AppEvent::GameStarted(game) => {
                game.window.request_redraw();
                self.game = Some(game);
            }
            AppEvent::Exit => event_loop.exit(),
            AppEvent::Task(task) => {
                self.spawn_task(task);
            }
            AppEvent::LoadString(path, sender) => {
                self.spawn_task(async move {
                    sender
                        .send(
                            async_fs::read_to_string(&path).await.with_context(|| {
                                format!("Could not load string: {}", path.display())
                            }),
                        )
                        .await
                        .unwrap();
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
                        .await
                        .unwrap();
                    Ok(())
                });
            }
            AppEvent::SaveString(path, contents, sender) => {
                self.spawn_task(async move {
                    log::debug!("SaveString");
                    sender
                        .send(async_fs::write(&path, &contents).await.with_context(|| {
                            format!("Could not save string: {} to {}", contents, path.display())
                        }))
                        .await
                        .unwrap();
                    Ok(())
                });
            }
            AppEvent::SaveBinary(path, contents, sender) => {
                log::debug!("SaveBinary");
                self.spawn_task(async move {
                    sender
                        .send(async_fs::write(&path, &contents).await.with_context(|| {
                            format!("Could not save data: {:?} to {}", &contents, path.display())
                        }))
                        .await
                        .unwrap();
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
            WindowEvent::MouseInput { state, button, .. } => {
                game.handle_mouse_button(button, state.is_pressed())
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

    pub fn spawn_task<Fut>(&self, task: Fut)
    where
        Fut: Future<Output = anyhow::Result<()>> + Send + Sync + 'static,
    {
        self.proxy
            .send_event(AppEvent::Task(Box::pin(task)))
            .unwrap();
    }

    pub async fn save_string(&self, path: impl AsRef<Path>, data: String) -> anyhow::Result<()> {
        let path = self.res_dir.join(path);
        let (sender, receiver) = bounded(1);
        self.proxy
            .send_event(AppEvent::SaveString(path, data, sender))
            .unwrap();
        receiver.recv().await?
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
