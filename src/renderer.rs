// use std::{
//     cell::{Cell, RefCell},
//     rc::Rc,
//     sync::Arc,
//     time::Duration,
// };

// use font_kit::source::SystemSource;

// use raqote::{DrawOptions, DrawTarget, Point, SolidSource, Source};

// use font_kit::font::Font;
// use log::{debug, error, info, warn};
// use smithay_client_toolkit::{
//     environment::Environment,
//     output::{with_output_info, OutputInfo},
//     reexports::{
//         calloop::{self, timer::Timer},
//         protocols::wlr::unstable::layer_shell::v1::client::{
//             zwlr_layer_shell_v1, zwlr_layer_surface_v1,
//         },
//     },
//     seat::{
//         keyboard::{map_keyboard_repeat, Event as KbEvent, KeyState, RepeatKind},
//         with_seat_data,
//     },
//     shm::AutoMemPool,
// };
// use wayland_client::{
//     protocol::{wl_keyboard, wl_output, wl_shm, wl_surface},
//     Attached, Main,
// };

// use crate::config::Color;

// use super::{
//     app::{AppContext, LoopAction, LoopContext, RMenuEnv},
//     command,
// };

// static DEFAULT_HEIGHT: u32 = 32;

// static DEFAULT_FONT: &'static [u8; 42756] = include_bytes!("../assets/ShareTechMono-Regular.ttf");

// static DEFAULT_FONT_SIZE: f32 = 24.;
// static DEFAULT_FONT_SPACING: f32 = 2.;

// static DEFAULT_HIGHLIGHT: Color = Color {
//     r: 0xFF,
//     g: 0x00,
//     b: 0x00,
//     a: 0xFF,
// };

// static DEFAULT_FOREGROUND: Color = Color {
//     r: 0xD0,
//     g: 0xD0,
//     b: 0xD0,
//     a: 0xFF,
// };

// static DEFAULT_BACKGROUND: Color = Color {
//     r: 0x10,
//     g: 0x10,
//     b: 0x10,
//     a: 0xEE,
// };

// impl Into<SolidSource> for Color {
//     fn into(self) -> SolidSource {
//         SolidSource {
//             r: self.r,
//             g: self.g,
//             b: self.b,
//             a: self.a,
//         }
//     }
// }

// #[derive(PartialEq, Copy, Clone)]
// enum RenderEvent {
//     Configure(i32, i32),
//     Closed,
// }

// pub struct Surface {
//     surface: wl_surface::WlSurface,
//     layer_surface: Main<zwlr_layer_surface_v1::ZwlrLayerSurfaceV1>,
//     next_render_event: Rc<Cell<Option<RenderEvent>>>,
//     renderer_context: Rc<RefCell<RendererContext>>,
//     dimensions: Option<(i32, i32)>,
//     context: Rc<RefCell<AppContext>>,
//     pool: RefCell<AutoMemPool>,
//     cursor: Option<f32>,
// }

// struct RendererContext {
//     highlight: SolidSource,
//     foreground: SolidSource,
//     background: SolidSource,
//     font_spacing: f32,
//     font_size: f32,
//     font: Font,
// }

// pub struct Renderer {
//     env: Environment<RMenuEnv>,
//     surfaces: Rc<RefCell<Vec<(u32, Surface)>>>,
//     renderer_context: Rc<RefCell<RendererContext>>,
//     context: LoopContext,
// }

// impl Surface {
//     fn new(
//         output: &wl_output::WlOutput,
//         surface: wl_surface::WlSurface,
//         layer_shell: &Attached<zwlr_layer_shell_v1::ZwlrLayerShellV1>,
//         renderer_context: Rc<RefCell<RendererContext>>,
//         context: Rc<RefCell<AppContext>>,
//         pool: RefCell<AutoMemPool>,
//     ) -> Self {
//         let height = context
//             .borrow()
//             .app_config
//             .config
//             .style
//             .as_ref()
//             .map(|s| s.height)
//             .unwrap_or(DEFAULT_HEIGHT);

//         let layer_surface = layer_shell.get_layer_surface(
//             &surface,
//             Some(output),
//             zwlr_layer_shell_v1::Layer::Top,
//             String::from("rmenu"),
//         );

//         // request exclusive keyboard events for our layer
//         layer_surface
//             .set_keyboard_interactivity(zwlr_layer_surface_v1::KeyboardInteractivity::OnDemand);

//         // request to expand to the surface edges ignoring exlusive zones
//         layer_surface.set_exclusive_zone(-1);

//         // set hight and leave widht 0 to strtch the whole screen in configure
//         layer_surface.set_size(0, height);

//         // Anchor to the top left/right corner of the output
//         layer_surface.set_anchor(
//             zwlr_layer_surface_v1::Anchor::Top
//                 | zwlr_layer_surface_v1::Anchor::Left
//                 | zwlr_layer_surface_v1::Anchor::Right,
//         );

//         let next_render_event = Rc::new(Cell::new(None::<RenderEvent>));
//         let next_render_event_handle = Rc::clone(&next_render_event);
//         layer_surface.quick_assign(move |layer_surface, event, _| {
//             match (event, next_render_event_handle.get()) {
//                 (zwlr_layer_surface_v1::Event::Closed, _) => {
//                     next_render_event_handle.set(Some(RenderEvent::Closed));
//                 }
//                 (
//                     zwlr_layer_surface_v1::Event::Configure {
//                         serial,
//                         width,
//                         height,
//                     },
//                     next,
//                 ) if next != Some(RenderEvent::Closed) => {
//                     layer_surface.ack_configure(serial);
//                     next_render_event_handle
//                         .set(Some(RenderEvent::Configure(width as i32, height as i32)));
//                 }
//                 (_, _) => {}
//             }
//         });

//         // Commit so that the server will send a configure event
//         surface.commit();

//         Self {
//             surface,
//             layer_surface,
//             next_render_event,
//             renderer_context,
//             dimensions: None,
//             cursor: None,
//             context,
//             pool,
//         }
//     }

//     /// Handles any events that have occurred since the last call, redrawing if needed.
//     /// Returns true if the surface should be dropped.
//     pub fn handle_events(&mut self) -> bool {
//         match self.next_render_event.take() {
//             Some(RenderEvent::Closed) => true,
//             Some(RenderEvent::Configure(width, height)) => {
//                 let dimensions = Some((width, height));
//                 if self.dimensions != dimensions {
//                     self.dimensions = dimensions;
//                     self.draw(dimensions);
//                 }
//                 false
//             }
//             None => false,
//         }
//     }

//     fn draw(&mut self, dimensions: Option<(i32, i32)>) {
//         if let Some((width, height)) = dimensions {
//             match self
//                 .pool
//                 .borrow_mut()
//                 .buffer(width, height, 4 * width, wl_shm::Format::Argb8888)
//             {
//                 Ok((canvas, buffer)) => {
//                     let mut dt = DrawTarget::new(width, height);

//                     let renderer_context = self.renderer_context.borrow();
//                     let current_index = self.context.borrow().current_index;

//                     let options = DrawOptions::new();
//                     let point_size = renderer_context.font_size;
//                     let highlight_brush = Source::Solid(renderer_context.highlight);
//                     let foreground_brush = Source::Solid(renderer_context.foreground);
//                     let background_brush = Source::Solid(renderer_context.background);

//                     let filter = &self.context.borrow().input.0;
//                     let filter_text = format!("> {}", filter);

//                     dt.fill_rect(
//                         0.,
//                         0.,
//                         width as f32,
//                         height as f32,
//                         &background_brush,
//                         &options,
//                     );

//                     let offset = draw_text(
//                         &mut dt,
//                         &renderer_context.font,
//                         point_size,
//                         filter_text.as_str(),
//                         Point::new(0., height as f32 * 3. / 5.),
//                         &foreground_brush,
//                         &options,
//                         renderer_context.font_spacing,
//                     );

//                     self.cursor = Some(offset);

//                     let mut start_list = offset.max(200.);

//                     // a little space just to be sure
//                     start_list += 20.;

//                     if current_index > 0 {
//                         start_list = draw_text(
//                             &mut dt,
//                             &renderer_context.font,
//                             point_size,
//                             "<",
//                             Point::new(start_list, height as f32 * 3. / 5.),
//                             &foreground_brush,
//                             &options,
//                             renderer_context.font_spacing,
//                         ) + 15.;
//                     }

//                     for (index, name) in self
//                         .context
//                         .borrow()
//                         .list
//                         .filtered
//                         .iter()
//                         .map(|c| &c.name)
//                         .skip(current_index)
//                         .enumerate()
//                     {
//                         start_list = draw_text(
//                             &mut dt,
//                             &renderer_context.font,
//                             point_size,
//                             name,
//                             Point::new(start_list, height as f32 * 3. / 5.),
//                             if index == 0 {
//                                 &highlight_brush
//                             } else {
//                                 &foreground_brush
//                             },
//                             &options,
//                             renderer_context.font_spacing,
//                         ) + 15.;

//                         // break if we are outside
//                         if start_list > width as f32 {
//                             break;
//                         }
//                     }

//                     for (src, dst) in dt
//                         .get_data_u8()
//                         .chunks_exact(4)
//                         .zip(canvas.chunks_exact_mut(4))
//                     {
//                         dst[0] = src[0];
//                         dst[1] = src[1];
//                         dst[2] = src[2];
//                         dst[3] = src[3];
//                     }

//                     // Attach the buffer to the surface and mark the entire surface as damaged
//                     self.surface.attach(Some(&buffer), 0, 0);
//                     self.surface.damage_buffer(0, 0, width, height);

//                     // Finally, commit the surface
//                     self.surface.commit();
//                 }
//                 Err(e) => {
//                     error!("Failed to request SHM Buffer! - {}", e);
//                 }
//             }
//         } else {
//             warn!("No dimensions given!");
//         }
//     }
//     pub fn redraw(&mut self) {
//         self.draw(self.dimensions);
//     }
//     pub fn draw_cursor(&self, draw: bool) {
//         if let (Some((width, height)), Some(offset)) = (self.dimensions, self.cursor) {
//             match self
//                 .pool
//                 .borrow_mut()
//                 .buffer(width, height, 4 * width, wl_shm::Format::Argb8888)
//             {
//                 Ok((canvas, buffer)) => {
//                     let mut dt = DrawTarget::new(width, height);

//                     let renderer_context = self.renderer_context.borrow();

//                     let options = DrawOptions::new();
//                     let point_size = renderer_context.font_size;
//                     let fg_brush = Source::Solid(renderer_context.foreground);
//                     let bg_brush = Source::Solid(renderer_context.background);

//                     dt.fill_rect(0., 0., width as f32, height as f32, &bg_brush, &options);

//                     let new_offset = draw_text(
//                         &mut dt,
//                         &renderer_context.font,
//                         point_size,
//                         "|",
//                         Point::new(offset, height as f32 * 3. / 5.),
//                         if draw { &fg_brush } else { &bg_brush },
//                         &options,
//                         renderer_context.font_spacing,
//                     );

//                     for (src, dst) in dt
//                         .get_data_u8()
//                         .chunks_exact(4)
//                         .zip(canvas.chunks_exact_mut(4))
//                     {
//                         dst[0] = src[0];
//                         dst[1] = src[1];
//                         dst[2] = src[2];
//                         dst[3] = src[3];
//                     }

//                     // Attach the buffer to the surface and mark the new part as damaged
//                     self.surface.attach(Some(&buffer), 0, 0);
//                     self.surface.damage_buffer(
//                         offset as i32,
//                         0,
//                         (new_offset - offset) as i32,
//                         height,
//                     );

//                     // Finally, commit the surface
//                     self.surface.commit();
//                 }
//                 Err(e) => {
//                     error!("Failed to request SHM Buffer! - {}", e);
//                 }
//             }
//         }
//     }
// }

// impl Drop for Surface {
//     fn drop(&mut self) {
//         self.layer_surface.destroy();
//         self.surface.destroy();
//     }
// }

// impl Renderer {
//     pub fn new(env: Environment<RMenuEnv>, context: LoopContext) -> Self {
//         let renderer_context = {
//             let app_config = &context.app_context.borrow().app_config;

//             let font = app_config
//                 .config
//                 .font
//                 .as_ref()
//                 .and_then(|font| {
//                     let o1 = font.path.as_ref().and_then(|path| {
//                         std::fs::read(path).ok().and_then(|data| {
//                             font_kit::font::Font::from_bytes(Arc::new(data), 0).ok()
//                         })
//                     });

//                     let o2 = font.name.as_ref().and_then(|name| {
//                         println!("{}", name);
//                         SystemSource::new()
//                             .select_by_postscript_name(&name)
//                             .ok()
//                             .and_then(|h| h.load().ok())
//                     });

//                     debug!("FONT path {:?}, name {:?}", o1, o2);

//                     o1.or(o2)
//                 })
//                 .unwrap_or(
//                     font_kit::font::Font::from_bytes(Arc::new(DEFAULT_FONT.to_vec()), 0)
//                         .expect("To load FreeMono TTF"),
//                 );

//             Rc::new(RefCell::new(RendererContext {
//                 font,
//                 highlight: app_config
//                     .config
//                     .style
//                     .as_ref()
//                     .and_then(|style| style.highlight_color.clone())
//                     .unwrap_or(DEFAULT_HIGHLIGHT)
//                     .into(),
//                 foreground: app_config
//                     .config
//                     .style
//                     .as_ref()
//                     .and_then(|style| style.foreground_color.clone())
//                     .unwrap_or(DEFAULT_FOREGROUND)
//                     .into(),
//                 background: app_config
//                     .config
//                     .style
//                     .as_ref()
//                     .and_then(|style| style.background_color.clone())
//                     .unwrap_or(DEFAULT_BACKGROUND)
//                     .into(),
//                 font_spacing: app_config
//                     .config
//                     .font
//                     .as_ref()
//                     .and_then(|f| f.spacing)
//                     .unwrap_or(DEFAULT_FONT_SPACING),
//                 font_size: app_config
//                     .config
//                     .font
//                     .as_ref()
//                     .and_then(|f| f.size)
//                     .unwrap_or(DEFAULT_FONT_SIZE),
//             }))
//         };

//         let renderer = Renderer {
//             surfaces: Rc::new(RefCell::new(Vec::<(u32, Surface)>::new())),
//             renderer_context,
//             context,
//             env,
//         };

//         renderer.init();

//         renderer
//     }
//     fn init(&self) {
//         self.setup_keyboard_handler();
//         self.setup_output_handler();
//         self.setup_cursor();
//     }
//     fn setup_cursor(&self) {
//         let source = Timer::new().expect("Failed to create timer event source!");
//         source.handle().add_timeout(Duration::from_millis(500), ());

//         let mut draw = true;
//         let surfaces_handle = self.surfaces.clone();
//         self.context
//             .handle
//             .insert_source(source, move |_, metadata, _| {
//                 for (_, surface) in surfaces_handle.borrow().iter() {
//                     surface.draw_cursor(draw);
//                 }
//                 draw = !draw;
//                 metadata.add_timeout(Duration::from_millis(500), ());
//             })
//             .expect("Failed to insert event source!");
//     }
//     fn setup_output_handler(&self) {
//         let layer_shell = self
//             .env
//             .require_global::<zwlr_layer_shell_v1::ZwlrLayerShellV1>();

//         let env_handle = self.env.clone();
//         let context_handle = self.context.app_context.clone();
//         let renderer_context_handle = self.renderer_context.clone();
//         let surfaces_handle = self.surfaces.clone();
//         let output_handler = move |output: wl_output::WlOutput, info: &OutputInfo| {
//             if info.obsolete {
//                 // an output has been removed, release it
//                 surfaces_handle.borrow_mut().retain(|(i, _)| *i != info.id);
//                 output.release();
//             } else {
//                 // an output has been created, construct a surface for it
//                 let surface = env_handle.create_surface().detach();
//                 let pool = env_handle
//                     .create_auto_pool()
//                     .expect("Failed to create a memory pool!");
//                 (*surfaces_handle.borrow_mut()).push((
//                     info.id,
//                     Surface::new(
//                         &output,
//                         surface,
//                         &layer_shell.clone(),
//                         renderer_context_handle.clone(),
//                         context_handle.clone(),
//                         RefCell::new(pool),
//                     ),
//                 ));
//             }
//         };

//         // Process currently existing outputs
//         for output in self.env.get_all_outputs() {
//             if let Some(info) = with_output_info(&output, Clone::clone) {
//                 output_handler(output, &info);
//             }
//         }

//         // Setup a listener for changes
//         // The listener will live for as long as we keep this handle alive
//         let _listner_handle = self
//             .env
//             .listen_for_outputs(move |output, info, _| output_handler(output, info));
//     }
//     fn setup_keyboard_handler(&self) {
//         let mut seats = Vec::<(
//             String,
//             Option<(wl_keyboard::WlKeyboard, calloop::RegistrationToken)>,
//         )>::new();

//         let handle = self.context.handle.clone();

//         // first process already existing seats
//         for seat in self.env.get_all_seats() {
//             if let Some((has_kbd, name)) = with_seat_data(&seat, |seat_data| {
//                 (
//                     seat_data.has_keyboard && !seat_data.defunct,
//                     seat_data.name.clone(),
//                 )
//             }) {
//                 debug!("{:?} - {} - {}", seat, has_kbd, name);
//                 if has_kbd {
//                     let seat_name = name.clone();
//                     match map_keyboard_repeat(
//                         handle.clone(),
//                         &seat,
//                         None,
//                         RepeatKind::System,
//                         move |event, _, mut dispatch_data| {
//                             let loop_context = dispatch_data
//                                 .get::<LoopContext>()
//                                 .expect("To get our Loop Context");
//                             let app_context = &loop_context.app_context;

//                             if Self::handle_keyboard_event(event, &seat_name, app_context) {
//                                 // apply filter
//                                 app_context.borrow_mut().filter();

//                                 loop_context.action.set(Some(LoopAction::Redraw));
//                             }
//                         },
//                     ) {
//                         Ok((kbd, repeat_source)) => {
//                             seats.push((name, Some((kbd, repeat_source))));
//                         }
//                         Err(e) => {
//                             eprintln!("Failed to map keyboard on seat {} : {:?}.", name, e);
//                             seats.push((name, None));
//                         }
//                     }
//                 } else {
//                     seats.push((name, None));
//                 }
//             }
//         }

//         // then setup a listener for changes
//         let _seat_listener = self.env.listen_for_seats(move |seat, seat_data, _| {
//             // find the seat in the vec of seats, or insert it if it is unknown
//             let idx = seats.iter().position(|(name, _)| name == &seat_data.name);
//             let idx = idx.unwrap_or_else(|| {
//                 seats.push((seat_data.name.clone(), None));
//                 seats.len() - 1
//             });

//             let handle = handle.clone();
//             let (_, ref mut opt_kbd) = &mut seats[idx];
//             // we should map a keyboard if the seat has the capability & is not defunct
//             if seat_data.has_keyboard && !seat_data.defunct {
//                 if opt_kbd.is_none() {
//                     // we should initalize a keyboard
//                     let seat_name = seat_data.name.clone();
//                     match map_keyboard_repeat(
//                         handle,
//                         &seat,
//                         None,
//                         RepeatKind::System,
//                         move |event, _, mut dispatch_data| {
//                             let loop_context = dispatch_data
//                                 .get::<LoopContext>()
//                                 .expect("To get our Loop Context");
//                             let context = &loop_context.app_context;

//                             if Self::handle_keyboard_event(event, &seat_name, context) {
//                                 // apply filter
//                                 context.borrow_mut().filter();

//                                 loop_context.action.set(Some(LoopAction::Redraw));
//                             }
//                         },
//                     ) {
//                         Ok((kbd, repeat_source)) => {
//                             *opt_kbd = Some((kbd, repeat_source));
//                         }
//                         Err(e) => {
//                             eprintln!(
//                                 "Failed to map keyboard on seat {} : {:?}.",
//                                 seat_data.name, e
//                             )
//                         }
//                     }
//                 }
//             } else if let Some((kbd, source)) = opt_kbd.take() {
//                 // the keyboard has been removed, cleanup
//                 kbd.release();
//                 handle.remove(source);
//             }
//         });
//     }
//     fn handle_keyboard_event(
//         event: KbEvent,
//         seat_name: &str,
//         context: &Rc<RefCell<AppContext>>,
//     ) -> bool {
//         match event {
//             KbEvent::Enter { keysyms, .. } => {
//                 debug!(
//                     "Gained focus on seat '{}' while {} keys pressed.",
//                     seat_name,
//                     keysyms.len(),
//                 );
//                 false
//             }
//             KbEvent::Leave { .. } => {
//                 debug!("Lost focus on seat '{}'.", seat_name);
//                 false
//             }
//             KbEvent::Key {
//                 keysym,
//                 state,
//                 utf8,
//                 rawkey,
//                 ..
//             } => {
//                 debug!("Key {:?}: {:x} on seat '{}'.", state, keysym, seat_name);
//                 if state == KeyState::Pressed {
//                     Self::handle_key(&context, rawkey, utf8)
//                 } else {
//                     false
//                 }
//             }
//             KbEvent::Modifiers { modifiers } => {
//                 debug!(
//                     "Modifiers changed to {:?} on seat '{}'.",
//                     modifiers, seat_name
//                 );
//                 context.borrow_mut().modifiers = modifiers;
//                 false
//             }
//             KbEvent::Repeat {
//                 keysym,
//                 rawkey,
//                 utf8,
//                 ..
//             } => {
//                 debug!("Key repetition {:x} on seat '{}'.", keysym, seat_name);
//                 Self::handle_key(&context, rawkey, utf8)
//             }
//         }
//     }
//     fn handle_key(context: &Rc<RefCell<AppContext>>, rawkey: u32, utf8: Option<String>) -> bool {
//         match rawkey {
//             /* ESC */
//             1 => {
//                 // exit on ESC pressed
//                 std::process::exit(0);
//             }
//             /* BACKSPACE */
//             14 => {
//                 // pop one char if backspace
//                 (*context.borrow_mut().input).pop();
//                 true
//             }
//             /* TAB */
//             15 => {
//                 let mut context = context.borrow_mut();
//                 if context.modifiers.shift {
//                     if context.current_index > 0 {
//                         // shift index left
//                         context.current_index -= 1;
//                     }
//                 } else {
//                     if context.current_index < context.list.filtered_len() {
//                         // shift index right
//                         context.current_index += 1;
//                     }
//                 }
//                 true
//             }
//             /* ENTER */
//             28 => {
//                 // execute !!
//                 let (exit, binary) = if let Some(target) = context.borrow().target() {
//                     info!("Execute {}", target);

//                     // launch
//                     (command::launch(target), Some(target.binary()))
//                 } else {
//                     // exit with failure (no target)
//                     (10, None)
//                 };

//                 // write history
//                 if let Some(binary) = binary {
//                     if let Err(e) = context
//                         .borrow_mut()
//                         .app_config
//                         .increment_and_store_history(binary)
//                     {
//                         warn!("Failed to store history data - {}", e);
//                     }
//                 }

//                 std::process::exit(exit);
//             }
//             _ => match utf8 {
//                 Some(txt) => {
//                     debug!(" -> Received text \"{}\".", txt);

//                     let mut context = context.borrow_mut();

//                     // append key to filter
//                     (*context.input).push_str(txt.as_str());

//                     // reset current index
//                     context.current_index = 0;

//                     true
//                 }
//                 _ => {
//                     debug!("Not handled KEY {:?}", rawkey);
//                     false
//                 }
//             },
//         }
//     }
//     pub fn handle_events(&self, redraw: bool) {
//         let mut surfaces = self.surfaces.borrow_mut();
//         let mut i = 0;
//         while i != surfaces.len() {
//             if surfaces[i].1.handle_events() {
//                 surfaces.remove(i);
//             } else {
//                 if redraw {
//                     surfaces[i].1.redraw();
//                 }

//                 i += 1;
//             }
//         }
//     }
// }

// fn draw_text(
//     dt: &mut DrawTarget,
//     font: &Font,
//     point_size: f32,
//     text: &str,
//     start: Point,
//     src: &Source,
//     options: &DrawOptions,
//     space_factor: f32,
// ) -> f32 {
//     let mut start = pathfinder_geometry::vector::vec2f(start.x, start.y);
//     let mut ids = Vec::new();
//     let mut positions = Vec::new();
//     for c in text.chars() {
//         let id = font.glyph_for_char(c).unwrap();
//         ids.push(id);
//         positions.push(Point::new(start.x(), start.y()));
//         start += font.advance(id).unwrap() * point_size / 24. / 96. * space_factor;
//     }
//     dt.draw_glyphs(font, point_size, &ids, &positions, src, options);
//     start.x()
// }
