use std::{fs::File, io::Write};

use crate::client::{Client, Config, Request, SessionControl, SessionEvent, SessionHandler};

use super::args::FetchArgs;

pub fn run(args: &FetchArgs) -> anyhow::Result<()> {
    let mut config = Config::new();
    config.set_http_compression(true);

    let client = Client::new(config);
    let request = Request::new(args.url.clone());

    let output_file = match &args.output {
        Some(path) => Some(File::create(path)?),
        None => None,
    };

    let response_file = match &args.output_response {
        Some(path) => Some(File::create(path)?),
        None => None,
    };

    let request_file = match &args.output_request {
        Some(path) => Some(File::create(path)?),
        None => None,
    };

    let handler = FetchHandler::new(output_file, response_file, request_file);
    let (_handler, result) = client.submit(request, handler);
    result?;

    Ok(())
}

struct FetchHandler {
    output: Option<File>,
    response: Option<File>,
    request: Option<File>,
}

impl FetchHandler {
    fn new(output: Option<File>, response: Option<File>, request: Option<File>) -> Self {
        Self {
            output,
            response,
            request,
        }
    }
}

impl SessionHandler for FetchHandler {
    fn upload_content(
        &mut self,
        _control: &mut dyn SessionControl,
        _buf: &mut [u8],
    ) -> Result<usize, Box<dyn std::error::Error + Send + Sync>> {
        Ok(0)
    }

    fn event(
        &mut self,
        _control: &mut dyn SessionControl,
        event: SessionEvent,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        match event {
            SessionEvent::HeaderReceived(data) | SessionEvent::BodyReceived(data) => {
                match &mut self.response {
                    Some(file) => file.write_all(data)?,
                    None => {},
                }
            }
            SessionEvent::HeaderSent(data) | SessionEvent::BodySent(data) => {
                match &mut self.request {
                    Some(file) => file.write_all(data)?,
                    None => {},
                }
            }

            SessionEvent::ContentReceived(data) => match &mut self.output {
                Some(file) => file.write_all(data)?,
                None => std::io::stdout().write_all(data)?,
            },

            _ => {}
        }

        Ok(())
    }
}
