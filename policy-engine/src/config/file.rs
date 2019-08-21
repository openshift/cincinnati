//! TOML file configuration options.

use super::options;
use super::AppSettings;
use commons::de::de_loglevel;
use commons::MergeOptions;
use failure::{Fallible, ResultExt};
use std::io::Read;
use std::{fs, io, path};

/// TOML configuration, top-level.
#[derive(Debug, Deserialize)]
pub struct FileOptions {
    /// Verbosity level.
    #[serde(default = "Option::default", deserialize_with = "de_loglevel")]
    pub verbosity: Option<log::LevelFilter>,

    /// Upstream options.
    pub upstream: Option<UpstreamOptions>,

    /// Policy plugins options.
    pub policy: Option<Vec<toml::Value>>,

    /// Web frontend options.
    pub service: Option<options::ServiceOptions>,

    /// Status service options.
    pub status: Option<options::StatusOptions>,
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
            "failed to read config file {}",
            cfg_path.as_ref().display()
        ))?;

        Ok(cfg)
    }
}

impl MergeOptions<Option<FileOptions>> for AppSettings {
    fn try_merge(&mut self, opts: Option<FileOptions>) -> failure::Fallible<()> {
        if let Some(file) = opts {
            assign_if_some!(self.verbosity, file.verbosity);
            self.try_merge(file.policy)?;
            self.try_merge(file.service)?;
            self.try_merge(file.status)?;
            self.try_merge(file.upstream)?;
        }
        Ok(())
    }
}

impl MergeOptions<Option<Vec<toml::Value>>> for AppSettings {
    fn try_merge(&mut self, opts: Option<Vec<toml::Value>>) -> Fallible<()> {
        if let Some(policies) = opts {
            for conf in policies {
                let plugin = cincinnati::plugins::deserialize_config(conf)?;
                self.policies.push(plugin);
            }
        }
        Ok(())
    }
}

/// Options for upstream fetcher.
#[derive(Debug, Deserialize)]
pub struct UpstreamOptions {
    /// Fetcher method.
    pub method: Option<String>,

    /// Cincinnati upstream options.
    pub cincinnati: Option<options::UpCincinnatiOptions>,
}

impl MergeOptions<Option<UpstreamOptions>> for AppSettings {
    fn try_merge(&mut self, opts: Option<UpstreamOptions>) -> Fallible<()> {
        if let Some(upstream) = opts {
            self.try_merge(upstream.cincinnati)?;
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
    fn toml_basic() {
        let url = hyper::Uri::from_static("https://example.com/foo");
        let toml_input = "[upstream.cincinnati]\nurl='https://example.com/foo'";
        let file_opts: FileOptions = toml::from_str(toml_input).unwrap();

        let pause = file_opts.upstream.unwrap().cincinnati.unwrap().url.unwrap();
        assert_eq!(pause, url);
    }

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

        let input_url = hyper::Uri::from_static("https://example.com");
        let opts = {
            use std::io::Write;

            let sample_config = r#"
                verbosity = 3

                [upstream]
                method = "cincinnati"

                [upstream.cincinnati]
                url = "https://example.com"

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

        let ups = opts.upstream.unwrap().cincinnati.unwrap();
        let ups_url = ups.url.unwrap();
        assert_eq!(ups_url, input_url);
    }

    #[test]
    fn toml_basic_policy() {
        use cincinnati::plugins::internal::channel_filter::ChannelFilterPlugin;
        use cincinnati::plugins::prelude::*;

        let expected: Vec<BoxedPlugin> = new_plugins!(InternalPluginWrapper(ChannelFilterPlugin {
            key_prefix: String::from("io.openshift.upgrades.graph"),
            key_suffix: String::from("release.channels"),
        }));
        let mut settings = AppSettings::default();

        let opts = {
            use std::io::Write;

            let sample_config = r#"
                [[policy]]
                name = "channel-filter"
                key_prefix = "io.openshift.upgrades.graph"
                key_suffix = "release.channels"
            "#;

            let mut config_file = tempfile::NamedTempFile::new().unwrap();
            config_file
                .write_fmt(format_args!("{}", sample_config))
                .unwrap();
            crate::config::FileOptions::read_filepath(config_file.path()).unwrap()
        };
        assert!(opts.policy.is_some());
        settings.try_merge(Some(opts)).unwrap();
        assert_eq!(settings.policies.len(), 1);

        let policies = settings.policy_plugins().unwrap();
        assert_eq!(policies, expected);
    }
}
