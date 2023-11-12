mod app;
mod renderer;
mod command;
mod config;
mod menu;

use app::App;

fn main() -> anyhow::Result<()> {
  env_logger::init();

  let config = config::parse().expect("A valid Config");

  App::new().run(config)
}
