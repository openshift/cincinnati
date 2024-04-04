extern crate dirs;
extern crate futures;
extern crate serde_json;
extern crate tokio;

use dkregistry::reference;
use dkregistry::v2::manifest::Manifest;
use std::result::Result;
use std::str::FromStr;
use std::{env, fs, io};

mod common;

#[tokio::main]
async fn main() {
    let dkr_ref = match std::env::args().nth(1) {
        Some(ref x) => reference::Reference::from_str(x),
        None => reference::Reference::from_str("quay.io/steveej/cincinnati-test-labels:0.0.0"),
    }
    .unwrap();
    let registry = dkr_ref.registry();

    println!("[{}] downloading image {}", registry, dkr_ref);

    let mut user = None;
    let mut password = None;
    let home = dirs::home_dir().unwrap();
    let cfg = fs::File::open(home.join(".docker/config.json"));
    if let Ok(fp) = cfg {
        let creds = dkregistry::get_credentials(io::BufReader::new(fp), &registry);
        if let Ok(user_pass) = creds {
            user = user_pass.0;
            password = user_pass.1;
        } else {
            println!("[{}] no credentials found in config.json", registry);
        }
    } else {
        user = env::var("DKREG_USER").ok();
        if user.is_none() {
            println!("[{}] no $DKREG_USER for login user", registry);
        }
        password = env::var("DKREG_PASSWD").ok();
        if password.is_none() {
            println!("[{}] no $DKREG_PASSWD for login password", registry);
        }
    };

    let res = run(&dkr_ref, user, password).await;

    if let Err(e) = res {
        println!("[{}] {}", registry, e);
        std::process::exit(1);
    };
}

async fn run(
    dkr_ref: &reference::Reference,
    user: Option<String>,
    passwd: Option<String>,
) -> Result<(), dkregistry::errors::Error> {
    let client = dkregistry::v2::Client::configure()
        .registry(&dkr_ref.registry())
        .insecure_registry(false)
        .username(user)
        .password(passwd)
        .build()?;

    let image = dkr_ref.repository();
    let login_scope = format!("repository:{}:pull", image);
    let version = dkr_ref.version();

    let dclient = client.authenticate(&[&login_scope]).await?;
    let manifest = dclient.get_manifest(&image, &version).await?;

    if let Manifest::S1Signed(s1s) = manifest {
        let labels = s1s.get_labels(0);
        println!("got labels: {:#?}", labels);
    } else {
        println!("got no labels");
    }

    Ok(())
}
