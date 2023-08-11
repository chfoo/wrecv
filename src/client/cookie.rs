use std::{
    fmt::Debug,
    sync::{Arc, Mutex},
};

use cookie_store::CookieStore;
use url::Url;

use crate::http::HeaderFields;

const MAX_HEADER_VALUE_LEN: usize = 4096usize;

#[derive(Debug, Clone)]
pub struct CookieJar {
    store: Option<Arc<Mutex<CookieStore>>>,
}

impl CookieJar {
    pub fn new() -> Self {
        Self {
            store:Some(Arc::new(Mutex::new(CookieStore::new(None))))
        }
    }

    pub fn new_disabled() -> Self {
        Self { store: None }
    }

    pub fn get_request_string(&self, url: &Url) -> String {
        if let Some(store) = &self.store {
            let store = store.lock().unwrap();
            format_client_header(store.get_request_values(url), MAX_HEADER_VALUE_LEN)
        } else {
            String::new()
        }
    }

    pub fn parse_from_response(&self, url: &Url, fields: &HeaderFields) {
        if let Some(store) = &self.store {
            let mut store = store.lock().unwrap();

            for value in fields.get_all("Set-Cookie") {
                let _ = store.parse(&value.to_string_lossy(), url);
            }
        }
    }

    pub fn clear(&self) {
        if let Some(store) = &self.store {
            let mut store = store.lock().unwrap();

            store.clear();
        }
    }
}

impl Default for CookieJar {
    fn default() -> Self {
        Self::new()
    }
}

fn format_client_header<'a, I: IntoIterator<Item = (&'a str, &'a str)>>(
    cookies: I,
    max_len: usize,
) -> String {
    let mut buf = String::new();

    for (name, value) in cookies {
        if buf.len() + name.len() + value.len() + 4 <= max_len {
            if !buf.is_empty() {
                buf.push_str("; ");
            }
            buf.push_str(name);
            buf.push('=');

            if value.contains(' ') {
                buf.push('"');
                buf.push_str(value);
                buf.push('"');
            } else {
                buf.push_str(value);
            }
        }
    }

    buf
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_client_header() {
        let result = format_client_header([("k1", "v1")].into_iter(), 4096);
        assert_eq!(&result, "k1=v1");

        let result = format_client_header(
            [("k1", "v1"), ("k2", "v2"), ("k3", "v 3")].into_iter(),
            4096,
        );
        assert_eq!(&result, "k1=v1; k2=v2; k3=\"v 3\"");
    }
}
