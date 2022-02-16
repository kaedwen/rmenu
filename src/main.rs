mod app;
mod command;
mod renderer;

use app::App;

fn main() -> std::io::Result<()> {
    env_logger::init();

    App::run()
}
