use std::path::PathBuf;

use clap::{Args, Parser, Subcommand};
use url::Url;

#[derive(Parser)]
#[command(version, about)]
pub struct ProgramArgs {
    #[command(subcommand)]
    pub command: Command,

    #[arg(long, value_enum, default_value = "warn")]
    /// Verbosity of logging output.
    pub log_level: tracing::level_filters::LevelFilter,

    #[arg(long)]
    /// Write logging output to a file.
    pub log_file: Option<PathBuf>,

    #[arg(long)]
    /// Send logging output to Systemd's Journal service.
    pub log_journald: bool,
}

#[derive(Subcommand)]
pub enum Command {
    /// Download a file.
    Fetch(FetchArgs),

    /// Look up IP addresses for a domain name.
    Lookup(LookupArgs),
}

#[derive(Args)]
pub struct FetchArgs {
    /// URL of file to download.
    pub url: Url,

    /// Save downloaded file to given path.
    #[arg(short, long)]
    pub output: Option<PathBuf>,

    /// Save protocol download data to given path.
    #[arg(short = 'r', long)]
    pub output_response: Option<PathBuf>,

    /// Save protocol upload data to given path.
    #[arg(short = 'q', long)]
    pub output_request: Option<PathBuf>,
}

#[derive(Args)]
pub struct LookupArgs {
    /// Domain name of the host.
    pub name: String,

    /// Output in JSON format.
    #[arg(short, long)]
    pub json: bool,
}
