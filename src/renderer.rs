use std::{
    cell::{Cell, RefCell},
    rc::Rc,
    sync::Arc,
};

use raqote::{DrawOptions, DrawTarget, Point, SolidSource, Source};

use font_kit::font::Font;
use log::{debug, error, info, warn};
use smithay_client_toolkit::{
    environment::Environment,
    output::{with_output_info, OutputInfo},
    reexports::{
        calloop::{self, EventLoop},
        protocols::wlr::unstable::layer_shell::v1::client::{
            zwlr_layer_shell_v1, zwlr_layer_surface_v1,
        },
    },
    seat::{
        keyboard::{map_keyboard_repeat, Event as KbEvent, KeyState, RepeatKind},
        with_seat_data,
    },
    shm::AutoMemPool,
};
use wayland_client::{
    protocol::{wl_keyboard, wl_output, wl_shm, wl_surface},
    Attached, Main,
};

use super::{
    app::{AppContext, LoopAction, LoopContext, RMenuEnv},
    command,
};

//pub static FREEMONO_REGULAR_FONT: &'static [u8; 584424] = include_bytes!("../FreeMono.ttf");
//pub static ROBOTO_REGULAR_FONT: &'static [u8; 289080] = include_bytes!("../Roboto-Medium.ttf");
pub static SHARETECH_REGULAR_FONT: &'static [u8; 42756] =
    include_bytes!("../ShareTechMono-Regular.ttf");

#[derive(PartialEq, Copy, Clone)]
enum RenderEvent {
    Configure(i32, i32),
    Closed,
}

pub struct Surface {
    surface: wl_surface::WlSurface,
    layer_surface: Main<zwlr_layer_surface_v1::ZwlrLayerSurfaceV1>,
    next_render_event: Rc<Cell<Option<RenderEvent>>>,
    dimensions: Option<(i32, i32)>,
    context: Rc<RefCell<AppContext>>,
    pool: RefCell<AutoMemPool>,
    font: Font,
}

pub struct Renderer {
    env: Environment<RMenuEnv>,
    context: Rc<RefCell<AppContext>>,
    surfaces: Rc<RefCell<Vec<(u32, Surface)>>>,
}

impl Surface {
    fn new(
        output: &wl_output::WlOutput,
        surface: wl_surface::WlSurface,
        layer_shell: &Attached<zwlr_layer_shell_v1::ZwlrLayerShellV1>,
        context: Rc<RefCell<AppContext>>,
        pool: RefCell<AutoMemPool>,
    ) -> Self {
        let layer_surface = layer_shell.get_layer_surface(
            &surface,
            Some(output),
            zwlr_layer_shell_v1::Layer::Top,
            String::from("rmenu"),
        );

        // request exclusive keyboard events for our layer
        layer_surface
            .set_keyboard_interactivity(zwlr_layer_surface_v1::KeyboardInteractivity::OnDemand);

        // request to expand to the surface edges ignoring exlusive zones
        layer_surface.set_exclusive_zone(-1);

        // set hight and leave widht 0 to strtch the whole screen in configure
        layer_surface.set_size(0, 32);

        // Anchor to the top left/right corner of the output
        layer_surface.set_anchor(
            zwlr_layer_surface_v1::Anchor::Top
                | zwlr_layer_surface_v1::Anchor::Left
                | zwlr_layer_surface_v1::Anchor::Right,
        );

        let next_render_event = Rc::new(Cell::new(None::<RenderEvent>));
        let next_render_event_handle = Rc::clone(&next_render_event);
        layer_surface.quick_assign(move |layer_surface, event, _| {
            match (event, next_render_event_handle.get()) {
                (zwlr_layer_surface_v1::Event::Closed, _) => {
                    next_render_event_handle.set(Some(RenderEvent::Closed));
                }
                (
                    zwlr_layer_surface_v1::Event::Configure {
                        serial,
                        width,
                        height,
                    },
                    next,
                ) if next != Some(RenderEvent::Closed) => {
                    layer_surface.ack_configure(serial);
                    next_render_event_handle
                        .set(Some(RenderEvent::Configure(width as i32, height as i32)));
                }
                (_, _) => {}
            }
        });

        // Commit so that the server will send a configure event
        surface.commit();

        let font = font_kit::font::Font::from_bytes(Arc::new(SHARETECH_REGULAR_FONT.to_vec()), 0)
            .expect("To load FreeMono TTF");

        Self {
            surface,
            layer_surface,
            next_render_event,
            dimensions: None,
            context,
            pool,
            font,
        }
    }

    /// Handles any events that have occurred since the last call, redrawing if needed.
    /// Returns true if the surface should be dropped.
    pub fn handle_events(&mut self) -> bool {
        match self.next_render_event.take() {
            Some(RenderEvent::Closed) => true,
            Some(RenderEvent::Configure(width, height)) => {
                let dimensions = Some((width, height));
                if self.dimensions != dimensions {
                    self.dimensions = dimensions;
                    self.draw(dimensions);
                }
                false
            }
            None => false,
        }
    }

    fn draw(&self, dimensions: Option<(i32, i32)>) {
        if let Some((width, height)) = dimensions {
            match self
                .pool
                .borrow_mut()
                .buffer(width, height, 4 * width, wl_shm::Format::Argb8888)
            {
                Ok((canvas, buffer)) => {
                    let mut dt = DrawTarget::new(width, height);

                    let point_size = 32.;
                    let options = DrawOptions::new();
                    let fg_brush = Source::Solid(SolidSource {
                        r: 0xFF,
                        g: 0x00,
                        b: 0x00,
                        a: 0xFF,
                    });
                    let bg_brush = Source::Solid(SolidSource {
                        r: 0x00,
                        g: 0x00,
                        b: 0x00,
                        a: 0xFF,
                    });

                    let filter = &self.context.borrow().input.0;
                    let filter_text = format!("> {}", filter);

                    dt.fill_rect(0., 0., width as f32, height as f32, &bg_brush, &options);

                    let offset = draw_text(
                        &mut dt,
                        &self.font,
                        point_size,
                        filter_text.as_str(),
                        Point::new(0., height as f32 * 4. / 5.),
                        &fg_brush,
                        &options,
                        2.,
                    );

                    let mut start_list = offset.max(200.);

                    // a little space just to be sure
                    start_list += 20.;

                    for name in self
                        .context
                        .borrow()
                        .list
                        .filtered
                        .iter()
                        .filter_map(|command| command.path.file_name().and_then(|s| s.to_str()))
                    {
                        start_list = draw_text(
                            &mut dt,
                            &self.font,
                            point_size,
                            name,
                            Point::new(start_list, height as f32 * 4. / 5.),
                            &fg_brush,
                            &options,
                            2.,
                        ) + 15.;

                        // break if we are outside
                        if start_list > width as f32 {
                            break;
                        }
                    }

                    for (src, dst) in dt
                        .get_data_u8()
                        .chunks_exact(4)
                        .zip(canvas.chunks_exact_mut(4))
                    {
                        dst[0] = src[0];
                        dst[1] = src[1];
                        dst[2] = src[2];
                        dst[3] = src[3];
                    }

                    // Attach the buffer to the surface and mark the entire surface as damaged
                    self.surface.attach(Some(&buffer), 0, 0);
                    self.surface.damage_buffer(0, 0, width, height);

                    // Finally, commit the surface
                    self.surface.commit();
                }
                Err(e) => {
                    error!("Failed to request SHM Buffer! - {}", e);
                }
            }
        } else {
            warn!("No dimensions given!");
        }
    }

    pub fn redraw(&self) {
        self.draw(self.dimensions);
    }
}

impl Drop for Surface {
    fn drop(&mut self) {
        self.layer_surface.destroy();
        self.surface.destroy();
    }
}

impl Renderer {
    pub fn new(env: Environment<RMenuEnv>, context: Rc<RefCell<AppContext>>) -> Self {
        Renderer {
            env,
            context: context.to_owned(),
            surfaces: Rc::new(RefCell::new(Vec::<(u32, Surface)>::new())),
        }
    }
    pub fn init(&self, event_loop: &EventLoop<LoopContext>) {
        self.setup_keyboard_handler(event_loop);
        self.setup_output_handler();
    }
    pub fn render_loop(&self, redraw: bool) {
        let mut surfaces = self.surfaces.borrow_mut();
        let mut i = 0;
        while i != surfaces.len() {
            if surfaces[i].1.handle_events() {
                surfaces.remove(i);
            } else {
                if redraw {
                    surfaces[i].1.redraw();
                }

                i += 1;
            }
        }
    }
    fn setup_output_handler(&self) {
        let layer_shell = self
            .env
            .require_global::<zwlr_layer_shell_v1::ZwlrLayerShellV1>();

        let env_handle = self.env.clone();
        let context_handle = self.context.clone();
        let surfaces_handle = self.surfaces.clone();
        let output_handler = move |output: wl_output::WlOutput, info: &OutputInfo| {
            if info.obsolete {
                // an output has been removed, release it
                surfaces_handle.borrow_mut().retain(|(i, _)| *i != info.id);
                output.release();
            } else {
                // an output has been created, construct a surface for it
                let surface = env_handle.create_surface().detach();
                let pool = env_handle
                    .create_auto_pool()
                    .expect("Failed to create a memory pool!");
                (*surfaces_handle.borrow_mut()).push((
                    info.id,
                    Surface::new(
                        &output,
                        surface,
                        &layer_shell.clone(),
                        context_handle.clone(),
                        RefCell::new(pool),
                    ),
                ));
            }
        };

        // Process currently existing outputs
        for output in self.env.get_all_outputs() {
            if let Some(info) = with_output_info(&output, Clone::clone) {
                output_handler(output, &info);
            }
        }

        // Setup a listener for changes
        // The listener will live for as long as we keep this handle alive
        let _listner_handle = self
            .env
            .listen_for_outputs(move |output, info, _| output_handler(output, info));
    }
    fn setup_keyboard_handler(&self, event_loop: &EventLoop<LoopContext>) {
        let mut seats = Vec::<(
            String,
            Option<(wl_keyboard::WlKeyboard, calloop::RegistrationToken)>,
        )>::new();

        // first process already existing seats
        for seat in self.env.get_all_seats() {
            if let Some((has_kbd, name)) = with_seat_data(&seat, |seat_data| {
                (
                    seat_data.has_keyboard && !seat_data.defunct,
                    seat_data.name.clone(),
                )
            }) {
                debug!("{:?} - {} - {}", seat, has_kbd, name);
                if has_kbd {
                    let seat_name = name.clone();
                    match map_keyboard_repeat(
                        event_loop.handle(),
                        &seat,
                        None,
                        RepeatKind::System,
                        move |event, _, mut dispatch_data| {
                            let loop_context = dispatch_data
                                .get::<LoopContext>()
                                .expect("To get our Loop Context");
                            let context = &loop_context.app_context;

                            if Self::handle_keyboard_event(event, &seat_name, context) {
                                // apply filter
                                context.borrow_mut().filter();

                                loop_context.action.set(Some(LoopAction::Redraw));
                            }
                        },
                    ) {
                        Ok((kbd, repeat_source)) => {
                            seats.push((name, Some((kbd, repeat_source))));
                        }
                        Err(e) => {
                            eprintln!("Failed to map keyboard on seat {} : {:?}.", name, e);
                            seats.push((name, None));
                        }
                    }
                } else {
                    seats.push((name, None));
                }
            }
        }

        // then setup a listener for changes
        let loop_handle = event_loop.handle();
        /*let _seat_listener = self.env.listen_for_seats(move |seat, seat_data, _| {
            // find the seat in the vec of seats, or insert it if it is unknown
            let idx = seats.iter().position(|(name, _)| name == &seat_data.name);
            let idx = idx.unwrap_or_else(|| {
                seats.push((seat_data.name.clone(), None));
                seats.len() - 1
            });

            let (_, ref mut opt_kbd) = &mut seats[idx];
            // we should map a keyboard if the seat has the capability & is not defunct
            if seat_data.has_keyboard && !seat_data.defunct {
                if opt_kbd.is_none() {
                    // we should initalize a keyboard
                    let seat_name = seat_data.name.clone();
                    match map_keyboard_repeat(
                        loop_handle.clone(),
                        &seat,
                        None,
                        RepeatKind::System,
                        move |event, _, dispatch_data| {
                            let loop_context = dispatch_data
                                .get::<LoopContext>()
                                .expect("To get our Loop Context");
                            let context = &loop_context.app_context;

                            if Self::handle_keyboard_event(event, &seat_name, context) {
                                // apply filter
                                context.borrow_mut().filter();

                                loop_context.action.set(Some(LoopAction::Redraw));
                            }
                        },
                    ) {
                        Ok((kbd, repeat_source)) => {
                            *opt_kbd = Some((kbd, repeat_source));
                        }
                        Err(e) => {
                            eprintln!(
                                "Failed to map keyboard on seat {} : {:?}.",
                                seat_data.name, e
                            )
                        }
                    }
                }
            } else if let Some((kbd, source)) = opt_kbd.take() {
                // the keyboard has been removed, cleanup
                kbd.release();
                loop_handle.remove(source);
            }
        });*/
    }
    fn handle_keyboard_event(
        event: KbEvent,
        seat_name: &str,
        context: &Rc<RefCell<AppContext>>,
    ) -> bool {
        match event {
            KbEvent::Enter { keysyms, .. } => {
                debug!(
                    "Gained focus on seat '{}' while {} keys pressed.",
                    seat_name,
                    keysyms.len(),
                );
                false
            }
            KbEvent::Leave { .. } => {
                debug!("Lost focus on seat '{}'.", seat_name);
                false
            }
            KbEvent::Key {
                keysym,
                state,
                utf8,
                rawkey,
                ..
            } => {
                debug!("Key {:?}: {:x} on seat '{}'.", state, keysym, seat_name);
                if state == KeyState::Pressed {
                    Self::handle_key(&context, rawkey, utf8)
                } else {
                    false
                }
            }
            KbEvent::Modifiers { modifiers } => {
                debug!(
                    "Modifiers changed to {:?} on seat '{}'.",
                    modifiers, seat_name
                );
                false
            }
            KbEvent::Repeat {
                keysym,
                rawkey,
                utf8,
                ..
            } => {
                debug!("Key repetition {:x} on seat '{}'.", keysym, seat_name);
                Self::handle_key(&context, rawkey, utf8)
            }
        }
    }
    fn handle_key(context: &Rc<RefCell<AppContext>>, rawkey: u32, utf8: Option<String>) -> bool {
        match rawkey {
            /* ESC */
            1 => {
                // exit on ESC pressed
                std::process::exit(0);
            }
            /* BACKSPACE */
            14 => {
                // pop one char if backspace
                (*context.borrow_mut().input).pop();
                true
            }
            /* ENTER */
            28 => {
                // execute !!
                if let Some(target) = context.borrow().target() {
                    info!("Execute {}", target);

                    // launch and exit
                    std::process::exit(command::launch(target));
                } else {
                    // exit with failure (no target)
                    std::process::exit(10);
                }
            }
            _ => match utf8 {
                Some(txt) => {
                    debug!(" -> Received text \"{}\".", txt);

                    (*context.borrow_mut().input).push_str(txt.as_str());
                    true
                }
                _ => false,
            },
        }
    }
}

fn draw_text(
    dt: &mut DrawTarget,
    font: &Font,
    point_size: f32,
    text: &str,
    start: Point,
    src: &Source,
    options: &DrawOptions,
    space_factor: f32,
) -> f32 {
    let mut start = pathfinder_geometry::vector::vec2f(start.x, start.y);
    let mut ids = Vec::new();
    let mut positions = Vec::new();
    for c in text.chars() {
        let id = font.glyph_for_char(c).unwrap();
        ids.push(id);
        positions.push(Point::new(start.x(), start.y()));
        start += font.advance(id).unwrap() * point_size / 24. / 96. * space_factor;
    }
    dt.draw_glyphs(font, point_size, &ids, &positions, src, options);
    start.x()
}
