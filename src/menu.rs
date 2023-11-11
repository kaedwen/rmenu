use smithay_client_toolkit::{
  compositor::CompositorHandler,
  delegate_compositor, delegate_keyboard, delegate_layer, delegate_output, delegate_registry,
  delegate_seat, delegate_shm,
  output::{OutputHandler, OutputState},
  registry::{ProvidesRegistryState, RegistryState},
  registry_handlers,
  seat::{keyboard::KeyboardHandler, SeatHandler, SeatState},
  shell::wlr_layer::LayerShellHandler,
  shm::{slot::SlotPool, Shm, ShmHandler},
};

use wayland_client::{
  globals::GlobalList,
  protocol::{wl_keyboard::WlKeyboard, wl_output, wl_surface},
  Connection, QueueHandle,
};

pub struct Shell {
  registry_state: RegistryState,
  seat_state: SeatState,
  output_state: OutputState,
  shm: Shm,

  exit: bool,
  pool: SlotPool,
  keyboard: Option<WlKeyboard>,
  keyboard_focus: bool,
}

impl CompositorHandler for Shell {
  fn frame(
    &mut self,
    conn: &Connection,
    qh: &QueueHandle<Self>,
    surface: &wl_surface::WlSurface,
    time: u32,
  ) {
  }

  fn scale_factor_changed(
    &mut self,
    conn: &Connection,
    qh: &QueueHandle<Self>,
    surface: &wl_surface::WlSurface,
    new_factor: i32,
  ) {
  }

  fn transform_changed(
    &mut self,
    conn: &Connection,
    qh: &QueueHandle<Self>,
    surface: &wl_surface::WlSurface,
    new_transform: wl_output::Transform,
  ) {
  }
}

impl OutputHandler for Shell {
  fn new_output(&mut self, conn: &Connection, qh: &QueueHandle<Self>, output: wl_output::WlOutput) {
  }

  fn output_destroyed(
    &mut self,
    conn: &Connection,
    qh: &QueueHandle<Self>,
    output: wl_output::WlOutput,
  ) {
  }

  fn output_state(&mut self) -> &mut OutputState {
    &mut self.output_state
  }

  fn update_output(
    &mut self,
    conn: &Connection,
    qh: &QueueHandle<Self>,
    output: wl_output::WlOutput,
  ) {
  }
}

impl LayerShellHandler for Shell {
  fn closed(
    &mut self,
    conn: &Connection,
    qh: &QueueHandle<Self>,
    layer: &smithay_client_toolkit::shell::wlr_layer::LayerSurface,
  ) {
    self.exit = true;
  }

  fn configure(
    &mut self,
    conn: &Connection,
    qh: &QueueHandle<Self>,
    layer: &smithay_client_toolkit::shell::wlr_layer::LayerSurface,
    configure: smithay_client_toolkit::shell::wlr_layer::LayerSurfaceConfigure,
    serial: u32,
  ) {
  }
}

impl SeatHandler for Shell {
  fn new_capability(
    &mut self,
    conn: &Connection,
    qh: &QueueHandle<Self>,
    seat: wayland_client::protocol::wl_seat::WlSeat,
    capability: smithay_client_toolkit::seat::Capability,
  ) {
  }

  fn new_seat(
    &mut self,
    conn: &Connection,
    qh: &QueueHandle<Self>,
    seat: wayland_client::protocol::wl_seat::WlSeat,
  ) {
  }

  fn remove_capability(
    &mut self,
    conn: &Connection,
    qh: &QueueHandle<Self>,
    seat: wayland_client::protocol::wl_seat::WlSeat,
    capability: smithay_client_toolkit::seat::Capability,
  ) {
  }

  fn remove_seat(
    &mut self,
    conn: &Connection,
    qh: &QueueHandle<Self>,
    seat: wayland_client::protocol::wl_seat::WlSeat,
  ) {
  }

  fn seat_state(&mut self) -> &mut SeatState {
    &mut self.seat_state
  }
}

impl KeyboardHandler for Shell {
  fn enter(
    &mut self,
    conn: &Connection,
    qh: &QueueHandle<Self>,
    keyboard: &wayland_client::protocol::wl_keyboard::WlKeyboard,
    surface: &wl_surface::WlSurface,
    serial: u32,
    raw: &[u32],
    keysyms: &[smithay_client_toolkit::seat::keyboard::Keysym],
  ) {
  }

  fn leave(
    &mut self,
    conn: &Connection,
    qh: &QueueHandle<Self>,
    keyboard: &wayland_client::protocol::wl_keyboard::WlKeyboard,
    surface: &wl_surface::WlSurface,
    serial: u32,
  ) {
  }

  fn press_key(
    &mut self,
    conn: &Connection,
    qh: &QueueHandle<Self>,
    keyboard: &wayland_client::protocol::wl_keyboard::WlKeyboard,
    serial: u32,
    event: smithay_client_toolkit::seat::keyboard::KeyEvent,
  ) {
  }

  fn release_key(
    &mut self,
    conn: &Connection,
    qh: &QueueHandle<Self>,
    keyboard: &wayland_client::protocol::wl_keyboard::WlKeyboard,
    serial: u32,
    event: smithay_client_toolkit::seat::keyboard::KeyEvent,
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
    conn: &Connection,
    qh: &QueueHandle<Self>,
    keyboard: &wayland_client::protocol::wl_keyboard::WlKeyboard,
    serial: u32,
    modifiers: smithay_client_toolkit::seat::keyboard::Modifiers,
  ) {
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
  pub fn new(globals: GlobalList, queue_handle: QueueHandle<Shell>) -> Shell {
    // Since we are not using the GPU in this example, we use wl_shm to allow software rendering to a buffer
    // we share with the compositor process.
    let shm = Shm::bind(&globals, &queue_handle).expect("wl_shm is not available");

    // We don't know how large the window will be yet, so lets assume the minimum size we suggested for the
    // initial memory allocation.
    let pool = SlotPool::new(256 * 256 * 4, &shm).expect("Failed to create pool");

    Shell {
      registry_state: RegistryState::new(&globals),
      seat_state: SeatState::new(&globals, &queue_handle),
      output_state: OutputState::new(&globals, &queue_handle),
      exit: false,
      keyboard: None,
      keyboard_focus: false,
      shm,
      pool,
    }
  }

  pub fn about_to_exit(&self) -> bool {
    self.exit
  }

  pub fn draw(&mut self, qh: &QueueHandle<Self>) {}
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
