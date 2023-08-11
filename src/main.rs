mod cli;
mod client;
mod dns;
mod error;
mod http;
mod string;
mod version;

use std::process::ExitCode;

fn main() -> ExitCode {
    let default_hook = std::panic::take_hook();

    std::panic::set_hook(Box::new(move |panic_info| {
        tracing::error!(%panic_info, "panic");
        default_hook(panic_info);
    }));

    match cli::run() {
        Ok(_) => ExitCode::SUCCESS,
        Err(error) => {
            tracing::error!(?error, "main exit failure");
            eprintln!("{}", error);
            ExitCode::FAILURE
        }
    }
}
