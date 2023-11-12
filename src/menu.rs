use log::{debug, info, warn};
use smithay_client_toolkit::{
  compositor::{CompositorHandler, CompositorState},
  delegate_compositor, delegate_keyboard, delegate_layer, delegate_output, delegate_registry,
  delegate_seat, delegate_shm,
  output::{OutputHandler, OutputState},
  registry::{ProvidesRegistryState, RegistryState},
  registry_handlers,
  seat::{
    keyboard::{KeyboardHandler, Keysym},
    Capability, SeatHandler, SeatState,
  },
  shell::wlr_layer::{LayerShell, LayerShellHandler},
  shell::{
    wlr_layer::{Anchor, KeyboardInteractivity, Layer, LayerSurface},
    WaylandSurface,
  },
  shm::{slot::SlotPool, Shm, ShmHandler},
};

use wayland_client::{
  globals::GlobalList,
  protocol::{wl_keyboard::WlKeyboard, wl_output, wl_shm, wl_surface},
  Connection, QueueHandle,
};

use crate::{app::AppContext, command, renderer::Renderer};

static DEFAULT_HEIGHT: u32 = 32;

pub struct Shell {
  registry_state: RegistryState,
  seat_state: SeatState,
  output_state: OutputState,
  shm: Shm,

  exit: bool,
  pool: SlotPool,
  keyboard: Option<WlKeyboard>,

  size: (Option<u32>, Option<u32>),
  layer: LayerSurface,

  renderer: Renderer,
  context: AppContext,
}

impl CompositorHandler for Shell {
  fn frame(
    &mut self,
    _conn: &Connection,
    qh: &QueueHandle<Self>,
    _surface: &wl_surface::WlSurface,
    _time: u32,
  ) {
    self.draw(qh)
  }

  fn scale_factor_changed(
    &mut self,
    _conn: &Connection,
    _qh: &QueueHandle<Self>,
    _surface: &wl_surface::WlSurface,
    _new_factor: i32,
  ) {
  }

  fn transform_changed(
    &mut self,
    _conn: &Connection,
    _qh: &QueueHandle<Self>,
    _surface: &wl_surface::WlSurface,
    _new_transform: wl_output::Transform,
  ) {
  }
}

impl OutputHandler for Shell {
  fn new_output(
    &mut self,
    _conn: &Connection,
    _qh: &QueueHandle<Self>,
    _output: wl_output::WlOutput,
  ) {
  }

  fn output_destroyed(
    &mut self,
    _conn: &Connection,
    _qh: &QueueHandle<Self>,
    _output: wl_output::WlOutput,
  ) {
  }

  fn output_state(&mut self) -> &mut OutputState {
    &mut self.output_state
  }

  fn update_output(
    &mut self,
    _conn: &Connection,
    _qh: &QueueHandle<Self>,
    _output: wl_output::WlOutput,
  ) {
  }
}

impl LayerShellHandler for Shell {
  fn closed(
    &mut self,
    _conn: &Connection,
    _qh: &QueueHandle<Self>,
    _layer: &smithay_client_toolkit::shell::wlr_layer::LayerSurface,
  ) {
    self.exit = true;
  }

  fn configure(
    &mut self,
    _conn: &Connection,
    qh: &QueueHandle<Self>,
    _layer: &smithay_client_toolkit::shell::wlr_layer::LayerSurface,
    configure: smithay_client_toolkit::shell::wlr_layer::LayerSurfaceConfigure,
    _serial: u32,
  ) {
    let new_size = (Some(configure.new_size.0), Some(configure.new_size.1));
    if self.size != new_size {
      self.size = new_size;
      self.draw(qh);
    }
  }
}

impl SeatHandler for Shell {
  fn new_capability(
    &mut self,
    _conn: &Connection,
    qh: &QueueHandle<Self>,
    seat: wayland_client::protocol::wl_seat::WlSeat,
    capability: smithay_client_toolkit::seat::Capability,
  ) {
    if capability == Capability::Keyboard && self.keyboard.is_none() {
      debug!("Set keyboard capability");
      let keyboard = self
        .seat_state
        .get_keyboard(qh, &seat, None)
        .expect("Failed to create keyboard");
      self.keyboard = Some(keyboard);
    }
  }

  fn new_seat(
    &mut self,
    _conn: &Connection,
    _qh: &QueueHandle<Self>,
    _seat: wayland_client::protocol::wl_seat::WlSeat,
  ) {
  }

  fn remove_capability(
    &mut self,
    _conn: &Connection,
    _qh: &QueueHandle<Self>,
    _seat: wayland_client::protocol::wl_seat::WlSeat,
    capability: smithay_client_toolkit::seat::Capability,
  ) {
    if capability == Capability::Keyboard {
      if let Some(keyboard) = self.keyboard.as_ref() {
        debug!("Unset keyboard capability");
        keyboard.release();
      }
    }
  }

  fn remove_seat(
    &mut self,
    _conn: &Connection,
    _qh: &QueueHandle<Self>,
    _seat: wayland_client::protocol::wl_seat::WlSeat,
  ) {
  }

  fn seat_state(&mut self) -> &mut SeatState {
    &mut self.seat_state
  }
}

impl KeyboardHandler for Shell {
  fn enter(
    &mut self,
    _conn: &Connection,
    _qh: &QueueHandle<Self>,
    _keyboard: &wayland_client::protocol::wl_keyboard::WlKeyboard,
    _surface: &wl_surface::WlSurface,
    _serial: u32,
    _raw: &[u32],
    _keysyms: &[smithay_client_toolkit::seat::keyboard::Keysym],
  ) {
  }

  fn leave(
    &mut self,
    _conn: &Connection,
    _qh: &QueueHandle<Self>,
    _keyboard: &wayland_client::protocol::wl_keyboard::WlKeyboard,
    _surface: &wl_surface::WlSurface,
    _serial: u32,
  ) {
  }

  fn press_key(
    &mut self,
    _conn: &Connection,
    _qh: &QueueHandle<Self>,
    _keyboard: &wayland_client::protocol::wl_keyboard::WlKeyboard,
    _serial: u32,
    event: smithay_client_toolkit::seat::keyboard::KeyEvent,
  ) {
    self.handle_key(event);
  }

  fn release_key(
    &mut self,
    _conn: &Connection,
    _qh: &QueueHandle<Self>,
    _keyboard: &wayland_client::protocol::wl_keyboard::WlKeyboard,
    _serial: u32,
    _event: smithay_client_toolkit::seat::keyboard::KeyEvent,
  ) {
  }

  fn update_keymap(
    &mut self,
    _conn: &Connection,
    _qh: &QueueHandle<Self>,
    _keyboard: &wayland_client::protocol::wl_keyboard::WlKeyboard,
    _keymap: smithay_client_toolkit::seat::keyboard::Keymap<'_>,
  ) {
  }

  fn update_modifiers(
    &mut self,
    _conn: &Connection,
    _qh: &QueueHandle<Self>,
    _keyboard: &wayland_client::protocol::wl_keyboard::WlKeyboard,
    _serial: u32,
    modifiers: smithay_client_toolkit::seat::keyboard::Modifiers,
  ) {
    self.context.modifiers = modifiers;
  }

  fn update_repeat_info(
    &mut self,
    _conn: &Connection,
    _qh: &QueueHandle<Self>,
    _keyboard: &wayland_client::protocol::wl_keyboard::WlKeyboard,
    _info: smithay_client_toolkit::seat::keyboard::RepeatInfo,
  ) {
  }
}

impl ShmHandler for Shell {
  fn shm_state(&mut self) -> &mut Shm {
    &mut self.shm
  }
}

impl Shell {
  pub fn new(app_context: AppContext, globals: GlobalList, qh: QueueHandle<Shell>) -> Self {
    let height = app_context
      .config
      .static_config
      .style
      .as_ref()
      .map(|s| s.height)
      .unwrap_or(DEFAULT_HEIGHT);

    // The compositor (not to be confused with the server which is commonly called the compositor) allows
    // configuring surfaces to be presented.
    let compositor = CompositorState::bind(&globals, &qh).expect("wl_compositor is not available");

    // This app uses the wlr layer shell, which may not be available with every compositor.
    let layer_shell = LayerShell::bind(&globals, &qh).expect("layer shell is not available");

    // A layer surface is created from a surface.
    let surface = compositor.create_surface(&qh);

    // And then we create the layer shell.
    let layer =
      layer_shell.create_layer_surface(&qh, surface, Layer::Top, Some("menu_layer"), None);

    // request to expand to the surface edges ignoring exlusive zones
    layer.set_exclusive_zone(-1);

    // set hight and leave widht 0 to stretch the whole screen in configure
    layer.set_size(0, height);

    // Anchor to the top left/right corner of the output
    layer.set_anchor(Anchor::TOP | Anchor::LEFT | Anchor::RIGHT);

    // request exclusive keyboard events for our layer
    layer.set_keyboard_interactivity(KeyboardInteractivity::OnDemand);

    // In order for the layer surface to be mapped, we need to perform an initial commit with no attached\
    // buffer. For more info, see WaylandSurface::commit
    //
    // The compositor will respond with an initial configure that we can then use to present to the layer
    // surface with the correct options.
    layer.commit();

    // Since we are not using the GPU in this example, we use wl_shm to allow software rendering to a buffer
    // we share with the compositor process.
    let shm = Shm::bind(&globals, &qh).expect("wl_shm is not available");

    // We don't know how large the window will be yet, so lets assume the minimum size we suggested for the
    // initial memory allocation.
    let pool = SlotPool::new(256 * 256 * 4, &shm).expect("Failed to create pool");

    Self {
      registry_state: RegistryState::new(&globals),
      seat_state: SeatState::new(&globals, &qh),
      output_state: OutputState::new(&globals, &qh),
      renderer: Renderer::new(&app_context.config.static_config),
      exit: false,
      keyboard: None,
      shm,
      pool,
      size: (None, Some(height)),
      layer,
      context: app_context,
    }
  }

  pub fn about_to_exit(&self) -> bool {
    self.exit
  }

  pub fn draw(&mut self, qh: &QueueHandle<Self>) {
    let width = self.size.0.unwrap_or(0) as i32;
    let height = self.size.1.unwrap_or(0) as i32;
    let stride = width * 4 as i32;

    let (buffer, canvas) = self
      .pool
      .create_buffer(width, height, stride, wl_shm::Format::Argb8888)
      .expect("create buffer");

    self.renderer.render(&self.context, width, height, canvas);

    // Damage the entire window
    self.layer.wl_surface().damage_buffer(0, 0, width, height);

    // Request our next frame
    self
      .layer
      .wl_surface()
      .frame(qh, self.layer.wl_surface().clone());

    // Attach and commit to present.
    buffer
      .attach_to(self.layer.wl_surface())
      .expect("buffer attach");
    self.layer.commit();
  }

  fn handle_key(&mut self, event: smithay_client_toolkit::seat::keyboard::KeyEvent) {
    debug!("Key press: {event:?}");
    match event.keysym {
      Keysym::Escape => {
        self.exit = true;
      }
      Keysym::BackSpace => {
        // pop one char
        self.context.input.pop();
      }
      Keysym::Tab => {
        if self.context.modifiers.shift {
          if self.context.current_index > 0 {
            // shift index left
            self.context.current_index -= 1;
          } else {
            if self.context.current_index < self.context.list.filtered_len() {
              // shift index right
              self.context.current_index += 1;
            }
          }
        }
      }
      Keysym::Return => {
        let (exit, binary) = if let Some(target) = self.context.target() {
          info!("Execute {}", target);

          // launch
          (command::launch(target), Some(target.binary()))
          //(0, None)
        } else {
          // exit with failure (no target)
          (10, None)
        };

        // write history
        if let Some(binary) = binary {
          if let Err(e) = self.context.config.increment_and_store_history(binary) {
            warn!("Failed to store history data - {}", e);
          }
        }

        std::process::exit(exit);
      }
      _ => match event.utf8 {
        Some(txt) => {
          debug!(" -> Received text `{}`", txt);

          // append key to filter
          self.context.input.push_str(txt.as_str());

          // apply the filter
          self.context.filter();

          // reset current index
          self.context.current_index = 0;
        }
        _ => {
          debug!("Not handled KEY {:?}", event.utf8);
        }
      },
    }
  }
}

delegate_compositor!(Shell);
delegate_output!(Shell);
delegate_shm!(Shell);

delegate_seat!(Shell);
delegate_keyboard!(Shell);

delegate_layer!(Shell);

delegate_registry!(Shell);

impl ProvidesRegistryState for Shell {
  fn registry(&mut self) -> &mut RegistryState {
    &mut self.registry_state
  }
  registry_handlers![OutputState, SeatState];
}
