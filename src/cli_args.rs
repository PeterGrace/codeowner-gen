use clap::Parser;
use std::path::PathBuf;

#[derive(Parser)]
#[command(version, about, long_about = None)]
pub struct CliArgs {
    pub input_file: Option<PathBuf>,
    #[arg(short = 'o', long = "output-file", default_value = "CODEOWNERS")]
    pub output_file: PathBuf,
}
