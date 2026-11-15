use glib::translate::IntoGlib;
use gtk4::{gio, glib};
use soup::prelude::*;

pub(super) struct Response {
    pub status: u32,
    pub body: glib::Bytes,
}

pub(super) fn send(
    method: &str,
    uri: &str,
    headers: &[(&str, &str)],
    body: Option<(&str, &[u8])>,
    cancel: Option<&gio::Cancellable>,
) -> Result<Response, glib::Error> {
    let session = soup::Session::builder().timeout(15).build();
    let message = soup::Message::new(method, uri)
        .map_err(|err| glib::Error::new(gio::IOErrorEnum::InvalidArgument, &err.to_string()))?;
    if let Some(request_headers) = message.request_headers() {
        for &(name, value) in headers {
            request_headers.append(name, value);
        }
    }
    if let Some((content_type, body)) = body {
        message.set_request_body_from_bytes(Some(content_type), Some(&glib::Bytes::from(body)));
    }
    let body = session.send_and_read(&message, cancel)?;
    Ok(Response {
        status: message.status().into_glib() as u32,
        body,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        io::{Read, Write},
        net::TcpListener,
        time::Duration,
    };

    #[test]
    fn request_sends_headers_and_body_and_preserves_http_errors() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let server = std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            stream
                .set_read_timeout(Some(Duration::from_secs(5)))
                .unwrap();
            let mut request = Vec::new();
            let mut chunk = [0; 1024];
            while !request.ends_with(b"code=test") {
                let size = stream.read(&mut chunk).unwrap();
                assert!(size > 0);
                request.extend_from_slice(&chunk[..size]);
            }
            let request = String::from_utf8(request).unwrap();
            assert!(request.starts_with("POST /token HTTP/1.1\r\n"));
            assert!(
                request
                    .to_ascii_lowercase()
                    .contains("authorization: bearer test\r\n")
            );
            assert!(
                request
                    .to_ascii_lowercase()
                    .contains("content-type: application/x-www-form-urlencoded\r\n")
            );
            stream.write_all(b"HTTP/1.1 401 Unauthorized\r\nContent-Length: 2\r\nConnection: close\r\n\r\n{}").unwrap();
        });
        let response = send(
            "POST",
            &format!("http://{address}/token"),
            &[("Authorization", "Bearer test")],
            Some(("application/x-www-form-urlencoded", b"code=test")),
            None,
        )
        .unwrap();
        assert_eq!(response.status, 401);
        assert_eq!(response.body.as_ref(), b"{}");
        server.join().unwrap();
    }
}
