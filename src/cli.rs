use std::path::PathBuf;

use clap::Parser;

#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
pub struct Args {
    #[clap(short, long)]
    pub config: Option<PathBuf>,
}

pub fn parse() -> Args {
    let mut args = Args::parse();

    if args.config.is_none() {
      args.config = dirs::home_dir();
    }

    args
}
