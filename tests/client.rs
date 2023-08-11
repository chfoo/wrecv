mod common;

use wrecv::client::{Client, Config, Request, SessionControl, SessionEvent, SessionHandler};

#[tracing_test::traced_test]
#[test]
fn test_client() {
    let mut server = common::mock_http_server();
    server.run_background().unwrap();

    let mut config = Config::new();
    config.set_http_09(true).set_http_cookies(true);

    let client = Client::new(config);
    let request = Request::new(
        format!("http://{}/index.html", server.address())
            .parse()
            .unwrap(),
    );

    struct MyHandler;

    impl SessionHandler for MyHandler {
        fn event(
            &mut self,
            _control: &mut dyn SessionControl,
            event: SessionEvent,
        ) -> Result<(), Box<dyn std::error::Error + Sync + Send>> {
            match event {
                SessionEvent::HttpRequest(_data, request) => {
                    assert_eq!(&request.uri, "/index.html");
                }
                SessionEvent::HttpResponse(_data, response) => {
                    assert_eq!(response.status_code, 200);
                }
                SessionEvent::ContentReceived(data) => {
                    assert_eq!(data, "Hello world!".as_bytes());
                }
                _ => {}
            }
            Ok(())
        }
    }

    let handler = MyHandler;
    let (_handler, result) = client.submit(request, handler);
    result.unwrap();

    server.close();
}
