use anyhow::Result;
use clap::Parser;
use core::fmt;
use log::{debug, info};
use serde::{de::Visitor, Deserialize, Deserializer};
use std::collections::HashMap;
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, Default)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

#[derive(Debug, Default, Deserialize)]
pub struct Style {
    pub highlight_color: Option<Color>,
    pub foreground_color: Option<Color>,
    pub background_color: Option<Color>,
    pub height: u32,
}

#[derive(Debug, Default, Deserialize)]
pub struct Font {
    pub path: Option<PathBuf>,
    pub name: Option<String>,
    pub spacing: Option<f32>,
    pub size: Option<f32>,
}

#[derive(Debug, Default, Deserialize)]
pub struct Config {
    pub blacklist: Option<Vec<String>>,
    pub whitelist: Option<Vec<String>>,
    pub style: Option<Style>,
    pub font: Option<Font>,
}

#[derive(Debug, Default)]
pub struct History(HashMap<String, u32>);

#[derive(Debug)]
pub struct AppConfig {
    pub config: Config,
    pub history: History,
    args: Args,
}

#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
pub struct Args {
    #[clap(short, long)]
    pub config: Option<PathBuf>,
    #[clap(short, long)]
    pub history: Option<PathBuf>,
}

pub fn parse() -> Result<AppConfig> {
    let mut args = Args::parse();

    if args.config.is_none() {
        args.config = dirs::home_dir().map(|h| h.join(".config/rmenu/config.yaml"))
    }

    let config = if let Some(path) = &args.config {
        info!("Reading config from {}", &path.display());
        serde_yaml::from_reader(&std::fs::File::open(path)?)?
    } else {
        Default::default()
    };

    if args.history.is_none() {
        args.history = dirs::home_dir().map(|h| h.join(".config/rmenu/history"))
    }

    let history = if let Some(path) = &args.history {
        History::from_path(path.as_path())
    } else {
        Default::default()
    };

    Ok(AppConfig {
        args,
        history,
        config,
    })
}

impl AppConfig {
    pub fn increment_and_store_history(&mut self, binary: String) -> std::io::Result<()> {
        (*self.history.0.entry(binary).or_insert(0)) += 1;
        self.store_history()
    }
    fn store_history(&self) -> std::io::Result<()> {
        if let Some(path) = &self.args.history {
            self.history.to_path(&path)
        } else {
            Ok(())
        }
    }
}

impl History {
    pub fn from_path(path: &Path) -> Self {
        if let Ok(file) = std::fs::File::open(path) {
            let re = regex::Regex::new(r"^(?P<binary>.*)\|(?P<weight>\d+)$").unwrap();
            let x = BufReader::new(file)
                .lines()
                .filter_map(Result::ok)
                .filter_map(|line| {
                    debug!("Line {}", line);
                    re.captures(&line).and_then(|groups| {
                        if let (Some(path), Some(weight)) =
                            (groups.name("binary"), groups.name("weight"))
                        {
                            Some((
                                String::from(path.as_str()),
                                weight.as_str().parse::<u32>().unwrap_or(0),
                            ))
                        } else {
                            None
                        }
                    })
                })
                .collect();
            Self(x)
        } else {
            Self(Default::default())
        }
    }
    pub fn get_weight(&self, name: &String) -> u32 {
        *self.0.get(name).unwrap_or(&0)
    }
    pub fn to_path(&self, path: &Path) -> std::io::Result<()> {
        let mut file = std::fs::File::create(path)?;
        for (binary, weight) in &self.0 {
            file.write(format!("{}|{}\n", binary, weight).as_bytes())?;
        }

        Ok(())
    }
}

impl<'de> Deserialize<'de> for Color {
    fn deserialize<D>(deserializer: D) -> Result<Color, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct ColorVisitor;

        fn parse<E>(value: &str) -> Result<Color, E>
        where
            E: serde::de::Error,
        {
            let mut color: Color = Default::default();
            let re = regex::Regex::new(
                r"^#(?P<r>[0-9a-f]{2})(?P<g>[0-9a-f]{2})(?P<b>[0-9a-f]{2})(?P<a>[0-9a-f]{2})$",
            )
            .unwrap();
            if let Some(groups) = re.captures(&value.to_ascii_lowercase()) {
                if let Some(r) = groups.name("r") {
                    if let Ok(r) = u8::from_str_radix(r.as_str(), 16) {
                        color.r = r;
                    }
                }
                if let Some(g) = groups.name("g") {
                    if let Ok(g) = u8::from_str_radix(g.as_str(), 16) {
                        color.g = g;
                    }
                }
                if let Some(b) = groups.name("b") {
                    if let Ok(b) = u8::from_str_radix(b.as_str(), 16) {
                        color.b = b;
                    }
                }
                if let Some(a) = groups.name("a") {
                    if let Ok(a) = u8::from_str_radix(a.as_str(), 16) {
                        color.a = a;
                    }
                }
            }

            Ok(color)
        }

        impl<'de> Visitor<'de> for ColorVisitor {
            type Value = Color;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("struct PhoneNumber")
            }

            fn visit_str<E>(self, value: &str) -> Result<Color, E>
            where
                E: serde::de::Error,
            {
                parse(value)
            }

            fn visit_string<E>(self, value: String) -> Result<Color, E>
            where
                E: serde::de::Error,
            {
                parse(&value)
            }
        }

        deserializer.deserialize_string(ColorVisitor)
    }
}
