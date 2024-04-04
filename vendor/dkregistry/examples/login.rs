extern crate futures;
extern crate tokio;

mod common;

use std::result::Result;
use std::{boxed, error};

#[tokio::main]
async fn main() {
    let registry = match std::env::args().nth(1) {
        Some(x) => x,
        None => "registry-1.docker.io".into(),
    };

    let login_scope = match std::env::args().nth(2) {
        Some(x) => x,
        None => "".into(),
    };

    let user = std::env::var("DKREG_USER").ok();
    if user.is_none() {
        println!("[{}] no $DKREG_USER for login user", registry);
    }
    let password = std::env::var("DKREG_PASSWD").ok();
    if password.is_none() {
        println!("[{}] no $DKREG_PASSWD for login password", registry);
    }

    let res = run(&registry, user, password, login_scope).await;

    if let Err(e) = res {
        println!("[{}] {}", registry, e);
        std::process::exit(1);
    };
}

async fn run(
    host: &str,
    user: Option<String>,
    passwd: Option<String>,
    login_scope: String,
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

    let dclient = client.authenticate(&[&login_scope]).await?;
    dclient.is_auth().await?;
    Ok(())
}
