use winit::event_loop::EventLoop;

use crate::app::App;

mod app;
mod game;

pub fn run() -> anyhow::Result<()> {
    #[cfg(not(target_arch = "wasm32"))]
    {
        env_logger::init();
    }
    #[cfg(target_arch = "wasm32")]
    {
        console_log::init_with_level(log::Level::Info).unwrap_throw();
    }

    let event_loop = EventLoop::with_user_event().build()?;
    let proxy = event_loop.create_proxy();
    let mut app = App::new(proxy, "res");
    event_loop.run_app(&mut app)?;

    Ok(())
}
