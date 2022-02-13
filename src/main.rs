mod app;
mod command;
mod renderer;

use smithay_client_toolkit::reexports::calloop::{self, EventLoop};

use app::{App, LoopContext};

fn main() -> std::io::Result<()> {
    env_logger::init();

    let mut app = App::new()?;

    loop {
        app.app_loop();
    }
}
