pub type BoxedError = Box<dyn std::error::Error + Send + Sync>;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("unsupported feature {feature}")]
    UnsupportedFeature { feature: String },

    #[error("invalid argument {value} {reason}")]
    InvalidArgument { value: String, reason: String },

    #[error(transparent)]
    Parse(#[from] ParseError),

    #[error(transparent)]
    Protocol(#[from] ProtocolError),

    #[error(transparent)]
    Network(#[from] NetworkError),

    #[error(transparent)]
    Io(#[from] std::io::Error),

    #[error("miscellaneous/internal error: {0}")]
    Other(#[from] OtherError),
}

impl From<curl::Error> for Error {
    fn from(value: curl::Error) -> Self {
        if value.is_couldnt_connect() {
            Self::Network(NetworkError::Connect(Box::new(value)))
        } else if value.is_couldnt_resolve_host() || value.is_couldnt_resolve_proxy() {
            Self::Network(NetworkError::Dns(Box::new(value)))
        } else if value.is_ssl_connect_error()
            || value.is_ssl_certproblem()
            || value.is_peer_failed_verification()
            || value.is_ssl_issuer_error()
        {
            Self::Protocol(ProtocolError::TlsVerification(Box::new(value)))
        } else if value.is_operation_timedout() {
            Self::Network(NetworkError::TimedOut(Box::new(value)))
        } else {
            Self::Other(OtherError::from(value))
        }
    }
}

impl From<trust_dns_resolver::error::ResolveError> for Error {
    fn from(value: trust_dns_resolver::error::ResolveError) -> Self {
        match value.kind() {
            trust_dns_resolver::error::ResolveErrorKind::NoConnections => {
                NetworkError::Dns(Box::new(value)).into()
            }

            trust_dns_resolver::error::ResolveErrorKind::Timeout => {
                NetworkError::TimedOut(Box::new(value)).into()
            }

            trust_dns_resolver::error::ResolveErrorKind::NoRecordsFound {
                query: _,
                soa: _,
                negative_ttl: _,
                response_code: _,
                trusted: _,
            } => NetworkError::Dns(Box::new(value)).into(),

            trust_dns_resolver::error::ResolveErrorKind::Proto(_)
            | trust_dns_resolver::error::ResolveErrorKind::Message(_)
            | trust_dns_resolver::error::ResolveErrorKind::Msg(_)
            | trust_dns_resolver::error::ResolveErrorKind::Io(_) => Self::Other(value.into()),

            _ => Self::Other(value.into()),
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub struct ParseError {
    reason: String,
    position: Option<u64>,
    snippet: Option<String>,
    #[source]
    source: Option<BoxedError>,
}

impl ParseError {
    pub fn new<S: Into<String>>(reason: S) -> Self {
        Self {
            reason: reason.into(),
            position: None,
            snippet: None,
            source: None,
        }
    }

    pub fn with_position(mut self, position: u64) -> Self {
        self.position = Some(position);
        self
    }

    pub fn with_str_position(mut self, char_position: u64, text: &str) -> Self {
        self.position = Some(char_position);
        self.snippet = Some(text.chars().skip(char_position as usize).take(5).collect());
        self
    }

    pub fn with_source(mut self, source: BoxedError) -> Self {
        self.source = Some(source);
        self
    }

    pub fn reason(&self) -> &str {
        &self.reason
    }

    pub fn position(&self) -> Option<u64> {
        self.position
    }

    pub fn snippet(&self) -> Option<&str> {
        self.snippet.as_deref()
    }
}

impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(position) = self.position {
            if let Some(snippet) = &self.snippet {
                f.write_fmt(format_args!(
                    "parse error near position {} ({:?}): {}",
                    position, snippet, self.reason
                ))
            } else {
                f.write_fmt(format_args!(
                    "parse error near position {}: {}",
                    position, self.reason
                ))
            }
        } else {
            f.write_fmt(format_args!("parse error: {}", self.reason))
        }
    }
}

#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum ProtocolError {
    #[error("protocol request error: {0}")]
    InvalidRequest(BoxedError),

    #[error("protocol response error: {0}")]
    InvalidResponse(BoxedError),

    #[error("TLS verification error: {0}")]
    TlsVerification(BoxedError),

    #[error(transparent)]
    Custom(#[from] BoxedError),
}

#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum NetworkError {
    #[error("connection error: {0}")]
    Connect(BoxedError),

    #[error("DNS resolution error: {0}")]
    Dns(BoxedError),

    #[error("network operation timed out: {0}")]
    TimedOut(BoxedError),

    #[error("connection disconnected: {0}")]
    Disconnected(BoxedError),

    #[error(transparent)]
    Custom(#[from] BoxedError),
}

#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum OtherError {
    #[error(transparent)]
    Curl(#[from] curl::Error),

    #[error(transparent)]
    Trust(#[from] trust_dns_resolver::error::ResolveError),

    #[error(transparent)]
    Custom(#[from] BoxedError),
}
