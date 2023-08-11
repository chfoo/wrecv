use crate::error::{Error, ParseError};

use super::{RequestHeader, ResponseHeader, ResponseTrailer};

pub fn scan_header_boundary(data: &[u8]) -> Option<usize> {
    let mut index = 0;

    for line in data.split_inclusive(|&v| v == b'\n') {
        index += line.len();

        if line.iter().all(|v| v.is_ascii_whitespace()) && line.ends_with(&[b'\n']) {
            return Some(index);
        }
    }

    None
}

pub(super) fn parse_request_header(data: &[u8]) -> Result<RequestHeader, Error> {
    let mut headers = [httparse::EMPTY_HEADER; 128];
    let mut request = httparse::Request::new(&mut headers);

    match request.parse(data) {
        Ok(status) => match status {
            httparse::Status::Complete(_) => Ok(request.into()),
            httparse::Status::Partial => {
                Err(ParseError::new("HTTP request header incomplete").into())
            }
        },
        Err(error) => Err(ParseError::new("HTTP request header parse error")
            .with_source(Box::new(error))
            .into()),
    }
}

pub(super) fn parse_response_header(data: &[u8]) -> Result<ResponseHeader, Error> {
    let mut headers = [httparse::EMPTY_HEADER; 128];
    let mut response = httparse::Response::new(&mut headers);

    match response.parse(data) {
        Ok(status) => match status {
            httparse::Status::Complete(_) => Ok(response.into()),
            httparse::Status::Partial => {
                Err(ParseError::new("HTTP response header incomplete").into())
            }
        },
        Err(error) => Err(ParseError::new("HTTP response header parse error")
            .with_source(Box::new(error))
            .into()),
    }
}

pub(super) fn parse_response_trailer(data: &[u8]) -> Result<ResponseTrailer, Error> {
    let mut trailer = ResponseTrailer::new();
    let mut headers = [httparse::EMPTY_HEADER; 128];

    let result = httparse::parse_headers(data, &mut headers);

    match result {
        Ok(status) => match status {
            httparse::Status::Complete((_size, headers)) => {
                for header in headers {
                    trailer.fields.append(header.name, header.value);
                }

                Ok(trailer)
            }
            httparse::Status::Partial => {
                Err(ParseError::new("HTTP header fields incomplete").into())
            }
        },
        Err(error) => Err(ParseError::new("HTTP header fields parse error")
            .with_source(Box::new(error))
            .into()),
    }
}

impl From<httparse::Request<'_, '_>> for RequestHeader {
    fn from(value: httparse::Request) -> Self {
        let mut crate_request = RequestHeader::new();
        crate_request.method = value.method.unwrap().to_string();
        crate_request.uri = value.path.unwrap().to_string();
        crate_request.version = format!("HTTP/1.{}", value.version.unwrap());

        for header in value.headers {
            crate_request.fields.append(header.name, header.value);
        }

        crate_request
    }
}

impl From<httparse::Response<'_, '_>> for ResponseHeader {
    fn from(value: httparse::Response<'_, '_>) -> Self {
        let mut crate_response = ResponseHeader::new();
        crate_response.version = format!("HTTP/1.{}", value.version.unwrap());
        crate_response.status_code = value.code.unwrap();
        crate_response.reason_phrase = value.reason.unwrap().to_string();

        for header in value.headers {
            crate_response.fields.append(header.name, header.value);
        }

        crate_response
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scan_header_boundary_empty() {
        assert_eq!(scan_header_boundary(b""), None);
    }

    #[test]
    fn test_scan_header_boundary_none() {
        assert_eq!(scan_header_boundary(b"abc"), None);
    }

    #[test]
    fn test_scan_header_boundary_one_line() {
        assert_eq!(scan_header_boundary(b"abc\r\n\r\nxyz"), Some(7));
    }

    #[test]
    fn test_scan_header_boundary_lines() {
        assert_eq!(scan_header_boundary(b"abc\r\ndef\r\n\r\nxyz"), Some(12));
    }

    #[test]
    fn test_scan_header_boundary_whitespace() {
        assert_eq!(scan_header_boundary(b"abc\r\n \t \r\nxyz"), Some(10));
    }

    #[test]
    fn test_parse_request() {
        let request = parse_request_header(
            "GET /index.html HTTP/1.1\r\nHost: example.com\r\n\r\n".as_bytes(),
        )
        .unwrap();

        assert_eq!(&request.method, "GET");
        assert_eq!(&request.uri, "/index.html");
        assert_eq!(&request.version, "HTTP/1.1");

        assert_eq!(request.fields.get("host"), Some(&"example.com".into()));
    }

    #[test]
    fn test_parse_response() {
        let response =
            parse_response_header("HTTP/1.1 200 OK\r\nContent-Length: 123\r\n\r\n".as_bytes())
                .unwrap();

        assert_eq!(&response.version, "HTTP/1.1");
        assert_eq!(response.status_code, 200);
        assert_eq!(&response.reason_phrase, "OK");

        assert_eq!(response.fields.get("content-length"), Some(&"123".into()));
    }

    #[test]
    fn test_parse_response_trailer() {
        let trailer = parse_response_trailer("Abc: xyz\r\n\r\n".as_bytes()).unwrap();

        assert_eq!(trailer.fields.get("abc"), Some(&"xyz".into()));
    }
}
