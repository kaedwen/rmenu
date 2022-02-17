mod app;
mod cli;
mod command;
mod renderer;

use app::App;

fn main() -> std::io::Result<()> {
    env_logger::init();

    let config = cli::parse().expect("A valid Config");

    App::new()?.run(config)
}
