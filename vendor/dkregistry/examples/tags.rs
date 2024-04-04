extern crate futures;
extern crate tokio;

mod common;

use futures::stream::StreamExt;
use std::result::Result;
use std::{boxed, error};

#[tokio::main]
async fn main() {
    let registry = match std::env::args().nth(1) {
        Some(x) => x,
        None => "registry-1.docker.io".into(),
    };

    let image = match std::env::args().nth(2) {
        Some(x) => x,
        None => "library/debian".into(),
    };
    println!("[{}] requesting tags for image {}", registry, image);

    let user = std::env::var("DKREG_USER").ok();
    if user.is_none() {
        println!("[{}] no $DKREG_USER for login user", registry);
    }
    let password = std::env::var("DKREG_PASSWD").ok();
    if password.is_none() {
        println!("[{}] no $DKREG_PASSWD for login password", registry);
    }

    let res = run(&registry, user, password, &image).await;

    if let Err(e) = res {
        println!("[{}] {}", registry, e);
        std::process::exit(1);
    };
}

async fn run(
    host: &str,
    user: Option<String>,
    passwd: Option<String>,
    image: &str,
) -> Result<(), boxed::Box<dyn error::Error>> {
    env_logger::Builder::new()
        .filter(Some("dkregistry"), log::LevelFilter::Trace)
        .filter(Some("trace"), log::LevelFilter::Trace)
        .try_init()?;

    let client = dkregistry::v2::Client::configure()
        .registry(host)
        .insecure_registry(false)
        .username(user)
        .password(passwd)
        .build()?;

    let login_scope = format!("repository:{}:pull", image);

    let dclient = client.authenticate(&[&login_scope]).await?;

    dclient
        .get_tags(&image, Some(7))
        .collect::<Vec<_>>()
        .await
        .into_iter()
        .map(Result::unwrap)
        .for_each(|tag| {
            println!("{:?}", tag);
        });

    Ok(())
}
