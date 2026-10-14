use app_lib::core::company::{
    config::{validate_store_endpoint, CompanyConfig},
    store_client::StoreClient,
};
use std::{
    io::{Read, Write},
    net::TcpListener,
};

#[test]
fn company_flow08_release_configuration_requires_explicit_https_allowlist_and_no_placeholder() {
    let hosts = vec!["skills.internal.company.test".to_string()];
    for url in [
        "http://skills.internal.company.test",
        "https://localhost",
        "https://127.0.0.1",
        "https://skills-store.company.example",
        "https://outside.test",
        "https://user:password@skills.internal.company.test",
        "https://skills.internal.company.test?token=secret",
        "https://skills.internal.company.test/api",
    ] {
        assert!(
            validate_store_endpoint(url, &hosts, false).is_err(),
            "{url}"
        );
    }
    assert!(validate_store_endpoint("https://skills.internal.company.test", &hosts, false).is_ok());
    assert!(validate_store_endpoint("https://skills.internal.company.test", &[], false).is_err());
    assert!(validate_store_endpoint("http://127.0.0.1:4318", &[], true).is_ok());
    assert!(validate_store_endpoint("http://outside.test", &[], true).is_err());
    assert!(
        validate_store_endpoint("", &[], false).is_ok(),
        "unconfigured local features work without pretending a Store exists"
    );
}

#[test]
fn company_flow08_redirect_never_forwards_bearer_to_another_origin() {
    let destination = TcpListener::bind("127.0.0.1:0").unwrap();
    destination.set_nonblocking(true).unwrap();
    let source = TcpListener::bind("127.0.0.1:0").unwrap();
    let url = format!("http://{}", source.local_addr().unwrap());
    let redirect = format!("http://{}/steal", destination.local_addr().unwrap());
    let thread = std::thread::spawn(move || {
        let (mut stream, _) = source.accept().unwrap();
        stream
            .set_read_timeout(Some(std::time::Duration::from_secs(5)))
            .unwrap();
        let mut request = vec![];
        let mut buffer = [0u8; 1024];
        while !request.windows(4).any(|chunk| chunk == b"\r\n\r\n") {
            let n = stream.read(&mut buffer).unwrap();
            if n == 0 {
                break;
            }
            request.extend_from_slice(&buffer[..n]);
        }
        assert!(String::from_utf8(request)
            .unwrap()
            .to_lowercase()
            .contains("authorization: bearer synthetic-release-test"));
        stream.write_all(format!("HTTP/1.1 302 Found\r\nLocation: {redirect}\r\nContent-Length: 0\r\nConnection: close\r\n\r\n").as_bytes()).unwrap();
    });
    let root = tempfile::tempdir().unwrap();
    let config =
        CompanyConfig::for_test(&url, root.path().join("profile"), root.path().join("data"))
            .unwrap();
    assert!(StoreClient::new(&config)
        .unwrap()
        .detail("synthetic-release-test", "skl_001")
        .is_err());
    thread.join().unwrap();
    assert_eq!(
        destination.accept().unwrap_err().kind(),
        std::io::ErrorKind::WouldBlock
    );
}
