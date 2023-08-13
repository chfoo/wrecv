use std::net::{Ipv4Addr, SocketAddr, TcpListener};

use axum::{http::header::SET_COOKIE, response::Redirect, routing::get, Router};
use tokio::{runtime::Runtime, sync::oneshot::Sender};

pub struct ServerHandle {
    address: SocketAddr,
    shutdown_sender: Option<Sender<()>>,
}

impl ServerHandle {
    pub fn address(&self) -> SocketAddr {
        self.address
    }

    pub fn close(&mut self) {
        if let Some(sender) = self.shutdown_sender.take() {
            sender.send(()).unwrap();
        }
    }
}

impl Drop for ServerHandle {
    fn drop(&mut self) {
        self.close();
    }
}

pub fn run_test_server() -> ServerHandle {
    let app = Router::new()
        .route("/", get(|| async { "Hello world!" }))
        .route("/redirect", get(|| async { Redirect::temporary("/") }))
        .route(
            "/set-cookie",
            get(|| async {
                (
                    axum::response::AppendHeaders([(SET_COOKIE, "key1=value1")]),
                    "cookie",
                )
            }),
        );

    let (sender, receiver) = tokio::sync::oneshot::channel::<()>();

    let address = SocketAddr::new(Ipv4Addr::LOCALHOST.into(), 0);
    let listener = TcpListener::bind(address).unwrap();
    let address = listener.local_addr().unwrap();

    let runtime = Runtime::new().unwrap();

    std::thread::spawn(move || {
        runtime.block_on(async {
            let span = tracing::debug_span!("test_http_server", %address);
            let _guard = span.enter();
            let server = axum::Server::from_tcp(listener)
                .unwrap()
                .serve(app.into_make_service());
            let server = server.with_graceful_shutdown(async {
                receiver.await.ok();
            });

            tracing::debug!("starting test http server");
            server.await.unwrap();
            tracing::debug!("test http server stopped");
        });
    });

    ServerHandle {
        address,
        shutdown_sender: Some(sender),
    }
}
