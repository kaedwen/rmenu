use std::time::Duration;

use log::info;

use smithay_client_toolkit::{
  compositor::CompositorState,
  reexports::{calloop::EventLoop, calloop_wayland_source::WaylandSource},
  seat,
  shell::{
    wlr_layer::{Anchor, KeyboardInteractivity, Layer, LayerShell},
    WaylandSurface,
  },
};

use wayland_client::{globals::registry_queue_init, Connection};

use crate::{command, config, menu};

// #[derive(Debug)]
// pub struct LayerShell {
//     wlr_layer_shell: zwlr_layer_shell_v1::ZwlrLayerShellV1,
// }

// #[derive(PartialEq, Debug)]
// pub enum LoopAction {
//     Redraw,
// }

// #[derive(Clone)]
// pub struct LoopContext {
//     pub action: Rc<Cell<Option<LoopAction>>>,
//     pub app_context: Rc<RefCell<AppContext>>,
//     pub handle: LoopHandle<'static, LoopContext>,
// }

pub struct Filter(pub String);

pub struct AppContext {
  pub input: Filter,
  pub list: command::CommandList,
  pub app_config: config::AppConfig,
  pub current_index: usize,
  pub modifiers: seat::keyboard::Modifiers,
}

pub struct App {}

// impl Deref for Filter {
//     type Target = String;

//     fn deref(&self) -> &Self::Target {
//         &self.0
//     }
// }

// impl DerefMut for Filter {
//     fn deref_mut(&mut self) -> &mut String {
//         &mut self.0
//     }
// }

// impl LoopContext {
//     fn new(handle: LoopHandle<'static, LoopContext>, app_context: AppContext) -> Self {
//         Self {
//             action: Rc::new(Cell::new(None)),
//             app_context: Rc::new(RefCell::new(app_context)),
//             handle,
//         }
//     }
// }

// impl AppContext {
//     pub fn target(&self) -> Option<&command::Command> {
//         self.list.filtered.get(self.current_index)
//     }
//     pub fn filter(&mut self) {
//         self.list.filter(&self.input, &self.app_config.history);
//         info!("{}", self.list);
//     }
// }

// default_environment!(RMenuEnv,
//   fields = [
//       layer_shell: SimpleGlobal<zwlr_layer_shell_v1::ZwlrLayerShellV1>,
//   ],
//   singles = [
//       zwlr_layer_shell_v1::ZwlrLayerShellV1 => layer_shell
//   ],
// );

impl App {
  pub fn new() -> App {
    App {}
  }
  pub fn run(&mut self, app_config: config::AppConfig) -> anyhow::Result<()> {
    info!("Config {:?}", app_config);

    // All Wayland apps start by connecting the compositor (server).
    let conn = Connection::connect_to_env().expect("Failed to connect to compositor");

    // Enumerate the list of globals to get the protocols the server implements.
    let (globals, event_queue) = registry_queue_init(&conn).expect("Failed to initialize queue");
    let queue_handle = event_queue.handle();

    let mut event_loop: EventLoop<menu::Shell> =
      EventLoop::try_new().expect("Failed to initialize the event loop!");
    WaylandSource::new(conn.clone(), event_queue).insert(event_loop.handle())?;

    // The compositor (not to be confused with the server which is commonly called the compositor) allows
    // configuring surfaces to be presented.
    let compositor =
      CompositorState::bind(&globals, &queue_handle).expect("wl_compositor is not available");

    // This app uses the wlr layer shell, which may not be available with every compositor.
    let layer_shell =
      LayerShell::bind(&globals, &queue_handle).expect("layer shell is not available");

    // A layer surface is created from a surface.
    let surface = compositor.create_surface(&queue_handle);

    // And then we create the layer shell.
    let layer = layer_shell.create_layer_surface(
      &queue_handle,
      surface,
      Layer::Top,
      Some("menu_layer"),
      None,
    );

    // Configure the layer surface, providing things like the anchor on screen, desired size and the keyboard
    // interactivity
    layer.set_anchor(Anchor::TOP);
    layer.set_keyboard_interactivity(KeyboardInteractivity::OnDemand);
    layer.set_size(256, 256);

    // In order for the layer surface to be mapped, we need to perform an initial commit with no attached\
    // buffer. For more info, see WaylandSurface::commit
    //
    // The compositor will respond with an initial configure that we can then use to present to the layer
    // surface with the correct options.
    layer.commit();

    let mut menu_shell = menu::Shell::new(globals, queue_handle);

    let app_context = AppContext {
      input: Filter(String::new()),
      list: command::CommandList::new(&app_config)?,
      modifiers: Default::default(),
      current_index: 0,
      app_config,
    };

    info!("{}", app_context.list);

    // We don't draw immediately, the configure will notify us when to first draw.
    loop {
      event_loop.dispatch(Duration::from_millis(16), &mut menu_shell)?;

      if menu_shell.about_to_exit() {
        return Ok(());
      }
    }
  }
}
