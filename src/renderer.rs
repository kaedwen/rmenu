pub struct Surface {
    surface: wl_surface::WlSurface,
    layer_surface: Main<zwlr_layer_surface_v1::ZwlrLayerSurfaceV1>,
    next_render_event: Rc<Cell<Option<RenderEvent>>>,
    dimensions: Option<(i32, i32)>,
    context: Rc<RefCell<Context>>,
    pool: RefCell<AutoMemPool>,
    font: Font,
}

pub struct Renderer {}

impl Surface {
    fn new(
        output: &wl_output::WlOutput,
        surface: wl_surface::WlSurface,
        layer_shell: &Attached<zwlr_layer_shell_v1::ZwlrLayerShellV1>,
        context: Rc<RefCell<Context>>,
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
    fn handle_events(&mut self) -> bool {
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

    fn redraw(&self) {
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
    pub fn new() {
        let surfaces = Rc::new(RefCell::new(Vec::<(u32, Surface)>::new()));

        let layer_shell = env.require_global::<zwlr_layer_shell_v1::ZwlrLayerShellV1>();

        let env_handle = env.clone();
        let context_handle = Rc::clone(&context);
        let surfaces_handle = Rc::clone(&surfaces);
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
    }
}
