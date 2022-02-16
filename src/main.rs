mod app;
mod cli;
mod command;
mod renderer;

use app::App;

fn main() -> std::io::Result<()> {
    env_logger::init();

    App::new(cli::parse())?.run()
}
