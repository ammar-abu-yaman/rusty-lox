use clap::Parser;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
pub struct Cli {
    pub filename: Option<String>,

    #[arg(short, long, default_value_t = false)]
    pub debug: bool,
}
