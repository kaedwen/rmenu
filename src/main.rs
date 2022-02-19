mod app;
mod config;
mod command;
mod renderer;

use app::App;

fn main() -> std::io::Result<()> {
    env_logger::init();

    let config = config::parse().expect("A valid Config");

    App::new()?.run(config)
}
