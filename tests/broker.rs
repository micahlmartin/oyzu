use oyzu::broker::{Fetcher, Session, Source};
use serde_json::json;
use std::{
    fs,
    io::{Read, Write},
    net::TcpListener,
    thread,
    time::{Duration, Instant},
};

fn upstream(response: String) -> (String, thread::JoinHandle<String>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = format!("http://{}/", listener.local_addr().unwrap());
    let worker = thread::spawn(move || {
        let (mut socket, _) = listener.accept().unwrap();
        socket
            .set_read_timeout(Some(Duration::from_secs(5)))
            .unwrap();
        let mut request = Vec::new();
        let mut byte = [0u8; 1];
        while !request.ends_with(b"\r\n\r\n") {
            socket.read_exact(&mut byte).unwrap();
            request.push(byte[0]);
        }
        socket.write_all(response.as_bytes()).unwrap();
        String::from_utf8(request).unwrap()
    });
    (address, worker)
}

fn sequence(responses: Vec<String>) -> (String, thread::JoinHandle<Vec<String>>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    listener.set_nonblocking(true).unwrap();
    let address = format!("http://{}/", listener.local_addr().unwrap());
    let worker = thread::spawn(move || {
        let deadline = Instant::now() + Duration::from_secs(10);
        let mut requests = Vec::new();
        for response in responses {
            let mut socket = loop {
                match listener.accept() {
                    Ok((socket, _)) => break socket,
                    Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                        assert!(
                            Instant::now() < deadline,
                            "expected upstream request never arrived"
                        );
                        thread::sleep(Duration::from_millis(5));
                    }
                    Err(error) => panic!("{error}"),
                }
            };
            socket.set_nonblocking(false).unwrap();
            socket
                .set_read_timeout(Some(Duration::from_secs(5)))
                .unwrap();
            let mut request = Vec::new();
            let mut byte = [0u8; 1];
            while !request.ends_with(b"\r\n\r\n") {
                socket.read_exact(&mut byte).unwrap();
                request.push(byte[0]);
            }
            socket.write_all(response.as_bytes()).unwrap();
            requests.push(String::from_utf8(request).unwrap());
        }
        requests
    });
    (address, worker)
}

fn reply(status: u16, headers: &str, body: &str) -> String {
    format!(
        "HTTP/1.1 {status} Test\r\n{headers}Content-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    )
}

#[test]
fn retries_transient_statuses_and_partial_transfers_without_reusing_failed_bytes() {
    for status in [408, 429, 502, 503, 504] {
        let (address, server) = sequence(vec![
            reply(status, "Retry-After: 0\r\n", "secret"),
            reply(200, "", "complete"),
        ]);
        let mut fetcher = Fetcher::new(vec![Source::new(
            "source",
            &address,
            Some("Bearer canary".into()),
        )
        .unwrap()])
        .unwrap();
        let response = fetcher.fetch(&address).unwrap();
        assert_eq!(response.status, 200);
        assert_eq!(response.body, b"complete");
        assert!(server
            .join()
            .unwrap()
            .iter()
            .all(|r| r.to_lowercase().contains("authorization: bearer canary")));
    }
    let (address, server) = sequence(vec![
        "HTTP/1.1 200 OK\r\nContent-Length: 20\r\nConnection: close\r\n\r\npartial".into(),
        reply(200, "", "complete"),
    ]);
    let mut fetcher = Fetcher::new(vec![Source::new("source", &address, None).unwrap()]).unwrap();
    assert_eq!(fetcher.fetch(&address).unwrap().body, b"complete");
    assert_eq!(server.join().unwrap().len(), 2);
}

#[test]
fn retries_are_bounded_and_terminal_statuses_and_long_delays_are_not_retried() {
    let (address, server) = sequence(vec![reply(503, "", "secret"); 3]);
    let mut fetcher = Fetcher::new(vec![Source::new("source", &address, None).unwrap()]).unwrap();
    let result = fetcher.fetch(&address).unwrap();
    assert_eq!(result.status, 503);
    assert_eq!(result.body, b"approved source returned HTTP 503");
    assert_eq!(server.join().unwrap().len(), 3);
    for (status, after) in [
        (401, ""),
        (403, ""),
        (404, ""),
        (500, ""),
        (429, "Retry-After: 600\r\n"),
        (503, "Retry-After: invalid\r\n"),
    ] {
        let (address, server) = upstream(reply(status, after, "secret"));
        let mut fetcher =
            Fetcher::new(vec![Source::new("source", &address, None).unwrap()]).unwrap();
        let result = fetcher.fetch(&address).unwrap();
        assert_eq!(result.status, status);
        assert!(!String::from_utf8(result.body).unwrap().contains("secret"));
        server.join().unwrap();
    }
}

#[test]
fn redirects_do_not_reset_retries_or_forward_the_original_credentials() {
    let (second, second_server) = sequence(vec![reply(503, "", "secret"); 2]);
    let (first, first_server) = sequence(vec![
        reply(503, "", "secret"),
        reply(302, &format!("Location: {second}\r\n"), ""),
    ]);
    let mut fetcher = Fetcher::new(vec![
        Source::new("first", &first, Some("Bearer first-only".into())).unwrap(),
        Source::new("second", &second, None).unwrap(),
    ])
    .unwrap();
    assert_eq!(fetcher.fetch(&first).unwrap().status, 503);
    assert_eq!(first_server.join().unwrap().len(), 2);
    let second_requests = second_server.join().unwrap();
    assert_eq!(second_requests.len(), 2);
    assert!(second_requests
        .iter()
        .all(|r| !r.to_lowercase().contains("authorization:")));
}

#[test]
fn spool_distinguishes_denial_from_upstream_transport_failure_without_secret_urls() {
    for denied in [true, false] {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = format!("http://{}/", listener.local_addr().unwrap());
        drop(listener);
        let spool = tempfile::tempdir().unwrap();
        let private = tempfile::tempdir().unwrap();
        let sources = if denied {
            vec![]
        } else {
            vec![Source::new("source", &address, Some("Bearer secret".into())).unwrap()]
        };
        let session = Session::start(spool.path(), private.path(), sources).unwrap();
        let id = "0123456789abcdef0123456789abcdef";
        let request = json!({"url":format!("{address}?token=private-query")});
        fs::write(
            spool.path().join(format!("{id}.pending")),
            request.to_string(),
        )
        .unwrap();
        fs::rename(
            spool.path().join(format!("{id}.pending")),
            spool.path().join(format!("{id}.request")),
        )
        .unwrap();
        let response = spool.path().join(format!("{id}.response"));
        let deadline = Instant::now() + Duration::from_secs(10);
        while !response.exists() {
            assert!(Instant::now() < deadline);
            thread::sleep(Duration::from_millis(10));
        }
        let text = fs::read_to_string(response).unwrap();
        let metadata: serde_json::Value = serde_json::from_str(&text).unwrap();
        assert_eq!(metadata["status"], if denied { 403 } else { 502 });
        assert_eq!(
            metadata["errorCode"],
            if denied {
                "SOURCE_DENIED"
            } else {
                "SOURCE_UNAVAILABLE"
            }
        );
        let body = fs::read_to_string(spool.path().join(format!("{id}.body"))).unwrap();
        for secret in ["private-query", "Bearer secret", &address] {
            assert!(!text.contains(secret) && !body.contains(secret));
        }
        drop(session);
    }
}

#[test]
fn credentials_are_host_only_and_redirects_are_reauthorized() {
    let (second, second_server) =
        upstream("HTTP/1.1 200 OK\r\nContent-Length: 7\r\nConnection: close\r\n\r\npackage".into());
    let (first,first_server)=upstream(format!("HTTP/1.1 302 Found\r\nLocation: {second}files/a.whl\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"));
    let mut fetcher = Fetcher::new(vec![
        Source::new(
            "private",
            &format!("{first}index/"),
            Some("Bearer canary-secret".into()),
        )
        .unwrap(),
        Source::new("files", &format!("{second}files/"), None).unwrap(),
    ])
    .unwrap();
    let response = fetcher.fetch(&format!("{first}index/a")).unwrap();
    assert_eq!(response.body, b"package");
    assert_eq!(response.source_id, "files");
    assert!(first_server
        .join()
        .unwrap()
        .to_lowercase()
        .contains("authorization: bearer canary-secret"));
    assert!(!second_server
        .join()
        .unwrap()
        .to_lowercase()
        .contains("authorization:"));
    for url in [
        format!("{first}admin"),
        format!("{first}index/%2fadmin"),
        "https://unapproved.invalid/package".into(),
    ] {
        let error = fetcher.fetch(&url).err().unwrap().to_string();
        assert!(error.contains("SOURCE_DENIED"));
        assert!(!error.contains("canary-secret"));
    }
}

#[test]
fn scoped_spool_relays_content_without_exposing_upstream_headers() {
    let (address,server)=upstream("HTTP/1.1 200 OK\r\nContent-Length: 4\r\nSet-Cookie: upstream-secret\r\nConnection: close\r\n\r\nbody".into());
    let spool = tempfile::tempdir().unwrap();
    let private = tempfile::tempdir().unwrap();
    let session = Session::start(
        spool.path(),
        private.path(),
        vec![Source::new("private", &address, Some("Bearer broker-only".into())).unwrap()],
    )
    .unwrap();
    let id = "0123456789abcdef0123456789abcdef";
    fs::write(
        spool.path().join(format!("{id}.pending")),
        serde_json::to_vec(&json!({"url":address})).unwrap(),
    )
    .unwrap();
    fs::rename(
        spool.path().join(format!("{id}.pending")),
        spool.path().join(format!("{id}.request")),
    )
    .unwrap();
    let response = spool.path().join(format!("{id}.response"));
    let deadline = Instant::now() + Duration::from_secs(10);
    while !response.exists() {
        assert!(Instant::now() < deadline);
        thread::sleep(Duration::from_millis(10));
    }
    let metadata = fs::read_to_string(response).unwrap();
    assert!(!metadata.contains("secret"));
    assert!(!metadata.contains("broker-only"));
    assert_eq!(
        fs::read(spool.path().join(format!("{id}.body"))).unwrap(),
        b"body"
    );
    drop(session);
    assert!(server.join().unwrap().contains("broker-only"));
}

#[test]
fn denied_redirect_is_not_followed_and_error_body_is_not_relayed() {
    let (address,server)=upstream("HTTP/1.1 302 Found\r\nLocation: https://unapproved.invalid/path\r\nContent-Length: 0\r\nConnection: close\r\n\r\n".into());
    let mut fetcher = Fetcher::new(vec![Source::new("private", &address, None).unwrap()]).unwrap();
    assert!(fetcher.fetch(&address).is_err());
    server.join().unwrap();
    let (address, server) = upstream(
        "HTTP/1.1 401 Unauthorized\r\nContent-Length: 6\r\nConnection: close\r\n\r\nsecret".into(),
    );
    let mut fetcher = Fetcher::new(vec![Source::new("private", &address, None).unwrap()]).unwrap();
    let response = fetcher.fetch(&address).unwrap();
    assert_eq!(response.status, 401);
    assert!(!String::from_utf8(response.body).unwrap().contains("secret"));
    server.join().unwrap();
}
