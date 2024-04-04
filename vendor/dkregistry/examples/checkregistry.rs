extern crate tokio;

use std::{boxed, error};

#[tokio::main]
async fn main() {
    let registry = match std::env::args().nth(1) {
        Some(x) => x,
        None => "registry-1.docker.io".into(),
    };

    let res = run(&registry).await;

    if let Err(e) = res {
        println!("[{}] {}", registry, e);
        std::process::exit(1);
    };
}

async fn run(host: &str) -> Result<bool, boxed::Box<dyn error::Error>> {
    let dclient = dkregistry::v2::Client::configure()
        .registry(host)
        .insecure_registry(false)
        .build()?;

    let supported = dclient.is_v2_supported().await?;
    if supported {
        println!("{} supports v2", host);
    } else {
        println!("{} does NOT support v2", host);
    }
    Ok(supported)
}
