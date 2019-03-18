//! Command-line options.

// TODO(lucab): drop all aliases after migration.

/// CLI configuration flags, top-level.
#[derive(Debug, StructOpt)]
pub struct CliOptions {
    /// Verbosity level
    #[structopt(long = "verbosity", short = "v", parse(from_occurrences))]
    pub verbosity: u8,

    /// Path to configuration file
    #[structopt(short = "c")]
    pub config_path: Option<String>,

    #[structopt(flatten)]
    pub service: ServiceOptions,

    #[structopt(flatten)]
    pub status: StatusOptions,

    #[structopt(flatten)]
    pub registry: UpstreamRegistryOptions,
}

/// CLI configuration flags, status service.
#[derive(Debug, StructOpt)]
pub struct StatusOptions {
    /// Address on which the status service will listen
    #[structopt(long = "status.address")]
    pub address: Option<String>,

    /// Port to which the status service will bind
    #[structopt(long = "status.port")]
    pub port: Option<u16>,
}

/// CLI configuration flags, main Cincinnati service.
#[derive(Debug, StructOpt)]
pub struct ServiceOptions {
    /// Address on which the server will listen
    #[structopt(long = "service.address", alias = "address")]
    pub address: Option<String>,

    /// Port to which the server will bind
    #[structopt(long = "service.port", alias = "port")]
    pub port: Option<u16>,

    /// Namespace prefix for all service endpoints
    #[structopt(long = "service.path_prefix")]
    pub path_prefix: Option<String>,

    /// Comma-separated set of mandatory client parameters
    #[structopt(long = "service.mandatory_client_parameters")]
    pub mandatory_client_parameters: Option<String>,
}

/// CLI configuration flags, Docker-registry fetcher.
#[derive(Debug, StructOpt)]
pub struct UpstreamRegistryOptions {
    /// Duration of the pause (in seconds) between registry scans
    #[structopt(long = "upstream.registry.period")]
    pub period: Option<u64>,

    /// URL for the container image registry
    #[structopt(long = "upstream.registry.url", alias = "registry")]
    pub url: Option<String>,

    /// Name of the container image repository
    #[structopt(long = "upstream.registry.repository", alias = "repository")]
    pub repository: Option<String>,

    /// Credentials file for authentication against the image registry
    #[structopt(
        long = "upstream.registry.credentials_path",
        alias = "credentials-file"
    )]
    pub credentials_path: Option<String>,

    /// Metadata key where to record the manifest-reference
    #[structopt(long = "upstream.registry.manifestref_key")]
    pub manifestref_key: Option<String>,
}
