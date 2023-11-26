use log::info;

use smithay_client_toolkit::seat::keyboard;
use wayland_client::{globals::registry_queue_init, Connection};

use crate::{
  command,
  config::{self, AppConfig},
  menu,
};

pub struct AppContext {
  pub config: AppConfig,
  pub input: String,
  pub list: command::CommandList,
  pub current_index: usize,
  pub modifiers: keyboard::Modifiers,
}

pub struct App {}

impl AppContext {
  pub fn target(&self) -> Option<&command::Command> {
    self.list.filtered.get(self.current_index)
  }
  fn filter(&mut self) {
      self.list.filter(&self.input, &self.config.history);
      info!("{}", self.list);
  }
  pub fn pop_and_filter(&mut self) {
    self.input.pop();
    self.filter()
  }
  pub fn append_and_filter(&mut self, input: &str) {
    self.input.push_str(input);
    self.filter()
  }
}

impl App {
  pub fn new() -> App {
    App {}
  }
  pub fn run(&mut self, app_config: config::AppConfig) -> anyhow::Result<()> {
    info!("Config {:?}", app_config);

    let app_context = AppContext {
      input: String::new(),
      list: command::CommandList::new(&app_config)?,
      modifiers: Default::default(),
      current_index: 0,
      config: app_config,
    };

    info!("{}", app_context.list);

    // All Wayland apps start by connecting the compositor (server).
    let conn = Connection::connect_to_env().expect("Failed to connect to compositor");

    // Enumerate the list of globals to get the protocols the server implements.
    let (globals, mut event_queue) =
      registry_queue_init(&conn).expect("Failed to initialize queue");
    let qh = event_queue.handle();

    let mut menu_shell = menu::Shell::new(app_context, globals, qh);

    // Run the loop until exit
    while !menu_shell.about_to_exit() {
      event_queue.blocking_dispatch(&mut menu_shell)?;
    }

    Ok(())
  }
}
