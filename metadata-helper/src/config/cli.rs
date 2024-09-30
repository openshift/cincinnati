//! Command-line options for metadata-helper.

use super::options;
use super::AppSettings;
use commons::prelude_errors::*;
use commons::MergeOptions;

/// CLI configuration flags, top-level.
#[derive(Debug, StructOpt)]
pub struct CliOptions {
    /// Verbosity level
    #[structopt(short = "v", parse(from_occurrences))]
    pub verbosity: u64,

    /// Path to configuration file
    #[structopt(short = "c")]
    pub config_path: Option<String>,

    // Main service options
    #[structopt(flatten)]
    pub service: options::ServiceOptions,

    // Signature service options
    #[structopt(flatten)]
    pub signatures: options::SignaturesOptions,

    // Status service options
    #[structopt(flatten)]
    pub status: options::StatusOptions,
}

impl MergeOptions<CliOptions> for AppSettings {
    fn try_merge(&mut self, opts: CliOptions) -> Fallible<()> {
        self.verbosity = match opts.verbosity {
            0 => self.verbosity,
            1 => log::LevelFilter::Info,
            2 => log::LevelFilter::Debug,
            _ => log::LevelFilter::Trace,
        };

        self.try_merge(Some(opts.service))?;
        self.try_merge(Some(opts.signatures))?;
        self.try_merge(Some(opts.status))?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::CliOptions;
    use crate::config::AppSettings;
    use structopt::StructOpt;

    #[test]
    fn cli_basic() {
        let no_args = vec!["argv0"];
        let no_args_cli = CliOptions::from_iter_safe(no_args).unwrap();
        assert_eq!(no_args_cli.verbosity, 0);

        let verbose_args = vec!["argv0", "-vvv"];
        let verbose_cli = CliOptions::from_iter_safe(verbose_args).unwrap();
        assert_eq!(verbose_cli.verbosity, 3);

        let svc_port_args = vec!["argv0", "--service.port", "9999"];
        let svc_port_cli = CliOptions::from_iter_safe(svc_port_args).unwrap();
        assert_eq!(svc_port_cli.service.port, Some(9999));

        let sig_dir_args = vec!["argv0", "--signatures.dir", "/a/b"];
        let sig_dir_cli = CliOptions::from_iter_safe(sig_dir_args).unwrap();
        assert_eq!(sig_dir_cli.signatures.dir, Some("/a/b".to_string()));
    }

    #[test]
    fn cli_override_toml() {
        use crate::config::file::FileOptions;
        use commons::MergeOptions;

        let mut settings = AppSettings::default();
        assert_eq!(settings.verbosity, log::LevelFilter::Warn);

        let toml_verbosity = r#"verbosity="vvv""#;
        let file_opts: FileOptions = toml::from_str(toml_verbosity).unwrap();
        assert_eq!(file_opts.verbosity, Some(log::LevelFilter::Trace));

        settings.try_merge(Some(file_opts)).unwrap();
        assert_eq!(settings.verbosity, log::LevelFilter::Trace);

        let args = vec!["argv0", "-vv"];
        let cli_opts = CliOptions::from_iter_safe(args).unwrap();
        assert_eq!(cli_opts.verbosity, 2);

        settings.try_merge(cli_opts).unwrap();
        assert_eq!(settings.verbosity, log::LevelFilter::Debug);
    }
}
