mod common;
mod cookie;
mod curl;
mod pool;

use std::{cell::RefCell, rc::Rc};

use crate::{client::curl::CurlSession, error::Error};

use self::{cookie::CookieJar, pool::ConnectionPool};

pub use common::*;

#[derive(Debug, Clone)]
pub struct Client {
    config: Rc<RefCell<Config>>,
    connection_pool: ConnectionPool,
    cookie_jar: CookieJar,
}

impl Client {
    pub fn new(config: Config) -> Self {
        let cookie_jar = if config.http_cookies() {
            CookieJar::new()
        } else {
            CookieJar::new_disabled()
        };

        Self {
            config: Rc::new(RefCell::new(config)),
            connection_pool: ConnectionPool::new(),
            cookie_jar,
        }
    }

    pub fn config(&self) -> std::cell::Ref<Config> {
        self.config.borrow()
    }

    pub fn config_mut(&mut self) -> std::cell::RefMut<Config> {
        self.config.borrow_mut()
    }

    pub fn cookie_jar(&self) -> &CookieJar {
        &self.cookie_jar
    }

    pub fn cookie_jar_mut(&mut self) -> &mut CookieJar {
        &mut self.cookie_jar
    }

    pub fn submit<H: SessionHandler + 'static>(
        &self,
        request: Request,
        handler: H,
    ) -> (H, Result<(), Error>) {
        let url = request.url().as_str();
        let span = tracing::info_span!("client session", url);
        let _guard = span.enter();

        let mut session = match request.url().scheme() {
            "http" | "https" => {
                tracing::debug!(mode = "http", "init session");

                Box::new(CurlSession::new_http(
                    self.config.clone(),
                    request,
                    handler,
                    self.connection_pool.clone(),
                    self.cookie_jar.clone(),
                ))
            }
            "ftp" => {
                tracing::debug!(mode = "ftp", "init session");

                Box::new(CurlSession::new_ftp(
                    self.config.clone(),
                    request,
                    handler,
                    self.connection_pool.clone(),
                    self.cookie_jar.clone(),
                ))
            }
            _ => {
                return (
                    handler,
                    Err(Error::UnsupportedFeature {
                        feature: request.url().scheme().to_string(),
                    }),
                )
            }
        };

        session.wait()
    }
}

impl Default for Client {
    fn default() -> Self {
        Self::new(Config::default())
    }
}
