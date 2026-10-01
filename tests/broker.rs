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
