use std::{
    io::{Read, Write},
    net::{Ipv4Addr, SocketAddr, SocketAddrV4, TcpListener, TcpStream},
    sync::{Arc, Mutex},
    time::Duration,
};

pub struct SimpleServer {
    handler: Option<Box<dyn RequestHandler>>,
    stop_flag: Arc<Mutex<bool>>,
    address: SocketAddr,
}

impl SimpleServer {
    pub fn new(handler: Box<dyn RequestHandler>) -> Self {
        Self {
            handler: Some(handler),
            stop_flag: Arc::new(Mutex::new(false)),
            address: SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::LOCALHOST, 0)),
        }
    }

    pub fn address(&self) -> SocketAddr {
        self.address
    }

    pub fn run_background(&mut self) -> std::io::Result<()> {
        let listener = TcpListener::bind("127.0.0.1:0")?;
        self.address = listener.local_addr()?;
        let stop_flag = self.stop_flag.clone();
        let handler = self.handler.take().unwrap();

        std::thread::spawn(move || Self::run_loop(listener, handler, stop_flag));

        Ok(())
    }

    pub fn close(&self) {
        let mut flag = self.stop_flag.lock().unwrap();
        *flag = true;
    }

    fn run_loop(
        listener: TcpListener,
        mut handler: Box<dyn RequestHandler>,
        stop_flag: Arc<Mutex<bool>>,
    ) -> std::io::Result<()> {
        listener.set_nonblocking(true)?;

        loop {
            let flag = stop_flag.lock().unwrap();

            if *flag {
                break;
            }

            drop(flag);

            match listener.accept() {
                Ok((stream, _addr)) => handler.handle(stream)?,
                Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                    std::thread::sleep(Duration::from_secs(1));
                }
                Err(error) => return Err(error),
            }
        }

        Ok(())
    }
}

impl Drop for SimpleServer {
    fn drop(&mut self) {
        self.close();
    }
}

pub trait RequestHandler: Send {
    fn handle(&mut self, stream: TcpStream) -> std::io::Result<()>;
}

pub struct MockHttpHandler {}

impl MockHttpHandler {
    pub fn new() -> Self {
        Self {}
    }
}

impl RequestHandler for MockHttpHandler {
    fn handle(&mut self, mut stream: TcpStream) -> std::io::Result<()> {
        stream.set_nodelay(true)?;
        stream.set_read_timeout(Some(Duration::from_secs(1)))?;

        let mut buf = Vec::new();
        let _ = stream.read_to_end(&mut buf);

        stream.write_all(
            "HTTP/1.1 200 OK\
\r\nContent-type: text/plain\
\r\nSet-Cookie: test=\"Hello world!\"\
\r\n\r\n\
Hello world!"
                .as_bytes(),
        )?;
        stream.flush()?;
        drop(stream);

        Ok(())
    }
}

pub fn mock_http_server() -> SimpleServer {
    SimpleServer::new(Box::new(MockHttpHandler::new()))
}
