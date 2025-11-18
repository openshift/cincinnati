# AGENTS.md

## Project Overview

Cincinnati is an update protocol for automatic updates, primarily used by OpenShift. It represents transitions between releases as a directed acyclic graph (DAG) and allows clients to perform automatic updates between releases.

## Build System

- **Language**: Rust (Rust 2018 edition)
- **Command Runner**: Use `just` for common development tasks

### Common Development Commands

```bash
# Run all tests with formatting checks
just test

# Start services locally
just run-graph-builder    # Start graph-builder service (port 8080)
just run-policy-engine    # Start policy-engine service (port 8081)
just run-metadata-helper # Start metadata-helper service

# Run CI test suite and e2e tests
just run-ci-tests
just run-e2e-test-only   # Run only end-to-end tests

cargo build --release    # Release build
```

## Architecture Overview

Cincinnati follows a microservices architecture with two main services:

### Core Services

1. **Graph Builder** (`graph-builder/`) - Scrapes releases from container registries and builds update graphs
   - Serves `/graph` endpoint on port 8080
   - Integrates with Docker registries, GitHub, and OpenShift metadata
   - Plugin-based architecture for extensible release processing

2. **Policy Engine** (`policy-engine/`) - Applies policies to update graphs and serves filtered graphs
   - Serves `/graph` endpoint on port 8081 with OpenAPI specification
   - Filters graphs based on client parameters (channel, architecture)
   - Enforces rollout policies and business logic

### Key Libraries

- **cincinnati/** - Core library with graph types and plugin infrastructure
- **commons/** - Shared utilities (web framework, metrics, logging, HTTP client)
- **quay/** - Container registry client for Docker v2 API
- **metadata-helper/** - Metadata processing service for handling signatures and additional data

### Plugin System

- Plugin interface defined in `cincinnati/src/plugins/interface.proto`
- External web-based plugins for custom logic
- Internal built-in plugins for common operations (filtering, metadata parsing, graph manipulation)

### Policy Engine Plugins

The policy engine uses a plugin-based architecture to process and filter update graphs. All plugins are internal Rust-based implementations that run in sequence to transform the graph. Here's a detailed breakdown of each plugin:

#### Core Filtering Plugins

**1. arch-filter** (`cincinnati/src/plugins/internal/arch_filter.rs`)
- **Purpose**: Filters graph releases by architecture (e.g., `amd64`, `arm64`)
- **How it works**: Reads `arch` parameter from client requests and removes releases that don't match the requested architecture
- **Metadata processing**: Uses `io.openshift.upgrades.graph.release.arch` metadata key to identify compatible architectures
- **Additional features**: Strips architecture suffixes from version strings (e.g., `4.1.0+amd64` → `4.1.0`)
- **Configuration**: Default key prefix `io.openshift.upgrades.graph`, key suffix `release.arch`
- **Use case**: Ensures clients only see releases compatible with their architecture

**2. channel-filter** (`cincinnati/src/plugins/internal/channel_filter.rs`)
- **Purpose**: Filters graph releases by release channel (e.g., `stable-4.2`, `fast-4.3`)
- **How it works**: Reads `channel` parameter from client requests and removes releases not assigned to that channel
- **Metadata processing**: Uses `io.openshift.upgrades.graph.release.channels` metadata key with comma-separated channel values
- **Validation**: Channel names must match regex `^[0-9a-z\-\.]+$`
- **Use case**: Allows different risk tolerance levels (stable vs fast channels) and version streams

#### Graph Manipulation Plugins

**3. edge-add-remove** (`cincinnati/src/plugins/internal/edge_add_remove.rs`)
- **Purpose**: Dynamically adds and removes edges based on release metadata
- **How it works**: Processes metadata labels with syntax `<prefix>.(previous|next).(add|remove)=(version1,version2,...)`
- **Operations**:
  - `*.previous.add`: Add edges from current release to specified previous releases
  - `*.next.add`: Add edges from current release to specified next releases
  - `*.previous.remove`: Remove edges from current release to specified previous releases
  - `*.next.remove`: Remove edges from current release to specified next releases
- **Advanced features**:
  - `*.previous.remove_regex`: Remove edges using regex patterns
  - Conditional edges support for complex update logic with risks and matching rules
- **Processing order**: Add operations first, then remove operations (removes take precedence)
- **Use case**: Implements custom update paths, blocks problematic upgrades, creates conditional updates

**4. node-remove** (`cincinnati/src/plugins/internal/node_remove.rs`)
- **Purpose**: Removes entire releases from the graph based on metadata flags
- **How it works**: Removes releases where `io.openshift.upgrades.graph.release.remove` metadata equals `"true"`
- **Use case**: Allows marking specific releases as unavailable or deprecated

#### Data Fetching Plugins

**5. cincinnati-graph-fetch** (`cincinnati/src/plugins/internal/cincinnati_graph_fetch.rs`)
- **Purpose**: Fetches complete update graphs from upstream Cincinnati endpoints 
- **How it works**: Makes HTTP GET requests to `/graph` endpoints (typically graph-builder on port 8080)
- **Features**:
  - 60-second response caching for performance
  - Gzip compression support
  - Configurable timeouts (default 30 seconds)
  - Prometheus metrics for request counting and error tracking
- **Use case**: Policy engine's primary data source, replaces input graph with fetched data

**6. quay-metadata** (`cincinnati/src/plugins/internal/metadata_fetch_quay.rs`)
- **Purpose**: Fetches additional metadata from quay.io container registry labels
- **How it works**: Uses manifest references to query quay.io API for label-based metadata
- **Requirements**: Releases must have `io.openshift.upgrades.graph.release.manifestref` metadata
- **Features**: All-or-nothing operation - fails if any release lacks required metadata
- **Use case**: Enriches release metadata with quay.io labels for advanced filtering

#### Graph Builder Plugins

**7. release-scrape-dockerv2** (`cincinnati/src/plugins/internal/graph_builder/release_scrape_dockerv2/`)
- **Purpose**: Scrapes container registries for release information and builds initial graphs
- **How it works**: Queries Docker v2 registries (like quay.io) for tags and metadata
- **Features**:
  - Concurrent fetching (default 16 concurrent requests)
  - Authentication support (username/password or credential files)
  - Custom SSL certificates support
  - Caching for performance
- **Output**: Creates graph nodes with manifest references for further metadata enrichment
- **Use case**: Primary graph construction from container registries

**8. dkrv2-openshift-secondary-metadata-scraper** (`cincinnati/src/plugins/internal/graph_builder/dkrv2_openshift_secondary_metadata_scraper/`)
- **Purpose**: Downloads and extracts OpenShift graph data from container images
- **Data Source**: Container registries (Docker v2 protocol)
- **How it works**: Pulls container images containing Cincinnati graph data, verifies signatures (optional), and extracts metadata files
- **Features**:
  - Downloads container images from registries (quay.io, etc.)
  - Layer caching to avoid re-downloading unchanged data
  - GPG signature verification for security
  - Output filtering with allowlist patterns
  - Automatic tarball creation for downstream processing
  - Symlink creation for signature files
- **Security**: Supports cryptographic verification using GPG keys and signature checking
- **Configuration**:
  - Registry/repository/tag specification
  - Authentication credentials
  - Signature verification settings
  - Output directory and filtering rules
- **Output files**: Extracts channels YAML, blocked-edges YAML, raw metadata.json, version files, and LICENSE
- **Use case**: Downloads official OpenShift release metadata from container images for graph construction

**9. github-openshift-secondary-metadata-scraper** (`cincinnati/src/plugins/internal/graph_builder/github_openshift_secondary_metadata_scraper/`)
- **Purpose**: Downloads and extracts OpenShift graph data from GitHub repositories
- **Data Source**: GitHub repositories (Git-based)
- **How it works**: Downloads tarballs from GitHub releases or branches containing Cincinnati graph data and extracts metadata files
- **Features**:
  - Supports both branch-based and commit-based references
  - GitHub API authentication via OAuth tokens
  - Change detection to avoid unnecessary downloads
  - Output filtering with allowlist patterns
  - Automatic tarball creation for downstream processing
  - Symlink creation for signature files
- **Configuration**:
  - GitHub organization/repository specification
  - Branch or commit reference selection
  - OAuth token authentication
  - Output directory and filtering rules
- **Output files**: Extracts channels YAML, blocked-edges YAML, raw metadata.json, version files, and LICENSE
- **Use case**: Alternative metadata source for OpenShift releases from GitHub repositories

**Note**: Both `dkrv2-openshift-secondary-metadata-scraper` and `github-openshift-secondary-metadata-scraper` perform identical functions - they download and extract OpenShift secondary metadata files. The only difference is the data source:
- **dkrv2**: Uses container registries (Docker images) - more secure and official for production
- **github**: Uses GitHub repositories - useful for development, testing, or alternative deployment scenarios

Both plugins produce the same output format and are processed by the `openshift-secondary-metadata-parser` plugin that follows them in the pipeline.

**10. openshift-secondary-metadata-parser** (`cincinnati/src/plugins/internal/graph_builder/openshift_secondary_metadata_parser/`)
- **Purpose**: Parses OpenShift-specific metadata from graph-builder data
- **How it works**: Processes Cincinnati graph data format with channels, blocked edges, and raw metadata
- **Features**:
  - Channel-based release organization
  - Blocked edge processing (prevents problematic updates)
  - Signature verification support
  - Metadata validation and transformation
- **Use case**: Processes OpenShift release metadata for channel and blocking logic

#### Utility Plugins

**11. versioned-graph** (`cincinnati/src/plugins/internal/versioned_graph.rs`)
- **Purpose**: Adds version information to graph responses based on content-type negotiation
- **How it works**: Wraps graph data with version numbers for API compatibility
- **Use case**: Ensures backward compatibility across different Cincinnati API versions

## Development Workflow

1. **Local Development**: Run `just run-graph-builder` and `just run-policy-engine` in separate terminals
2. **Testing**: Use `just test` for comprehensive testing, including formatting checks
3. **API Testing**: Use `just get-graph-pe` to test policy-engine responses
4. **Graph Visualization**: Use `just display-graph` to visualize update paths

## Testing Architecture

- **Integration Tests**: End-to-end tests in `e2e/` crate for service-to-service testing
- **CI Tests**: Use `dist/cargo_test.sh` for comprehensive CI-style testing with JUnit output

## API Format

Services return Cincinnati JSON format:
```json
{
  "nodes": [{"version": "4.2.0", "payload": "sha256:...", "metadata": {...}}],
  "edges": [[0, 1]],
  "conditionalEdges": []
}
```

Policy-engine requires `channel` parameter and supports `arch` for filtering.

## Configuration

- TOML-based configuration files with environment variable support
- Plugin configuration through arrays in service config
- Service discovery via status endpoints on configured ports

## Deployment

### Docker Commands
```bash
# Build deployment image
docker build -f dist/Dockerfile.deploy/Dockerfile -t quay.io/user/cincinnati:tag .

# Build rust-toolset image (for development)
docker build -f dist/Dockerfile.rust-toolset/Dockerfile -t quay.io/user/cincinnati:rt-tag .

# Push to registry
docker push quay.io/user/cincinnati:tag
```

## Workspace Structure

This is a Cargo workspace with 11 crates. Key directories:
- `cincinnati/`, `commons/`, `graph-builder/`, `policy-engine/` - Core services
- `quay/`, `metadata-helper/`, `prometheus-query/` - Specialized utilities
- `e2e/` - Integration tests
- `docs/` - Developer and design documentation
- `dist/` - CI/CD and deployment artifacts (detailed below)
- `hack/` - Build and development scripts

### dist/ Directory Structure

The `dist/` folder contains all CI/CD, deployment, and operational artifacts:

#### Build and Deployment Scripts
- `build.sh` - Legacy build script (replaced by Prow CI)
- `build_deploy.sh` - Builds and optionally pushes deployment Docker images
- `cargo_test.sh` - Comprehensive test runner with feature flags for different crates
- `commons.sh` - Shared variables and functions for build scripts

#### Docker Images
- `Dockerfile.deploy/` - Production deployment image based on UBI9
- `Dockerfile.e2e/` - End-to-end testing image
- `Dockerfile.e2e-ubi/` - UBI-based E2E testing image
- `Dockerfile.rust-toolset/` - Development environment with Rust toolchain

#### CI/CD Integration
- `pr_check.sh` - Runs all upstream PR validation checks
- `prow_rustfmt.sh` - Code formatting checks for Prow CI
- `prow_yaml_lint.sh` - YAML validation for Prow CI
- `prepare_ci_credentials.sh` - Sets up CI credentials
- `prepare_ci_images.sh` - Prepares CI build images
- `prepare_e2e_images.sh` - Prepares end-to-end test images

#### OpenShift Deployment
- `openshift/cincinnati-deployment.yaml` - Main Cincinnati deployment template
- `openshift/cinci-with-mh-deployment.yaml` - Deployment including metadata-helper
- `openshift/cincinnati-e2e.yaml` - End-to-end testing deployment
- `openshift/load-testing.yaml` - Load testing configuration
- `openshift/observability.yaml` - Monitoring and observability setup

#### Monitoring and Observability
- `grafana/cincinnati-sli.json` - Service Level Indicator dashboards
- `grafana/dashboards/` - Generated ConfigMaps for Grafana dashboards
- `run_containerized_loadtesting.sh` - Load testing execution script

#### Dependency Management
- `vendor.sh` - Vendors dependencies and generates RH manifest for offline builds
