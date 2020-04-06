# Graph-builder configuration

Graph-builder can be configured via TOML files and command-line options, with the latter having higher priority.

## TOML options

TOML configuration currently supports the following sections and options:

 - `verbosity` (unsigned integer): log verbosity level, from 0 (errors and warnings only) to 3 (all trace messages). Default: 0.
 - `service` (section): configuration options related to the main HTTP Cincinnati service.
   - `address` (string): local IP for the main service. Default: "127.0.0.1".
   - `mandatory_client_parameters` (list of strings): Cincinnati query parameters that must be present in client requests. Default: empty.
   - `path_prefix` (string): namespace prefix for all API endpoints. Default: "".
   - `port` (unsigned integer): local port for the main service. Default: 8080.
 - `status` (section): configuration options related to the HTTP status service.
   - `address` (string): local IP for the status service. Default: "127.0.0.1".
   - `port` (unsigned integer): local port for the status service. Default: 9080.
 - `upstream` (section): configuration options related to upstream release-data provider.
   - `method` (string): upstream provider selector. Allowed values: "registry". Default: "registry".
   - `registry` (section): configuration for Docker-v2 registry provider.
     - `credentials_path` (string): path to file containing registry credentials, in "dockercfg" format. Default: unset.
     - `manifestref_key` (string): metadata key where to record the manifest-reference. Default: "io.openshift.upgrades.graph.release.manifestref".
     - `pause_secs` (unsigned integer): pause between repository scrapes, in seconds. Default: 300.
     - `repository` (string): target image in the registry. Default: "openshift".
     - `url` (string): URL for the registry. Default: "http://localhost:5000". 
