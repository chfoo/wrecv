use std::{
    fmt::Debug,
    net::{IpAddr, Ipv4Addr, SocketAddr},
    sync::OnceLock,
};

use url::Url;

use crate::{
    error::{BoxedError, Error},
    http::{HeaderFields, RequestHeader, ResponseHeader, ResponseTrailer},
};

#[derive(Debug, Clone)]
pub struct Config {
    bind_address: IpAddr,
    http_user_agent: String,
    http_headers: HeaderFields,
    http_09: bool,
    http_compression: bool,
    http_cookies: bool,
    tls_verification: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self::new()
    }
}

impl Config {
    pub fn new() -> Self {
        Self {
            bind_address: IpAddr::V4(Ipv4Addr::UNSPECIFIED),
            http_user_agent: default_user_agent().to_string(),
            http_headers: Self::make_default_http_headers(),
            http_09: false,
            http_compression: false,
            http_cookies: false,
            tls_verification: true,
        }
    }

    fn make_default_http_headers() -> HeaderFields {
        let mut fields = HeaderFields::new();

        for (name, value) in default_http_headers() {
            fields.append(*name, *value);
        }

        fields
    }

    pub fn bind_address(&self) -> IpAddr {
        self.bind_address
    }

    pub fn set_bind_address(&mut self, bind_address: IpAddr) -> &mut Self {
        self.bind_address = bind_address;
        self
    }

    pub fn http_user_agent(&self) -> &str {
        self.http_user_agent.as_ref()
    }

    pub fn set_http_user_agent(&mut self, user_agent: String) -> &mut Self {
        self.http_user_agent = user_agent;
        self
    }

    pub fn http_headers(&self) -> &HeaderFields {
        &self.http_headers
    }

    pub fn http_headers_mut(&mut self) -> &mut HeaderFields {
        &mut self.http_headers
    }

    pub fn set_http_headers(&mut self, http_headers: HeaderFields) -> &mut Self {
        self.http_headers = http_headers;
        self
    }

    pub fn http_09(&self) -> bool {
        self.http_09
    }

    pub fn set_http_09(&mut self, enabled: bool) -> &mut Self {
        self.http_09 = enabled;
        self
    }

    pub fn http_compression(&self) -> bool {
        self.http_compression
    }

    pub fn set_http_compression(&mut self, enabled: bool) -> &mut Self {
        self.http_compression = enabled;
        self
    }

    pub fn http_cookies(&self) -> bool {
        self.http_cookies
    }

    pub fn set_http_cookies(&mut self, enabled: bool) -> &mut Self {
        self.http_cookies = enabled;
        self
    }

    pub fn tls_verification(&self) -> bool {
        self.tls_verification
    }

    pub fn set_tls_verification(&mut self, enabled: bool) -> &mut Self {
        self.tls_verification = enabled;
        self
    }
}

#[derive(Debug, Clone)]
pub struct Request {
    url: Url,
    http_headers: HeaderFields,
}

impl Request {
    pub fn new(url: Url) -> Self {
        Self {
            url,

            http_headers: HeaderFields::new(),
        }
    }

    pub fn url(&self) -> &Url {
        &self.url
    }

    pub fn set_url(&mut self, url: Url) -> &mut Self {
        self.url = url;
        self
    }

    pub fn http_headers(&self) -> &HeaderFields {
        &self.http_headers
    }

    pub fn http_headers_mut(&mut self) -> &mut HeaderFields {
        &mut self.http_headers
    }

    pub fn set_http_headers(&mut self, http_headers: HeaderFields) -> &mut Self {
        self.http_headers = http_headers;
        self
    }
}

pub trait Session<H: SessionHandler>: Debug {
    fn wait(&mut self) -> (H, Result<(), Error>);
}

pub trait SessionControl: Debug {
    fn abort(&mut self);
}

pub trait SessionHandler {
    fn upload_content(
        &mut self,
        control: &mut dyn SessionControl,
        buf: &mut [u8],
    ) -> Result<usize, BoxedError> {
        let _ = control;
        let _ = buf;
        Ok(0)
    }

    fn event(
        &mut self,
        control: &mut dyn SessionControl,
        event: SessionEvent,
    ) -> Result<(), BoxedError> {
        let _ = control;
        let _ = event;
        Ok(())
    }
}

#[non_exhaustive]
#[derive(Debug, Clone)]
pub enum SessionEvent<'a> {
    Connected(SocketAddr),
    HeaderReceived(&'a [u8]),
    HeaderSent(&'a [u8]),
    BodyReceived(&'a [u8]),
    BodySent(&'a [u8]),
    ContentSent(&'a [u8]),
    ContentReceived(&'a [u8]),
    HttpRequest(&'a [u8], RequestHeader),
    HttpResponse(&'a [u8], ResponseHeader),
    HttpResponseTrailer(&'a [u8], ResponseTrailer),
    Progress {
        download_total: u64,
        download_current: u64,
        upload_total: u64,
        upload_current: u64,
    },
}

pub fn default_user_agent() -> &'static str {
    static DEFAULT_USER_AGENT: OnceLock<String> = OnceLock::new();

    let version = DEFAULT_USER_AGENT.get_or_init(|| {
        let crate_version = crate::version::get_crate_version_mmp();
        let curl_version = curl::Version::get();

        format!(
            "Mozilla/5.0 (compatible; not Gecko KHTML AppleWebKit Firefox Chrome Safari) wrecv/{}.{} curl/{}.{}",
            crate_version.0,
            crate_version.1,
            (curl_version.version_num() >> 16) as u8,
            (curl_version.version_num() >> 8) as u8
        )
    });

    version
}

pub fn default_http_headers() -> &'static [(&'static str, &'static str)] {
    &[]
}
