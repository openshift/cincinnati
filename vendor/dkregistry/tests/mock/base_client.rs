extern crate dkregistry;
extern crate mockito;
extern crate tokio;

use self::mockito::mock;
use self::tokio::runtime::Runtime;

static API_VERSION_K: &'static str = "Docker-Distribution-API-Version";
static API_VERSION_V: &'static str = "registry/2.0";

#[test]
#[ignore]
fn test_base_no_insecure() {
    let addr = mockito::server_address().to_string();
    let _m = mock("GET", "/v2/")
        .with_status(200)
        .with_header(API_VERSION_K, API_VERSION_V)
        .create();

    let runtime = Runtime::new().unwrap();
    let dclient = dkregistry::v2::Client::configure()
        .registry(&addr)
        .insecure_registry(false)
        .username(None)
        .password(None)
        .build()
        .unwrap();

    let futcheck = dclient.is_v2_supported();

    // This relies on the fact that mockito is HTTP-only and
    // trying to speak TLS to it results in garbage/errors.
    runtime.block_on(futcheck).unwrap_err();

    mockito::reset();
}

#[test]
#[ignore]
fn test_base_useragent() {
    let addr = mockito::server_address().to_string();
    let _m = mock("GET", "/v2/")
        .match_header("user-agent", dkregistry::USER_AGENT)
        .with_status(200)
        .with_header(API_VERSION_K, API_VERSION_V)
        .create();

    let runtime = Runtime::new().unwrap();
    let dclient = dkregistry::v2::Client::configure()
        .registry(&addr)
        .insecure_registry(true)
        .username(None)
        .password(None)
        .build()
        .unwrap();

    let futcheck = dclient.is_v2_supported();

    let res = runtime.block_on(futcheck).unwrap();
    assert_eq!(res, true);

    mockito::reset();
}

#[test]
fn test_base_custom_useragent() {
    let ua = "custom-ua/1.0";

    let addr = mockito::server_address().to_string();
    let _m = mock("GET", "/v2/")
        .match_header("user-agent", ua)
        .with_status(200)
        .with_header(API_VERSION_K, API_VERSION_V)
        .create();

    let runtime = Runtime::new().unwrap();
    let dclient = dkregistry::v2::Client::configure()
        .registry(&addr)
        .insecure_registry(true)
        .user_agent(Some(ua.to_string()))
        .username(None)
        .password(None)
        .build()
        .unwrap();

    let futcheck = dclient.is_v2_supported();

    let res = runtime.block_on(futcheck).unwrap();
    assert_eq!(res, true);

    mockito::reset();
}

mod test_custom_root_certificate {
    use dkregistry::v2::Client;
    use native_tls::{HandshakeError, Identity, TlsStream};
    use reqwest::Certificate;
    use std::error::Error;
    use std::net::TcpListener;
    use std::path::{Path, PathBuf};

    fn run_server(listener: TcpListener, identity: Identity) -> Result<(), std::io::Error> {
        println!("Will accept tls connections at {}", listener.local_addr()?);

        let mut incoming = listener.incoming();

        let test_server = native_tls::TlsAcceptor::new(identity).unwrap();

        if let Some(stream_result) = incoming.next() {
            println!("Incoming");

            let stream = stream_result?;

            println!("Accepting incoming as tls");

            let accept_result = test_server.accept(stream);

            if let Err(e) = map_tls_io_error(accept_result) {
                eprintln!("Accept failed: {:?}", e);
            }

            println!("Done with stream");
        } else {
            panic!("Never received an incoming connection");
        }

        println!("No longer accepting connections");

        Ok(())
    }

    async fn run_client(ca_certificate: Option<Certificate>, client_host: String) {
        println!("Client creating");

        let mut config = Client::configure().registry(&client_host);

        if let Some(ca) = &ca_certificate {
            config = config.add_root_certificate(ca.clone());
        }

        let registry = config.build().unwrap();

        let err = registry.is_auth().await.unwrap_err();

        if let dkregistry::errors::Error::Reqwest(r) = err {
            if let Some(s) = r.source() {
                let oh: Option<&hyper::Error> = s.downcast_ref();

                if let Some(he) = oh {
                    println!("Hyper error: {:?}", he);

                    if ca_certificate.is_some() {
                        assert!(
                            he.is_closed(),
                            "is a ChannelClosed error, not a certificate error"
                        );
                    } else {
                        assert!(
                            he.is_connect(),
                            "is a Connect error, with a certificate failure as a cause"
                        );

                        let hec = he.source().unwrap();

                        let message = format!("{}", hec);
                        assert!(
                            message.contains("certificate verify failed"),
                            "'certificate verify failed' contained in: {}",
                            message
                        );
                    }
                    return;
                }
            }
        } else {
            eprintln!("Unexpected error: {:?}", err);
        }
    }

    fn map_tls_io_error<S>(
        tls_result: Result<TlsStream<S>, HandshakeError<S>>,
    ) -> Result<TlsStream<S>, String>
    where
        S: std::io::Read + std::io::Write,
    {
        match tls_result {
            Ok(stream) => Ok(stream),
            Err(he) => {
                match he {
                    HandshakeError::Failure(e) => Err(format!("{}", e)),
                    // Can't directly unwrap because TlsStream doesn't implement Debug trait
                    HandshakeError::WouldBlock(_) => Err("Would block".into()),
                }
            }
        }
    }

    fn output() -> PathBuf {
        PathBuf::from(file!())
            .canonicalize()
            .unwrap()
            .parent()
            .unwrap()
            .parent()
            .unwrap()
            .parent()
            .unwrap()
            .join("certificate")
            .join("output")
    }

    fn read_output_file<F: AsRef<Path>>(file_name: F) -> Vec<u8> {
        std::fs::read(output().join(file_name)).unwrap()
    }

    #[tokio::test]
    async fn without_ca() {
        with_ca_cert(None).await
    }

    #[tokio::test]
    pub async fn with_ca() {
        let ca_bytes = read_output_file("ca.pem");
        let ca = Certificate::from_pem(&ca_bytes).unwrap();

        with_ca_cert(Some(ca)).await;
    }

    async fn with_ca_cert(ca_certificate: Option<Certificate>) {
        let registry_bytes = read_output_file("localhost.crt");

        let registry_key_bytes = read_output_file("localhost-key-pkcs8.pem");
        let registry_identity = Identity::from_pkcs8(&registry_bytes, &registry_key_bytes).unwrap();

        let listener = TcpListener::bind("localhost:0").unwrap();

        // local_addr returns an IP address, but we need to use a name for TLS,
        // so extract only the port number.
        let listener_port = listener.local_addr().unwrap().port();

        let client_host = format!("localhost:{}", listener_port);

        let t_server = std::thread::spawn(move || run_server(listener, registry_identity));

        let t_client =
            tokio::task::spawn(async move { run_client(ca_certificate, client_host).await });

        println!("Joining client");
        t_client.await.unwrap();

        println!("Joining server");
        t_server.join().unwrap().unwrap();

        println!("Done");
    }
}
