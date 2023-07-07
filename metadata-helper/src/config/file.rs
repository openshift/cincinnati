//! TOML file configuration options.

use super::options;
use super::AppSettings;
use commons::de::de_loglevel;
use commons::prelude_errors::*;
use commons::MergeOptions;
use std::io::Read;
use std::{fs, io, path};

/// TOML configuration, top-level.
#[derive(Debug, Deserialize)]
pub struct FileOptions {
    /// Verbosity level.
    #[serde(default = "Option::default", deserialize_with = "de_loglevel")]
    pub verbosity: Option<log::LevelFilter>,

    /// Web frontend options.
    pub service: Option<options::ServiceOptions>,

    /// Status service options.
    pub status: Option<options::StatusOptions>,

    /// Signatures service options
    pub signatures: Option<options::SignaturesOptions>,
}

impl FileOptions {
    /// Parse a TOML configuration from path.
    pub fn read_filepath<P>(cfg_path: P) -> Fallible<Self>
    where
        P: AsRef<path::Path>,
    {
        let cfg_file = fs::File::open(&cfg_path).context(format!(
            "failed to open config path {:?}",
            cfg_path.as_ref()
        ))?;
        let mut bufrd = io::BufReader::new(cfg_file);

        let mut content = vec![];
        bufrd.read_to_end(&mut content)?;
        let cfg = toml::from_slice(&content).context(format!(
            "failed to parse config file {}:\n{}",
            cfg_path.as_ref().display(),
            std::str::from_utf8(&content).unwrap_or("file not decodable")
        ))?;

        Ok(cfg)
    }
}

impl MergeOptions<Option<FileOptions>> for AppSettings {
    fn try_merge(&mut self, opts: Option<FileOptions>) -> Fallible<()> {
        if let Some(file) = opts {
            assign_if_some!(self.verbosity, file.verbosity);
            self.try_merge(file.service)?;
            self.try_merge(file.status)?;
            self.try_merge(file.signatures)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::FileOptions;
    use crate::config::AppSettings;
    use commons::MergeOptions;

    #[test]
    fn toml_merge_settings() {
        let mut settings = AppSettings::default();
        assert_eq!(settings.status_port, 9081);

        let toml_input = "status.port = 2222";
        let file_opts: FileOptions = toml::from_str(toml_input).unwrap();

        settings.try_merge(Some(file_opts)).unwrap();
        assert_eq!(settings.status_port, 2222);
    }

    #[test]
    fn toml_sample_config() {
        use super::FileOptions;

        let input_url = hyper::Uri::from_static("0.0.0.0");
        let opts = {
            use std::io::Write;

            let sample_config = r#"
                verbosity = "vvv"

                [service]
                address = "0.0.0.0"
                port = 8383

                [status]
                address = "127.0.0.1"
            "#;

            let mut config_file = tempfile::NamedTempFile::new().unwrap();
            config_file
                .write_fmt(format_args!("{}", sample_config))
                .unwrap();
            FileOptions::read_filepath(config_file.path()).unwrap()
        };

        assert_eq!(opts.verbosity, Some(log::LevelFilter::Trace));
        assert!(opts.service.is_some());

        let srv = opts.service.unwrap();
        let srv_url = srv.address.unwrap();
        assert_eq!(srv_url, input_url);
    }
}
