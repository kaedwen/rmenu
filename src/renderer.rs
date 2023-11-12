use std::sync::Arc;

use font_kit::{font::Font, source::SystemSource};

use log::debug;
use raqote::{DrawOptions, DrawTarget, Point, SolidSource, Source};

use crate::{app::AppContext, config::{Color, StaticConfig}};

static DEFAULT_FONT: &'static [u8; 42756] = include_bytes!("../assets/ShareTechMono-Regular.ttf");

static DEFAULT_FONT_SIZE: f32 = 24.;
static DEFAULT_FONT_SPACING: f32 = 2.;

static DEFAULT_HIGHLIGHT: Color = Color {
  r: 0xFF,
  g: 0x00,
  b: 0x00,
  a: 0xFF,
};

static DEFAULT_FOREGROUND: Color = Color {
  r: 0xD0,
  g: 0xD0,
  b: 0xD0,
  a: 0xFF,
};

static DEFAULT_BACKGROUND: Color = Color {
  r: 0x10,
  g: 0x10,
  b: 0x10,
  a: 0xEE,
};

struct RendererContext {
  highlight: SolidSource,
  foreground: SolidSource,
  background: SolidSource,
  font_spacing: f32,
  font_size: f32,
  font: Font,
}

pub struct Renderer {
  context: RendererContext,
  cursor: Option<f32>,
}

impl Into<SolidSource> for Color {
  fn into(self) -> SolidSource {
    SolidSource {
      r: self.r,
      g: self.g,
      b: self.b,
      a: self.a,
    }
  }
}

impl Renderer {
  pub fn new(config: &StaticConfig) -> Renderer {
    let default_font = font_kit::font::Font::from_bytes(
      Arc::new(DEFAULT_FONT.to_vec()), 
      0
    ).ok();

    let font = config
      .font
      .as_ref()
      .and_then(|font| {
        let o1 = font.path.as_ref().and_then(|path| {
          std::fs::read(path)
            .ok()
            .and_then(|data| font_kit::font::Font::from_bytes(Arc::new(data), 0).ok())
        });

        let o2 = font.name.as_ref().and_then(|name| {
          debug!("{}", name);
          SystemSource::new()
            .select_by_postscript_name(&name)
            .ok()
            .and_then(|h| h.load().ok())
        });

        debug!("FONT path {:?}, name {:?}", o1, o2);

        o1.or(o2)
      }).or(default_font).expect("To load FreeMono TTF");

    Self {
      context: RendererContext {
        font,
        highlight: config
          .style
          .as_ref()
          .and_then(|style| style.highlight_color)
          .unwrap_or(DEFAULT_HIGHLIGHT)
          .into(),
        foreground: config
          .style
          .as_ref()
          .and_then(|style| style.foreground_color)
          .unwrap_or(DEFAULT_FOREGROUND)
          .into(),
        background: config
          .style
          .as_ref()
          .and_then(|style| style.background_color)
          .unwrap_or(DEFAULT_BACKGROUND)
          .into(),
        font_spacing: config
          .font
          .as_ref()
          .and_then(|f| f.spacing)
          .unwrap_or(DEFAULT_FONT_SPACING),
        font_size: config
          .font
          .as_ref()
          .and_then(|f| f.size)
          .unwrap_or(DEFAULT_FONT_SIZE),
      },
      cursor: None,
    }
  }
  pub fn render(&mut self, app_context: &AppContext, width: i32, height: i32, canvas: &mut [u8]) {
    let mut dt = DrawTarget::new(width as i32, height as i32);

    let current_index = app_context.current_index;

    let options = DrawOptions::new();
    let point_size = self.context.font_size;
    let highlight_brush = Source::Solid(self.context.highlight);
    let foreground_brush = Source::Solid(self.context.foreground);
    let background_brush = Source::Solid(self.context.background);

    let filter = &app_context.input;
    let filter_text = format!("> {}", filter);

    dt.fill_rect(
      0.,
      0.,
      width as f32,
      height as f32,
      &background_brush,
      &options,
    );

    let offset = draw_text(
      &mut dt,
      &self.context.font,
      point_size,
      filter_text.as_str(),
      Point::new(0., height as f32 * 3. / 5.),
      &foreground_brush,
      &options,
      self.context.font_spacing,
    );

    self.cursor = Some(offset);

    let mut start_list = offset.max(200.);

    // a little space just to be sure
    start_list += 20.;

    if current_index > 0 {
      start_list = draw_text(
        &mut dt,
        &self.context.font,
        point_size,
        "<",
        Point::new(start_list, height as f32 * 3. / 5.),
        &foreground_brush,
        &options,
        self.context.font_spacing,
      ) + 15.;
    }

    for (index, name) in app_context
      .list
      .filtered
      .iter()
      .map(|c| &c.name)
      .skip(current_index)
      .enumerate()
    {
      start_list = draw_text(
        &mut dt,
        &self.context.font,
        point_size,
        name,
        Point::new(start_list, height as f32 * 3. / 5.),
        if index == 0 {
          &highlight_brush
        } else {
          &foreground_brush
        },
        &options,
        self.context.font_spacing,
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
