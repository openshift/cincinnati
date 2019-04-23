//! Command-line options.

use super::options;
use super::AppSettings;
use commons::MergeOptions;

/// CLI configuration flags, top-level.
#[derive(Debug, StructOpt)]
pub struct CliOptions {
    /// Verbosity level
    #[structopt(short = "v", parse(from_occurrences))]
    pub verbosity: u8,

    /// Path to configuration file
    #[structopt(short = "c")]
    pub config_path: Option<String>,

    // TODO(lucab): drop this when plugins are configurable.
    #[structopt(long = "disable_quay_api_metadata")]
    pub disable_quay_api_metadata: Option<bool>,

    #[structopt(flatten)]
    pub service: options::ServiceOptions,

    #[structopt(flatten)]
    pub status: options::StatusOptions,

    /// Fetcher method.
    #[structopt(long = "upstream.method")]
    pub upstream_method: Option<String>,

    #[structopt(flatten)]
    pub upstream_registry: options::DockerRegistryOptions,
}

impl MergeOptions<CliOptions> for AppSettings {
    fn merge(&mut self, opts: CliOptions) -> () {
        self.verbosity = match opts.verbosity {
            0 => self.verbosity,
            1 => log::LevelFilter::Info,
            2 => log::LevelFilter::Debug,
            _ => log::LevelFilter::Trace,
        };
        self.merge(Some(opts.service));
        self.merge(Some(opts.status));
        self.merge(Some(opts.upstream_registry));

        // TODO(lucab): drop this when plugins are configurable.
        assign_if_some!(
            self.disable_quay_api_metadata,
            opts.disable_quay_api_metadata
        );
    }
}

#[cfg(test)]
mod tests {
    use super::CliOptions;
    use crate::config::AppSettings;
    use commons::MergeOptions;
    use structopt::StructOpt;

    #[test]
    fn cli_basic() {
        let no_args = vec!["argv0"];
        let no_args_cli = CliOptions::from_iter_safe(no_args).unwrap();
        assert_eq!(no_args_cli.verbosity, 0);
        assert_eq!(no_args_cli.upstream_method, None);

        let verbose_args = vec!["argv0", "-vvv"];
        let verbose_cli = CliOptions::from_iter_safe(verbose_args).unwrap();
        assert_eq!(verbose_cli.verbosity, 3);

        let svc_port_args = vec!["argv0", "--service.port", "9999"];
        let svc_port_cli = CliOptions::from_iter_safe(svc_port_args).unwrap();
        assert_eq!(svc_port_cli.service.port, Some(9999));
    }

    #[test]
    fn cli_merge_settings() {
        let repo = "cincinnati/cli-test";

        let mut settings = AppSettings::default();
        assert_eq!(settings.repository, "openshift".to_string());

        let args = vec!["argv0", "--upstream.registry.repository", repo];
        let cli = CliOptions::from_iter_safe(args).unwrap();
        assert_eq!(cli.upstream_registry.repository, Some(repo.to_string()));

        settings.merge(cli);
        assert_eq!(settings.repository, repo.to_string());
    }

    #[test]
    fn cli_override_toml() {
        use crate::config::file::FileOptions;

        let mut settings = AppSettings::default();
        assert_eq!(settings.verbosity, log::LevelFilter::Warn);

        let toml_verbosity = "verbosity=3";
        let file_opts: FileOptions = toml::from_str(toml_verbosity).unwrap();
        assert_eq!(file_opts.verbosity, Some(log::LevelFilter::Trace));

        settings.merge(Some(file_opts));
        assert_eq!(settings.verbosity, log::LevelFilter::Trace);

        let args = vec!["argv0", "-vv"];
        let cli_opts = CliOptions::from_iter_safe(args).unwrap();
        assert_eq!(cli_opts.verbosity, 2);

        settings.merge(cli_opts);
        assert_eq!(settings.verbosity, log::LevelFilter::Debug);
    }
}
