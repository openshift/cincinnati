use std::io::Read;
use std::path::{Path, PathBuf};

use actix_web::Error;
use commons::Fallible;
use log::debug;
use reqwest::Certificate;

/// Retrives all the .pem and .crt certificate files from the directory.
/// Scans till depth of 2, any file/directory below the depth of 2 will be ignored.
pub fn get_certs_from_dir(dir: &Path) -> Fallible<Vec<Certificate>, Error> {
    let mut certs: Vec<Certificate> = Vec::new();
    let mut dirs: Vec<(PathBuf, u8)> = vec![(dir.to_path_buf(), 0)];
    while !dirs.is_empty() {
        let (dir, level) = dirs.pop().unwrap();
        if let Ok(cert_paths) = std::fs::read_dir(dir) {
            for path in cert_paths {
                if let Ok(path) = path {
                    if path.metadata().unwrap().is_file() {
                        match path.path().extension() {
                            Some(extension) => {
                                if extension.to_ascii_lowercase() == "pem"
                                    || extension.to_ascii_lowercase() == "crt"
                                {
                                    let mut cert_buf = Vec::new();
                                    std::fs::File::open(&path.path())?
                                        .read_to_end(&mut cert_buf)?;

                                    // rename "TRUSTED CERTIFICATE" to "CERTIFICATE" in case of non-standard trusted certificates
                                    let pem_str = String::from_utf8(cert_buf).unwrap();
                                    let pem_str =
                                        pem_str.replace("TRUSTED CERTIFICATE", "CERTIFICATE");

                                    let certificate =
                                        reqwest::Certificate::from_pem(&pem_str.as_bytes());
                                    if certificate.is_ok() {
                                        debug!(
                                            "Adding {} to certificates",
                                            path.path().to_str().unwrap()
                                        );
                                        certs.push(certificate.unwrap());
                                    } else {
                                        debug!(
                                            "unable to process certificate {}: {}",
                                            path.file_name().to_str().unwrap_or_default(),
                                            certificate.unwrap_err()
                                        );
                                    }
                                };
                            }
                            None => {}
                        };
                    } else if path.metadata().unwrap().is_dir() {
                        if level < 2 {
                            dirs.push((path.path(), level + 1));
                        };
                    };
                }
            }
        }
    }
    Ok(certs)
}

#[cfg(test)]
pub mod tests {
    //! Common functionality for graph-builder tests

    use crate as cincinnati;

    use cincinnati::plugins::internal::graph_builder::release_scrape_dockerv2::registry;

    fn init_logger() {
        let _ = env_logger::try_init_from_env(env_logger::Env::default());
    }

    pub fn common_init() -> (tokio::runtime::Runtime, registry::cache::Cache) {
        init_logger();
        (
            tokio::runtime::Runtime::new().unwrap(),
            registry::cache::new(),
        )
    }
}
