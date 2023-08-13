use std::{net::{Ipv4Addr, SocketAddr, TcpListener}, time::Duration};

use libunftp::{options::Shutdown, Server};
use tempfile::TempDir;
use tokio::{runtime::Runtime, sync::oneshot::Sender};
use unftp_sbe_fs::ServerExt;

pub struct ServerHandle {
    address: SocketAddr,
    shutdown_sender: Option<Sender<()>>,
    temp_dir: Option<TempDir>,
}

impl ServerHandle {
    pub fn address(&self) -> SocketAddr {
        self.address
    }

    pub fn close(&mut self) {
        if let Some(sender) = self.shutdown_sender.take() {
            sender.send(()).unwrap();
        }
        self.temp_dir.take();
    }
}

impl Drop for ServerHandle {
    fn drop(&mut self) {
        self.close();
    }
}

pub fn run_test_server() -> ServerHandle {
    let runtime = Runtime::new().unwrap();
    let temp_dir = tempfile::Builder::new()
        .prefix("wrecv-test-")
        .tempdir()
        .unwrap();
    let temp_dir_path = temp_dir.path().to_owned();

    let (sender, receiver) = tokio::sync::oneshot::channel::<()>();
    let server = Server::with_fs(temp_dir_path.clone()).shutdown_indicator(async {
        receiver.await.ok();
        Shutdown::new().grace_period(Duration::from_secs(0))
    });

    let address = SocketAddr::new(Ipv4Addr::LOCALHOST.into(), 0);
    let listener = TcpListener::bind(address).unwrap();
    let address = listener.local_addr().unwrap();
    drop(listener);

    std::thread::spawn(move || {
        runtime.block_on(async {
            let span = tracing::debug_span!("test_ftp_server", %address);
            let _gaurd = span.enter();

            tracing::debug!(?temp_dir_path, "starting test ftp server");
            server.listen(address.to_string()).await.unwrap();
            tracing::debug!("test ftp server stopped");
        });
    });

    ServerHandle {
        address,
        shutdown_sender: Some(sender),
        temp_dir: Some(temp_dir),
    }
}
