use std::{fs::File, sync::Mutex};

use reopen::Reopen;
use tracing::metadata::LevelFilter;
use tracing_subscriber::{filter::Targets, prelude::*};

use super::args::ProgramArgs;

pub fn set_up_logging(args: &ProgramArgs) -> anyhow::Result<()> {
    let subscriber = tracing_subscriber::Registry::default();
    let target_str = if args.log_level == LevelFilter::TRACE {
        ""
    } else {
        "wrecv"
    };
    let filter = Targets::new().with_target(target_str, args.log_level);
    let subscriber = subscriber.with(filter);

    let layer = if args.log_file.is_none() && !args.log_journald {
        let layer = tracing_subscriber::fmt::layer();
        Some(layer)
    } else {
        None
    };
    let subscriber = subscriber.with(layer);

    let layer = if let Some(path) = &args.log_file {
        let path = path.to_owned();
        let file = Reopen::new(Box::new(move || {
            let path = path.clone();
            File::options()
                .create(true)
                .write(true)
                .append(true)
                .open(path)
        }))?;
        file.handle().register_signal(signal_hook::consts::SIGHUP)?;
        let file = Mutex::new(file);

        let layer = tracing_subscriber::fmt::layer().json().with_writer(file);
        Some(layer)
    } else {
        None
    };
    let subscriber = subscriber.with(layer);

    let layer = if args.log_journald {
        Some(tracing_journald::layer()?)
    } else {
        None
    };
    let subscriber = subscriber.with(layer);

    tracing::subscriber::set_global_default(subscriber)?;

    Ok(())
}
