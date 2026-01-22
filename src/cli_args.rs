use std::path::PathBuf;
use clap::Parser;

#[derive(Parser)]
#[command(version, about, long_about = None)]
pub struct CliArgs {
    pub input_file: Option<PathBuf>,
    #[arg(short='v', long="validate")]
    pub validate: bool
}
