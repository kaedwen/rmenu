use anyhow::{bail, Result};
use clap::Parser;
use core::fmt;
use log::info;
use serde::{de::Visitor, Deserialize, Deserializer};
use std::path::PathBuf;

#[derive(Debug, Clone, Copy, Default)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

#[derive(Debug, Deserialize)]
pub struct Style {
    pub foreground_color: Option<Color>,
    pub background_color: Option<Color>,
}

#[derive(Debug, Deserialize)]
pub struct Config {
    pub blacklist: Vec<String>,
    pub style: Style,
}

#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
pub struct Args {
    #[clap(short, long)]
    pub config: Option<PathBuf>,
}

pub fn parse() -> Result<Config> {
    let mut args = Args::parse();

    if args.config.is_none() {
        args.config = dirs::home_dir().map(|h| h.join(".config/rmenu/config.yaml"))
    }

    if let Some(path) = args.config {
        info!("Reading config from {}", &path.display());
        Ok(serde_yaml::from_reader(&std::fs::File::open(path)?)?)
    } else {
        bail!("No config found!")
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
