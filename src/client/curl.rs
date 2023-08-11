use std::{
    cell::RefCell,
    fmt::Debug,
    net::{IpAddr, SocketAddr},
    rc::Rc,
    str::FromStr,
    sync::OnceLock, time::Duration,
};

use curl::easy::{Easy, InfoType, Transfer};
use regex::Regex;

use crate::{
    error::{BoxedError, Error, OtherError},
    http::{FieldName, FieldValue, RequestHeader, ResponseHeader, ResponseTrailer},
};

use super::{
    cookie::CookieJar, pool::ConnectionPool, Config, Request, Session, SessionControl,
    SessionEvent, SessionHandler,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SessionMode {
    Http,
    Ftp,
}

pub struct CurlSession<H: SessionHandler> {
    config: Rc<RefCell<Config>>,
    request: Request,
    handler: Option<H>,
    mode: SessionMode,
    connection_pool: ConnectionPool,
    cookie_jar: CookieJar,
    curl_handle: Option<Easy>,
}

impl<H: SessionHandler> CurlSession<H> {
    pub fn new_http(
        config: Rc<RefCell<Config>>,
        request: Request,
        handler: H,
        connection_pool: ConnectionPool,
        cookie_jar: CookieJar,
    ) -> Self {
        Self::new(
            config,
            request,
            handler,
            connection_pool,
            cookie_jar,
            SessionMode::Http,
        )
    }

    pub fn new_ftp(
        config: Rc<RefCell<Config>>,
        request: Request,
        handler: H,
        connection_pool: ConnectionPool,
        cookie_jar: CookieJar,
    ) -> Self {
        Self::new(
            config,
            request,
            handler,
            connection_pool,
            cookie_jar,
            SessionMode::Ftp,
        )
    }

    fn new(
        config: Rc<RefCell<Config>>,
        request: Request,
        handler: H,
        connection_pool: ConnectionPool,
        cookie_jar: CookieJar,
        mode: SessionMode,
    ) -> Self {
        let curl_handle = connection_pool.get_curl_handle();

        Self {
            config,
            request,
            handler: Some(handler),
            mode,
            connection_pool,
            cookie_jar,
            curl_handle: Some(curl_handle),
        }
    }

    fn run(&mut self) -> Result<(), Error> {
        self.set_up()?;
        self.perform_with_callbacks()?;
        self.connection_pool
            .put_curl_handle(self.curl_handle.take().unwrap());
        Ok(())
    }

    fn set_up(&mut self) -> Result<(), Error> {
        let curl_handle = self.curl_handle.as_mut().unwrap();

        {
            let config = self.config.borrow();
            let bind_address = config.bind_address().to_string();

            curl_handle.interface(&bind_address)?;
            curl_handle.verbose(true)?;
            curl_handle.url(self.request.url().as_str())?;
            curl_handle.ssl_verify_host(config.tls_verification())?;
            curl_handle.ssl_verify_peer(config.tls_verification())?;
            curl_handle.connect_timeout(Duration::from_secs(30))?;
        }

        if self.mode == SessionMode::Http {
            self.set_up_http_settings()?;
            self.set_up_http_cookies()?;
            self.set_up_http_headers()?;
        }

        Ok(())
    }

    fn set_up_http_settings(&mut self) -> Result<(), Error> {
        let config = self.config.borrow();
        let curl_handle = self.curl_handle.as_mut().unwrap();

        curl_handle.http_09_allowed(config.http_09())?;

        if !config.http_user_agent().is_empty() {
            curl_handle.useragent(config.http_user_agent())?;
        }

        if !config.http_compression() {
            curl_handle.accept_encoding("gzip")?;
        }

        Ok(())
    }

    fn set_up_http_cookies(&mut self) -> Result<(), Error> {
        let curl_handle = self.curl_handle.as_mut().unwrap();
        let cookie_value = self.cookie_jar.get_request_string(self.request.url());

        if !cookie_value.is_empty() {
            curl_handle.cookie(&cookie_value)?;
        }

        Ok(())
    }

    fn set_up_http_headers(&mut self) -> Result<(), Error> {
        let mut header_list = curl::easy::List::new();
        let config = self.config.borrow();
        let curl_handle = self.curl_handle.as_mut().unwrap();

        for (name, value) in config.http_headers() {
            if !self.request.http_headers().contains_key(name) {
                let field = format_header_field(name, value)?;
                header_list.append(&field)?;
            }
        }

        for (name, value) in self.request.http_headers() {
            let field = format_header_field(name, value)?;
            header_list.append(&field)?;
        }

        curl_handle.http_headers(header_list)?;

        Ok(())
    }

    fn perform_with_callbacks(&mut self) -> Result<(), Error> {
        let handler = self.handler.take().unwrap();

        let callback_handler = CallbackHandler::new(handler, self.mode);
        let callback_handler = Rc::new(RefCell::new(callback_handler));

        let result = {
            let curl_handle = self.curl_handle.as_mut().unwrap();
            let mut curl_session = curl_handle.transfer();

            Self::set_up_debug_function(&mut curl_session, callback_handler.clone())?;
            Self::set_up_header_function(&mut curl_session, callback_handler.clone())?;
            Self::set_up_progress_function(&mut curl_session, callback_handler.clone())?;
            Self::set_up_read_function(&mut curl_session, callback_handler.clone())?;
            Self::set_up_write_function(&mut curl_session, callback_handler.clone())?;

            curl_session.perform()
        };

        let callback_handler = Rc::into_inner(callback_handler).unwrap().into_inner();
        let handler = callback_handler.handler;
        let error = callback_handler.error;

        self.handler = Some(handler);

        result?;

        if let Some(error) = error {
            Err(Error::Other(OtherError::Custom(error)))
        } else {
            Ok(())
        }
    }

    fn set_up_debug_function<'a, C: SessionHandler + 'a>(
        curl_session: &mut Transfer<'_, 'a>,
        callback_handler: Rc<RefCell<CallbackHandler<C>>>,
    ) -> Result<(), Error> {
        curl_session.debug_function(move |info_type, data| {
            let mut callback_handler = (*callback_handler).borrow_mut();
            callback_handler.debug_function(info_type, data)
        })?;
        Ok(())
    }

    fn set_up_header_function<'a, C: SessionHandler + 'a>(
        curl_session: &mut Transfer<'_, 'a>,
        callback_handler: Rc<RefCell<CallbackHandler<C>>>,
    ) -> Result<(), Error> {
        curl_session.header_function(move |data| {
            let mut callback_handler = (*callback_handler).borrow_mut();
            callback_handler.header_function(data)
        })?;
        Ok(())
    }

    fn set_up_progress_function<'a, C: SessionHandler + 'a>(
        curl_session: &mut Transfer<'_, 'a>,
        callback_handler: Rc<RefCell<CallbackHandler<C>>>,
    ) -> Result<(), Error> {
        curl_session.progress_function(
            move |download_total, download_current, upload_total, upload_current| {
                let mut callback_handler = (*callback_handler).borrow_mut();
                callback_handler.progress_function(
                    download_total,
                    download_current,
                    upload_total,
                    upload_current,
                )
            },
        )?;
        Ok(())
    }

    fn set_up_read_function<'a, C: SessionHandler + 'a>(
        curl_session: &mut Transfer<'_, 'a>,
        callback_handler: Rc<RefCell<CallbackHandler<C>>>,
    ) -> Result<(), Error> {
        curl_session.read_function(move |buf| {
            let mut callback_handler = (*callback_handler).borrow_mut();
            callback_handler.read_function(buf)
        })?;
        Ok(())
    }

    fn set_up_write_function<'a, C: SessionHandler + 'a>(
        curl_session: &mut Transfer<'_, 'a>,
        callback_handler: Rc<RefCell<CallbackHandler<C>>>,
    ) -> Result<(), Error> {
        curl_session.write_function(move |data| {
            let mut callback_handler = (*callback_handler).borrow_mut();
            callback_handler.write_function(data)
        })?;
        Ok(())
    }
}

impl<H: SessionHandler> Session<H> for CurlSession<H> {
    fn wait(&mut self) -> (H, Result<(), Error>) {
        let result = self.run();
        let handler = self.handler.take().unwrap();

        (handler, result)
    }
}

#[derive(Debug)]
pub struct CurlSessionControl {
    aborted: bool,
}

impl CurlSessionControl {
    fn new() -> Self {
        Self { aborted: false }
    }
}

impl SessionControl for CurlSessionControl {
    fn abort(&mut self) {
        self.aborted = true;
    }
}

impl<H: SessionHandler> Debug for CurlSession<H> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CurlSession").field("...", &"...").finish()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CallbackState {
    HttpRequest,
    HttpResponse,
    HttpResponseTrailer,
    Finished,
    Ftp,
}

struct CallbackHandler<H: SessionHandler> {
    handler: H,
    control: CurlSessionControl,
    state: CallbackState,
    error: Option<BoxedError>,
    receive_buf: Vec<u8>,
    send_buf: Vec<u8>,
}

impl<H: SessionHandler> CallbackHandler<H> {
    fn new(handler: H, mode: SessionMode) -> Self {
        let state = match mode {
            SessionMode::Http => CallbackState::HttpRequest,
            SessionMode::Ftp => CallbackState::Ftp,
        };

        Self {
            handler,
            control: CurlSessionControl::new(),
            error: None,
            state,
            receive_buf: Vec::new(),
            send_buf: Vec::new(),
        }
    }

    fn debug_function(&mut self, info_type: InfoType, data: &[u8]) {
        tracing::trace!(?info_type, data = ?crate::string::preview_bytes(data, 100), "debug");

        match info_type {
            InfoType::Text => {
                let result = self.handle_curl_log(data);

                if let Err(error) = result {
                    self.error = Some(error);
                    self.control.abort();
                }
            }
            InfoType::HeaderIn => {}
            InfoType::HeaderOut => {
                let result = self.handle_send_header(data);

                if let Err(error) = result {
                    self.error = Some(error);
                    self.control.abort();
                }
            }

            InfoType::DataIn => {
                let result = self.handle_receive_body(data);

                if let Err(error) = result {
                    self.error = Some(error);
                    self.control.abort();
                }
            }
            InfoType::DataOut => {
                let result = self.handle_send_body(data);

                if let Err(error) = result {
                    self.error = Some(error);
                    self.control.abort();
                }
            }
            InfoType::SslDataIn => {}
            InfoType::SslDataOut => {}
            _ => {}
        }
    }

    fn header_function(&mut self, data: &[u8]) -> bool {
        tracing::trace!(data = ?String::from_utf8_lossy(data), "header");

        let result = self.handle_receive_header(data);

        if let Err(error) = result {
            self.error = Some(error);
            self.control.abort();
        }

        !self.control.aborted
    }

    fn progress_function(
        &mut self,
        download_total: f64,
        download_current: f64,
        upload_total: f64,
        upload_current: f64,
    ) -> bool {
        tracing::trace!(
            download_total,
            download_current,
            upload_total,
            upload_current,
            "progress"
        );

        let result = self.handle_progress(
            download_total as u64,
            download_current as u64,
            upload_total as u64,
            upload_current as u64,
        );

        if let Err(error) = result {
            self.error = Some(error);
            self.control.abort();
        }

        !self.control.aborted
    }

    fn read_function(&mut self, buf: &mut [u8]) -> Result<usize, curl::easy::ReadError> {
        tracing::trace!("read");

        let result = self.handle_send_content(buf);

        match result {
            Ok(size) => {
                if self.control.aborted {
                    Err(curl::easy::ReadError::Abort)
                } else {
                    Ok(size)
                }
            }
            Err(error) => {
                self.error = Some(error);
                Err(curl::easy::ReadError::Abort)
            }
        }
    }

    fn write_function(&mut self, data: &[u8]) -> Result<usize, curl::easy::WriteError> {
        tracing::trace!(data = ?crate::string::preview_bytes(data, 100), "write");

        let result = self.handle_receive_content(data);

        if let Err(error) = result {
            self.error = Some(error);
            self.control.abort();
        }

        if self.control.aborted {
            Ok(0)
        } else {
            Ok(data.len())
        }
    }

    fn handle_curl_log(&mut self, data: &[u8]) -> Result<(), BoxedError> {
        let text = String::from_utf8_lossy(data);
        let text = text.trim_end();
        tracing::debug!(text, "curl");

        self.find_and_emit_connect_event(text)?;

        Ok(())
    }

    fn find_and_emit_connect_event(&mut self, text: &str) -> Result<(), BoxedError> {
        // FIXME: Upstream curl crate needs CURLOPT_PREREQFUNCTION support
        if let Some(address) = parse_connect_address(text) {
            tracing::info!(address = %address.ip(), port = address.port(), "connected");
            let event = SessionEvent::Connected(address);
            self.handler.event(&mut self.control, event)?;
        }

        Ok(())
    }

    fn handle_send_header(&mut self, data: &[u8]) -> Result<(), BoxedError> {
        let event = SessionEvent::HeaderSent(data);

        self.handler.event(&mut self.control, event)?;

        if self.state == CallbackState::HttpRequest {
            self.send_buf.extend_from_slice(data);

            if let Some(_index) = crate::http::scan_header_boundary(&self.send_buf) {
                let header = RequestHeader::parse(&self.send_buf)?;
                tracing::info!(method = &header.method, uri = &header.uri, "http request");

                let event = SessionEvent::HttpRequest(data, header);
                self.handler.event(&mut self.control, event)?;

                self.state = CallbackState::HttpResponse;
            }
        }

        Ok(())
    }

    fn handle_receive_header(&mut self, data: &[u8]) -> Result<(), BoxedError> {
        let event = SessionEvent::HeaderReceived(data);
        self.handler.event(&mut self.control, event)?;

        if self.state == CallbackState::HttpResponse {
            self.receive_buf.extend_from_slice(data);

            if let Some(_index) = crate::http::scan_header_boundary(&self.receive_buf) {
                let header = ResponseHeader::parse(&self.receive_buf)?;
                tracing::info!(
                    status_code = header.status_code,
                    reason_phrase = &header.reason_phrase,
                    "http response"
                );

                let event = SessionEvent::HttpResponse(data, header);
                self.handler.event(&mut self.control, event)?;

                self.state = CallbackState::HttpResponseTrailer;
            }
        } else if self.state == CallbackState::HttpResponseTrailer {
            self.receive_buf.extend_from_slice(data);

            if let Some(_index) = crate::http::scan_header_boundary(&self.receive_buf) {
                let header = ResponseTrailer::parse(&self.receive_buf)?;
                let event = SessionEvent::HttpResponseTrailer(data, header);

                self.handler.event(&mut self.control, event)?;

                self.state = CallbackState::Finished;
            }
        }

        Ok(())
    }

    fn handle_send_body(&mut self, data: &[u8]) -> Result<(), BoxedError> {
        let event = SessionEvent::BodySent(data);
        self.handler.event(&mut self.control, event)?;
        Ok(())
    }

    fn handle_receive_body(&mut self, data: &[u8]) -> Result<(), BoxedError> {
        let event = SessionEvent::BodyReceived(data);
        self.handler.event(&mut self.control, event)?;
        Ok(())
    }

    fn handle_send_content(&mut self, buf: &mut [u8]) -> Result<usize, BoxedError> {
        let size = self.handler.upload_content(&mut self.control, buf)?;

        let event = SessionEvent::ContentSent(&buf[0..size]);
        self.handler.event(&mut self.control, event)?;

        Ok(size)
    }

    fn handle_receive_content(&mut self, data: &[u8]) -> Result<(), BoxedError> {
        let event = SessionEvent::ContentReceived(data);
        self.handler.event(&mut self.control, event)?;
        Ok(())
    }

    fn handle_progress(
        &mut self,
        download_total: u64,
        download_current: u64,
        upload_total: u64,
        upload_current: u64,
    ) -> Result<(), BoxedError> {
        let event = SessionEvent::Progress {
            download_total,
            download_current,
            upload_total,
            upload_current,
        };

        self.handler.event(&mut self.control, event)?;

        Ok(())
    }
}

fn format_header_field(name: &FieldName, value: &FieldValue) -> Result<String, Error> {
    Ok(format!("{}:{}", name, value))
}

fn parse_connect_address(text: &str) -> Option<SocketAddr> {
    // Extract from Curl_verboseconnect
    static PATTERN: OnceLock<Regex> = OnceLock::new();
    let re =
        PATTERN.get_or_init(|| Regex::new(r"Connected to .+ \(([0-9.:]+)\) port (\d+)").unwrap());

    let captures = re.captures(text);

    if let Some(captures) = captures {
        let address = captures.get(1).unwrap();
        let port = captures.get(2).unwrap();

        let address = match IpAddr::from_str(address.as_str()) {
            Ok(address) => address,
            Err(error) => {
                tracing::debug!(?error, "curl info parse ip addr");
                return None;
            }
        };
        let port = match u16::from_str(port.as_str()) {
            Ok(port) => port,
            Err(error) => {
                tracing::debug!(?error, "curl info parse port");
                return None;
            }
        };

        Some(SocketAddr::new(address, port))
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use std::net::Ipv4Addr;

    use super::*;

    #[test]
    fn test_parse_connect_address() {
        let result = parse_connect_address("Connected to 127.0.0.1 (127.0.0.1) port 39753 (#0)\n");
        let expect = Some(SocketAddr::new(
            IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)),
            39753,
        ));
        assert_eq!(result, expect);
    }
}
