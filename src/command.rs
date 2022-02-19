use std::{
    fmt::Display,
    io::Result,
    path::{Path, PathBuf},
    process::Stdio,
};

use crate::cli;
use log::{debug, error, info};
use std::os::unix::fs::PermissionsExt;

#[derive(Clone)]
pub struct Command {
    pub path: PathBuf,
    pub name: String,
    weight: u32,
}

pub struct CommandList {
    initial: Vec<PathBuf>,
    pub filtered: Vec<Command>,
}

impl CommandList {
    pub fn new(app_config: &cli::AppConfig) -> std::io::Result<Self> {
        let initial = gather_commands(&app_config.config)?;
        let filtered = Self::filter_data(None::<&String>, &initial, &app_config.history);

        Ok(Self { initial, filtered })
    }
    pub fn filter(&mut self, filter: &String, history: &cli::History) {
        self.filtered = Self::filter_data(Some(filter), &self.initial, history)
    }
    pub fn filtered_len(&self) -> usize {
        self.filtered.len()
    }
    fn filename(path: &Path) -> Option<String> {
        path.file_name()
            .and_then(|s| s.to_str())
            .map(|s| String::from(s))
    }
    fn filter_data(
        filter: Option<&String>,
        data: &Vec<PathBuf>,
        history: &cli::History,
    ) -> Vec<Command> {
        let mut list = data
            .iter()
            .filter_map(|path| {
                Self::filename(path).and_then(|name| {
                    if let Some(filter) = filter {
                        Some(name)
                            .filter(|name| name.to_lowercase().starts_with(filter))
                            .map(|name| Command {
                                path: path.clone(),
                                weight: history.get_weight(&name),
                                name,
                            })
                    } else {
                        Some(Command {
                            path: path.clone(),
                            weight: history.get_weight(&name),
                            name,
                        })
                    }
                })
            })
            .collect::<Vec<Command>>();

        // sort the weights
        list.sort_by(|a, b| b.weight.cmp(&a.weight));

        list
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

impl Command {
    pub fn binary(&self) -> String {
        String::from(
            self.path
                .file_name()
                .and_then(|n| n.to_str())
                .expect("Binary string to be converted"),
        )
    }
}

fn gather_commands(config: &cli::Config) -> std::io::Result<Vec<PathBuf>> {
    let mut list = Vec::<PathBuf>::new();

    if let Ok(path) = std::env::var("PATH") {
        for p in path.split(":") {
            let target = Path::new(p);
            if target.is_dir() {
                debug!("Inspect PATH {}", p);
                list.extend(
                    std::fs::read_dir(target)?
                        .filter_map(Result::ok)
                        .filter(|i| {
                            // filter out not executable
                            i.metadata()
                                .map_or_else(|_| false, |m| m.permissions().mode() & 0o111 != 0)
                        })
                        .filter_map(|i| {
                            i.file_name().into_string().ok().and_then(|name| {
                                // filter out whitelisted binaries
                                let w = if let Some(list) = config.whitelist.as_ref() {
                                    if list.contains(&name) {
                                        // whitelist contains -> allow
                                        Some(i.path())
                                    } else {
                                        // whitelist does not contain -> hide
                                        None
                                    }
                                } else {
                                    // no whitelist given -> allow
                                    Some(i.path())
                                };

                                debug!("WHITE {} - {:?}", name, w);

                                // filter out blacklisted binaries
                                let b = if let Some(list) = config.blacklist.as_ref() {
                                    if list.contains(&name) {
                                        // blacklist contains -> hide
                                        None
                                    } else {
                                        // blacklist does not contain -> allow
                                        Some(i.path())
                                    }
                                } else {
                                    // no backlist given -> allow
                                    Some(i.path())
                                };

                                debug!("BLACK {} - {:?}", name, b);

                                w.and(b)
                            })
                        }),
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
