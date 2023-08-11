use std::{
    fmt::Debug,
    sync::{Arc, Mutex},
};

use curl::easy::Easy;

#[derive(Clone)]
pub struct ConnectionPool {
    curl_handles: Arc<Mutex<Vec<Easy>>>,
}

impl ConnectionPool {
    const MAX_HANDLES: usize = 20;

    pub fn new() -> Self {
        Self {
            curl_handles: Arc::new(Mutex::new(Vec::new())),
        }
    }

    pub fn get_curl_handle(&self) -> Easy {
        let mut handles = self.curl_handles.lock().unwrap();

        handles
            .pop()
            .map(|mut h| {
                h.reset();
                h
            })
            .unwrap_or_else(Easy::new)
    }

    pub fn put_curl_handle(&mut self, curl_handle: Easy) {
        let mut handles = self.curl_handles.lock().unwrap();

        if handles.len() < Self::MAX_HANDLES {
            handles.push(curl_handle);
        }
    }
}

impl Debug for ConnectionPool {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ConnectionPool")
            .field("...", &"...")
            .finish()
    }
}
