use std::{
    fmt::Display,
    io::Result,
    path::{Path, PathBuf},
    process::Stdio,
};

use log::{debug, error, info};
use std::os::unix::fs::PermissionsExt;

#[derive(Clone)]
pub struct Command {
    pub path: PathBuf,
}

pub struct CommandList {
    initial: Vec<Command>,
    pub filtered: Vec<Command>,
}

impl CommandList {
    pub fn new() -> std::io::Result<Self> {
        let initial = gather_commands()?;
        let filtered = initial.clone();

        Ok(Self { initial, filtered })
    }
    pub fn filter(&mut self, filter: &String) {
        self.filtered = self
            .initial
            .iter()
            .filter_map(|command| {
                command
                    .path
                    .file_name()
                    .and_then(|s| s.to_str())
                    .filter(|name| name.to_lowercase().starts_with(filter))
                    .map(|_| command.clone())
            })
            .collect();
    }
}

impl Display for CommandList {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "List has {} filtered entries and {} in total",
            self.filtered.len(),
            self.initial.len()
        )
    }
}

impl Display for Command {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.path.display())
    }
}

pub fn gather_commands() -> std::io::Result<Vec<Command>> {
    let mut list = Vec::<Command>::new();

    if let Ok(path) = std::env::var("PATH") {
        for p in path.split(":") {
            let target = Path::new(p);
            if target.is_dir() {
                debug!("Inspect PATH {}", p);
                list.extend(
                    std::fs::read_dir(target)?
                        .filter_map(Result::ok)
                        .filter(|i| {
                            i.metadata()
                                .map_or_else(|_| false, |m| m.permissions().mode() & 0o111 != 0)
                        })
                        .map(|i| Command { path: i.path() }),
                );
            }
        }
    }

    Ok(list)
}

pub fn launch(command: &Command) -> i32 {
    match std::process::Command::new(&command.path)
        .stdout(Stdio::null())
        .stdin(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
    {
        Ok(_) => {
            info!("Successfully spawned {}", command);
            0
        }
        Err(e) => {
            error!("Failed to spawn {} - {}", command, e);
            1
        }
    }
}
