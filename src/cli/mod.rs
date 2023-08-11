mod args;
mod fetch;
mod logging;
mod lookup;

use clap::Parser;

use self::args::{Command, ProgramArgs};

pub fn run() -> anyhow::Result<()> {
    curl::init();

    let args = ProgramArgs::parse();

    logging::set_up_logging(&args)?;

    match args.command {
        Command::Fetch(fetch_args) => fetch::run(&fetch_args),
        Command::Lookup(lookup_args) => lookup::run(&lookup_args),
    }
}
